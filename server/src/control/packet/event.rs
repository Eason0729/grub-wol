// TODO: fix bug-> if event didn't yield(timeout), signals(on the Registry) would have possible memory leak

use std::cell::{Cell, RefCell};
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
        let id = self.register(signal).await;

        let mut registry = self.registry.lock().unwrap();
        registry.payloads.remove(&id).unwrap()
    }
    /// wait for the signal while keep invoking the function
    pub async fn poll_until<F>(&self, signal: S, f: F, interval: time::Duration) -> P
    where
        F: Fn(),
    {
        let id = or(
            async {
                loop {
                    f();
                    smol::Timer::after(interval).await;
                }
            },
            self.register(signal),
        )
        .await;

        let mut registry = self.registry.lock().unwrap();
        registry.payloads.remove(&id).unwrap()
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
        let id = or(
            async {
                smol::Timer::after(timeout).await;
                0
            },
            self.register(signal),
        )
        .await;

        if id == 0 {
            Err(())
        } else {
            let mut registry = self.registry.lock().unwrap();
            let payload = registry.payloads.remove(&id).unwrap();
            Ok(payload)
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
        let id = or(
            self.register(signal),// marked here: #1
            or(
                async {
                    loop {
                        f();
                        smol::Timer::after(interval).await;
                    }
                },
                async {
                    smol::Timer::after(timeout).await;
                    0
                },
            ),
        )
        .await;
        // ``self.register(signal)`` drop here: #2
        // and what if ``self.register(signal)`` yield between #1 and #2?

        // TODO: fix fatal logical error
        if id == 0 {
            Err(())
        } else {
            let mut registry = self.registry.lock().unwrap();
            let payload = registry.payloads.remove(&id).unwrap();
            Ok(payload)
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
    fn register(&self, signal: S) -> SignalPoll<S, P> {
        let mut registry = self.registry.lock().unwrap();
        let id = registry.id_counter;
        registry.id_counter += 1;

        registry.signals.push(signal, id);

        SignalPoll {
            ignitor: &self,
            id,
            inited: Cell::new(false),
        }
    }
}

pub struct SignalPoll<'a, S, P>
where
    S: Hash + Eq,
{
    ignitor: &'a EventHook<S, P>,
    id: usize,
    inited: Cell<bool>,
}

impl<'a, S, P> Drop for SignalPoll<'a, S, P>
where
    S: Hash + Eq,
{
    fn drop(&mut self) {
        let mut registry = self.ignitor.registry.lock().unwrap();
        registry.wakers.remove(&self.id);
    }
}

impl<'a, S, P> Future for SignalPoll<'a, S, P>
where
    S: Hash + Eq,
{
    type Output = usize;

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> task::Poll<Self::Output> {
        if !self.inited.clone().take() {
            let mut registry = self.ignitor.registry.lock().unwrap();
            registry.wakers.insert(self.id, cx.waker().clone());
            self.inited.set(true);
            Poll::Pending
        } else {
            Poll::Ready(self.id)
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
