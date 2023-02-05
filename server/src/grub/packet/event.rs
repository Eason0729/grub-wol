// TODO: fix bug-> if event didn't yield(timeout), signals(on the Registry) would have possible memory leak

use std::cell::Cell;
use std::fmt::Debug;
use std::hash::Hash;
use std::pin::Pin;
use std::sync::Mutex;
use std::task::{self, Poll};
use std::time;
use std::{collections::*, future::Future};

use async_std::future::timeout;
use async_std::task::{sleep, spawn};
use futures_lite::future::race;

use super::hashvec::*;

struct Registry<S, P>
where
    S: Hash + Eq + Debug,
{
    id_counter: usize,
    wakers: HashMap<usize, task::Waker>,
    signals: HashVec<S, usize>,
    payloads: HashMap<usize, P>,
}

impl<S, P> Default for Registry<S, P>
where
    S: Hash + Eq + Debug,
{
    fn default() -> Self {
        Self {
            id_counter: 1,
            wakers: Default::default(),
            payloads: Default::default(),
            signals: Default::default(),
        }
    }
}

pub struct EventHook<S, P>
where
    S: Hash + Eq + Debug,
{
    registry: Mutex<Registry<S, P>>,
}

impl<S, P> Default for EventHook<S, P>
where
    S: Hash + Eq + Debug,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<S, P> EventHook<S, P>
where
    S: Hash + Eq + Debug,
{
    pub fn new() -> Self {
        Self {
            registry: Mutex::default(),
        }
    }
    /// wait for the signal
    pub async fn wait(&self, signal: S) -> P {
        let hook = self.register(signal);

        hook.try_wait().await;

        hook.try_yield().unwrap()
    }
    /// wait for the signal while keep invoking the function
    pub async fn poll_until<F>(&self, signal: S, f: F, interval: time::Duration) -> P
    where
        F: Fn() + Send + 'static,
    {
        let hook = self.register(signal);
        let poll_handle = spawn(async move {
            loop {
                f();
                sleep(interval).await;
            }
        });

        hook.try_wait().await;
        poll_handle.cancel().await;

        hook.try_yield().unwrap()
    }

    /// Wait for the singal before timeout
    ///
    /// # Panics
    ///
    /// Panics if SystemTime plus timeout excess the limitation
    ///
    /// # Errors
    ///
    /// This function will return an error if timeout
    pub async fn timeout(&self, signal: S, timeout_: time::Duration) -> Result<P, ()> {
        let hook = self.register(signal);

        timeout(timeout_, hook.try_wait()).await.ok();
        match hook.try_yield() {
            Some(x) => Ok(x),
            None => Err(()),
        }
    }
    /// Polling for the singal before timeout
    ///
    /// # Panics
    ///
    /// Panics if SystemTime plus timeout excess the limitation
    ///
    /// # Errors
    ///
    /// This function will return an error if timeout
    pub async fn poll_timeout<F>(
        &self,
        signal: S,
        f: F,
        interval: time::Duration,
        timeout_: time::Duration,
    ) -> Result<P, ()>
    where
        F: Fn() + Send + 'static,
    {
        let hook = self.register(signal);

        let poll_handle = spawn(async move {
            loop {
                f();
                sleep(interval).await;
            }
        });

        timeout(timeout_, hook.try_wait()).await.ok();

        poll_handle.cancel().await;

        match hook.try_yield() {
            Some(x) => Ok(x),
            None => Err(()),
        }
    }
    /// signal the caller for a ready event
    ///
    /// return None if the signal match a caller, Some(payload) otherwise
    pub fn signal(&self, s: &S, payload: P) -> Option<P> {
        log::trace!("signal {:?} sent", s);
        let mut registry = self.registry.lock().unwrap();

        while registry.signals.contains_key(s) {
            let id = registry.signals.pop(s).unwrap();
            if let Some(waker) = registry.wakers.get(&id) {
                let waker = waker.clone();
                registry.payloads.insert(id, payload);
                waker.wake();
                return None;
            }
        }

        Some(payload)
    }
    fn register<'a>(&'a self, signal: S) -> Hook<'a, S, P> {
        let mut registry = self.registry.lock().unwrap();
        let id = registry.id_counter;
        registry.id_counter += 1;
        log::trace!("hook registered with signal {:?}, id {}", signal, id);

        registry.signals.push(signal, id);

        Hook { ignitor: &self, id }
    }
}

// TODO: use PhontomData to prevent calling try_yield before calling try_wait
struct Hook<'a, S, P>
where
    S: Hash + Eq + Debug,
{
    ignitor: &'a EventHook<S, P>,
    id: usize,
}

impl<'a, S, P> Hook<'a, S, P>
where
    S: Hash + Eq + Debug,
{
    fn try_wait(&'a self) -> HookPoll<'a, S, P> {
        HookPoll {
            hook: self,
            inited: Cell::new(false),
        }
    }
    fn try_yield(self) -> Option<P> {
        let mut registry = self.ignitor.registry.lock().unwrap();

        registry.wakers.remove(&self.id).unwrap();

        match registry.payloads.remove(&self.id) {
            Some(payload) => Some(payload),
            None => None,
        }
    }
}

struct HookPoll<'a, S, P>
where
    S: Hash + Eq + Debug,
{
    hook: &'a Hook<'a, S, P>,
    inited: Cell<bool>,
}

impl<'a, S, P> Future for HookPoll<'a, S, P>
where
    S: Hash + Eq + Debug,
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Self::Output> {
        log::trace!("HookPool of id {} waken", self.hook.id);
        if !self.inited.clone().take() {
            let mut registry = self.hook.ignitor.registry.lock().unwrap();

            registry.wakers.insert(self.hook.id, cx.waker().clone());

            self.inited.set(true);
            Poll::Pending
        } else {
            let registry = self.hook.ignitor.registry.lock().unwrap();
            if registry.payloads.contains_key(&self.hook.id) {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    use async_std::task::spawn;

    use super::*;

    #[async_std::test]
    async fn signal() {
        let event_q = Arc::new(EventHook::default());
        let output = Arc::new(AtomicUsize::new(0));

        let event_q1 = event_q.clone();
        spawn(async move {
            sleep(Duration::from_millis(20)).await;
            for _ in 0..100 {
                event_q1.signal(&1, 2);
            }
        });

        for _ in 0..100 {
            let event_q = event_q.clone();
            let output = output.clone();
            spawn(async move {
                event_q.wait(1).await;
                output.fetch_add(1, Ordering::Relaxed);
            });
        }

        loop {
            sleep(time::Duration::from_millis(50)).await;
            if output.load(Ordering::Relaxed) == 100 {
                break;
            }
        }
    }

    #[async_std::test]
    async fn call_prevention() {
        let event_q = Arc::new(EventHook::default());
        let output = Arc::new(AtomicUsize::new(0));

        let event_q1 = event_q.clone();
        spawn(async move {
            sleep(Duration::from_millis(3000)).await;
            for _ in 0..100 {
                event_q1.signal(&1, 2);
            }
        });

        for _ in 0..100 {
            let event_q = event_q.clone();
            let output = output.clone();
            spawn(async move {
                event_q.wait(1).await;
                output.fetch_add(1, Ordering::Relaxed);
            });
        }

        loop {
            sleep(time::Duration::from_millis(50)).await;
            if output.load(Ordering::Relaxed) == 100 {
                break;
            }
        }
    }

    #[async_std::test]
    async fn timeout() {
        let event_q = Arc::new(EventHook::<(), ()>::default());
        let output = Arc::new(AtomicUsize::new(0));

        for _ in 0..100 {
            let event_q = event_q.clone();
            let output = output.clone();
            spawn(async move {
                if event_q
                    .timeout((), Duration::from_millis(20))
                    .await
                    .is_err()
                {
                    output.fetch_add(1, Ordering::Relaxed);
                }
            });
        }

        loop {
            sleep(time::Duration::from_millis(50)).await;
            if output.load(Ordering::Relaxed) == 100 {
                break;
            }
        }
    }

    #[test]
    fn timeout_remove() {
        let event_q = Arc::new(EventHook::<_, ()>::default());
        let event_q1 = event_q.clone();
        let event_q2 = event_q.clone();
        let event_q3 = event_q.clone();

        spawn(async move {
            sleep(Duration::from_millis(40)).await;
            // this should not get its payload back
            assert!(event_q1.signal(&0, ()).is_none());
            sleep(Duration::from_millis(40)).await;
            // this should get its payload back
            assert!(event_q1.signal(&0, ()).is_some());
        });
        // this should timeout
        spawn(async move {
            assert!(event_q2
                .timeout(0, Duration::from_millis(20))
                .await
                .is_err());
        });
        // this should work
        spawn(async move { assert!(event_q3.timeout(0, Duration::from_millis(60)).await.is_ok()) });
    }
}
