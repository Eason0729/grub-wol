// TODO: fix bug-> if event didn't yield(timeout), signals(on the Registry) would have possible memory leak

use std::cell::Cell;
use std::hash::Hash;
use std::pin::Pin;
use std::sync::Mutex;
use std::task::{self, Poll};
use std::time;
use std::{collections::*, future::Future};

use smol::future::or;

use super::hashvec::*;

struct Registry<S, P>
where
    S: Hash + Eq,
{
    id_counter: usize,
    wakers: HashMap<usize, task::Waker>,
    signals: HashVec<S, usize>,
    payloads: HashMap<usize, P>,
}

impl<S, P> Default for Registry<S, P>
where
    S: Hash + Eq,
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
    S: Hash + Eq,
{
    registry: Mutex<Registry<S, P>>,
}

impl<S, P> Default for EventHook<S, P>
where
    S: Hash + Eq,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<S, P> EventHook<S, P>
where
    S: Hash + Eq,
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
        F: Fn(),
    {
        let hook = self.register(signal);
        let id = or(
            async {
                loop {
                    f();
                    smol::Timer::after(interval).await;
                }
            },
            hook.try_wait(),
        )
        .await;

        hook.try_yield().unwrap()
    }

    /// .
    /// Wait for the singal before timeout
    ///
    /// # Panics
    ///
    /// Panics if SystemTime plus timeout excess the limitation
    ///
    /// # Errors
    ///
    /// This function will return an error if timeout
    pub async fn timeout(&self, signal: S, timeout: time::Duration) -> Result<P, ()> {
        let hook = self.register(signal);
        let id = or(
            async {
                smol::Timer::after(timeout).await;
            },
            hook.try_wait(),
        )
        .await;

        match hook.try_yield() {
            Some(x) => Ok(x),
            None => Err(()),
        }
    }
    /// .
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
        timeout: time::Duration,
    ) -> Result<P, ()>
    where
        F: Fn(),
    {
        let hook = self.register(signal);

        let id = or(
            hook.try_wait(),
            or(
                async {
                    loop {
                        f();
                        smol::Timer::after(interval).await;
                    }
                },
                async {
                    smol::Timer::after(timeout).await;
                },
            ),
        )
        .await;

        match hook.try_yield() {
            Some(x) => Ok(x),
            None => Err(()),
        }
    }
    /// signal the caller for a ready event
    ///
    /// return None if the signal match a caller, Some(payload) otherwise
    pub fn signal(&self, s: &S, payload: P) -> Option<P> {
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

        registry.signals.push(signal, id);

        Hook { ignitor: &self, id }
    }
}

struct Hook<'a, S, P>
where
    S: Hash + Eq,
{
    ignitor: &'a EventHook<S, P>,
    id: usize,
}

impl<'a, S, P> Hook<'a, S, P>
where
    S: Hash + Eq,
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
    S: Hash + Eq,
{
    hook: &'a Hook<'a, S, P>,
    inited: Cell<bool>,
}

impl<'a, S, P> Future for HookPoll<'a, S, P>
where
    S: Hash + Eq,
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Self::Output> {
        if !self.inited.clone().take() {
            let mut registry = self.hook.ignitor.registry.lock().unwrap();

            registry.wakers.insert(self.hook.id, cx.waker().clone());

            self.inited.set(true);
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}

#[cfg(test)]
mod test {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use super::*;
    use smol;

    #[test]
    fn signal() {
        let event_q = EventHook::default();
        let output = AtomicUsize::new(0);
        let ex = smol::LocalExecutor::new();

        ex.spawn(async {
            smol::Timer::after(Duration::from_millis(20)).await;
            for _ in 0..100 {
                event_q.signal(&1, 2);
            }
        })
        .detach();

        for _ in 0..100 {
            ex.spawn(async {
                event_q.wait(1).await;
                output.fetch_add(1, Ordering::Relaxed);
            })
            .detach();
        }

        loop {
            ex.try_tick();
            if ex.is_empty() {
                break;
            }
        }

        assert_eq!(output.load(Ordering::Relaxed), 100);
    }

    #[test]
    fn timeout() {
        let event_q = EventHook::<_, ()>::default();
        let output = AtomicUsize::new(0);
        let ex = smol::LocalExecutor::new();

        for _ in 0..100 {
            ex.spawn(async {
                if event_q.timeout(0, Duration::from_millis(20)).await.is_err() {
                    output.fetch_add(1, Ordering::Relaxed);
                }
            })
            .detach();
        }

        loop {
            ex.try_tick();
            if ex.is_empty() {
                break;
            }
        }

        assert_eq!(output.load(Ordering::Relaxed), 100);
    }

    #[test]
    fn timeout_remove() {
        let event_q = EventHook::<_, ()>::default();
        let ex = smol::LocalExecutor::new();

        ex.spawn(async {
            smol::Timer::after(Duration::from_millis(40)).await;
            // this should not get its payload back
            assert!(event_q.signal(&0, ()).is_none());
            smol::Timer::after(Duration::from_millis(40)).await;
            // this should get its payload back
            assert!(event_q.signal(&0, ()).is_some());
        })
        .detach();
        // this should timeout
        ex.spawn(async {
            assert!(event_q.timeout(0, Duration::from_millis(20)).await.is_err());
        })
        .detach();
        // this should work
        ex.spawn(async { assert!(event_q.timeout(0, Duration::from_millis(60)).await.is_ok()) })
            .detach();

        loop {
            ex.try_tick();
            if ex.is_empty() {
                break;
            }
        }
    }
}
