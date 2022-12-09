use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::pin::Pin;
use std::task::{self, Poll};
use std::time;
use std::{collections::*, future::Future};

use smol::future::or;

struct BTreeVec<K, V>
where
    K: Ord,
{
    tree: BTreeMap<K, Vec<V>>,
}

impl<K, V> Default for BTreeVec<K, V>
where
    K: Ord,
{
    fn default() -> Self {
        Self {
            tree: Default::default(),
        }
    }
}

impl<K, V> BTreeVec<K, V>
where
    K: Ord,
{
    fn push(&mut self, key: K, val: V) {
        if let Some(content) = self.tree.get_mut(&key) {
            content.push(val);
        } else {
            self.tree.insert(key, vec![val]);
        }
    }
    fn pop(&mut self, key: &K) -> Option<V> {
        if let Some(content) = self.tree.get_mut(key) {
            let result = content.pop();
            if content.is_empty() {
                self.tree.remove(key);
            }
            return result;
        }
        None
    }
    fn is_empty(&self, key: &K) -> bool {
        if let Some(x) = self.tree.get(key) {
            x.is_empty()
        } else {
            true
        }
    }
}

struct Registry<S, P>
where
    S: Ord,
{
    id_counter: usize,
    wakers: HashMap<usize, task::Waker>,
    signals: BTreeVec<S, usize>,
    payloads: HashMap<usize, P>,
}

impl<S, P> Default for Registry<S, P>
where
    S: Ord,
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
    S: Ord,
{
    registry: RefCell<Registry<S, P>>,
}

impl<S, P> Default for EventHook<S, P>
where
    S: Ord,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<S, P> EventHook<S, P>
where
    S: Ord,
{
    pub fn new() -> Self {
        Self {
            registry: RefCell::default(),
        }
    }
    /// wait for the signal
    pub async fn wait(&self, signal: S) -> P {
        let id = self.register(signal).await;

        let mut registry = self.registry.borrow_mut();
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

        let mut registry = self.registry.borrow_mut();
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
            let mut registry = self.registry.borrow_mut();
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
            self.register(signal),
        )
        .await;

        if id == 0 {
            Err(())
        } else {
            let mut registry = self.registry.borrow_mut();
            let payload = registry.payloads.remove(&id).unwrap();
            Ok(payload)
        }
    }
    /// signal the caller for a ready event
    ///
    /// return None if the signal match a caller, Some(payload) otherwise
    pub fn signal(&self, s: &S, payload: P) -> Option<P> {
        let mut registry = self.registry.borrow_mut();

        if let Some(id) = registry.signals.pop(s) {
            registry.payloads.insert(id, payload);
            registry.wakers.remove(&id).unwrap().wake();
            None
        } else {
            Some(payload)
        }
    }
    fn register(&self, signal: S) -> SignalPoll<S, P> {
        let mut registry = self.registry.borrow_mut();
        let id = registry.id_counter;
        registry.id_counter += 1;
        SignalPoll {
            ignitor: &self,
            id,
            inited: Cell::new(false),
        }
    }
}

pub struct SignalPoll<'a, S, P>
where
    S: Ord,
{
    ignitor: &'a EventHook<S, P>,
    id: usize,
    inited: Cell<bool>,
}

impl<'a, S, P> Future for SignalPoll<'a, S, P>
where
    S: Ord,
{
    type Output = usize;

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> task::Poll<Self::Output> {
        if !self.inited.clone().take() {
            let mut registry = self.ignitor.registry.borrow_mut();
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
            for _ in 0..200 {
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
}
