use super::coll::BTreeVec;
use std::cell::RefCell;
use std::pin::Pin;
use std::task::{self};
use std::{collections::*, future::Future};

pub struct Registry<S, P>
where
    S: Ord + Clone,
    P: Ord + Default,
{
    registry_counter: usize,
    id_counter: usize,
    pending: BTreeSet<usize>,
    wakers: BTreeVec<S, (usize, task::Waker)>,
    payloads: BTreeVec<usize, P>,
}

pub struct Ignitor<S, P>
where
    S: Ord + Clone,
    P: Ord + Default,
{
    registry: RefCell<Registry<S, P>>,
}

impl<S, P> Ignitor<S, P>
where
    S: Ord + Clone,
    P: Ord + Default,
{
    pub fn new() -> Self {
        Self {
            registry: RefCell::new(Registry {
                registry_counter: 0,
                id_counter: 0,
                pending: BTreeSet::default(),
                wakers: BTreeVec::default(),
                payloads: BTreeVec::default(),
            }),
        }
    }
    pub fn cancel(&self, id: usize) -> bool {
        let mut registry = self.registry.borrow_mut();
        if let Some((_, (_, waker))) = registry.wakers.find_pop(move |(pid, _)| *pid != id) {
            registry.pending.remove(&id);
            waker.wake();
            true
        } else {
            false
        }
    }
    pub fn signal(&self, s: &S, payload: P) {
        let mut registry = self.registry.borrow_mut();
        if let Some((id, waker)) = registry.wakers.pop(s) {
            registry.payloads.push(id, payload);
            registry.pending.remove(&id);
            waker.wake();
        }
    }
    #[inline]
    pub fn register<F>(&self, signal: S, should_wake: bool, f: F) -> SignalPoll<S, P, F>
    where
        F: Fn() -> bool,
    {
        let mut registry = self.registry.borrow_mut();
        let id = registry.id_counter;
        registry.id_counter += 1;
        registry.pending.insert(id);
        SignalPoll {
            f,
            ignitor: &self,
            signal,
            id,
            should_wake,
        }
    }
}

impl<S, P> Default for Ignitor<S, P>
where
    S: Ord + Clone,
    P: Ord + Default,
{
    fn default() -> Self {
        Self::new()
    }
}

pub struct SignalPoll<'a, S, P, F>
where
    S: Ord + Clone,
    P: Ord + Default,
    F: Fn() -> bool,
{
    f: F,
    ignitor: &'a Ignitor<S, P>,
    signal: S,
    id: usize,
    should_wake: bool,
}

impl<'a, S, P, F> Future for SignalPoll<'a, S, P, F>
where
    S: Ord + Clone,
    P: Ord + Default,
    F: Fn() -> bool,
{
    type Output = P;

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> task::Poll<Self::Output> {
        if (self.f)() {
            return task::Poll::Ready(P::default());
        }

        let id = self.id;
        let signal = self.signal.clone();

        let mut registry = self.ignitor.registry.borrow_mut();

        if registry.registry_counter == id {
            registry.registry_counter += 1;
            registry.wakers.push(signal, (id, cx.waker().clone()));
        } else if registry.registry_counter > id {
            if registry.pending.get(&id).is_none() {
                let payload = registry.payloads.pop(&id).unwrap();
                return task::Poll::Ready(payload);
            }
            if self.should_wake {
                cx.waker().wake_by_ref();
            }
        }
        task::Poll::Pending
    }
}

#[cfg(test)]
mod test {
    use std::cell::Cell;
    use std::sync::Mutex;
    use std::time::Duration;

    use super::*;
    use smol;

    #[test]
    fn basic() {
        let ignitor = Ignitor::default();
        let output = Mutex::new(0_usize);
        let ex = smol::LocalExecutor::new();

        ex.spawn(async {
            smol::Timer::after(Duration::from_secs(2)).await;
            ignitor.signal(&1, ());
        })
        .detach();

        ex.spawn(async {
            ignitor.register(1, false, || false).await;
            *output.lock().unwrap() = 1;
        })
        .detach();

        loop {
            ex.try_tick();
            if ex.is_empty() {
                break;
            }
        }
        assert_eq!(*output.lock().unwrap(), 1);
    }

    #[test]
    fn payload() {
        let ignitor = Ignitor::default();
        let output = Mutex::new(0_usize);
        let ex = smol::LocalExecutor::new();

        ex.spawn(async {
            smol::Timer::after(Duration::from_secs(2)).await;
            ignitor.signal(&1, 3_usize);
        })
        .detach();

        ex.spawn(async {
            *output.lock().unwrap() = ignitor.register(1, false, || false).await;
        })
        .detach();

        loop {
            ex.try_tick();
            if ex.is_empty() {
                break;
            }
        }
        assert_eq!(*output.lock().unwrap(), 3_usize);
    }

    #[test]
    fn poll() {
        let ignitor = Ignitor::default();
        let output = Cell::new(0_usize);
        let ex = smol::LocalExecutor::new();

        ex.spawn(async {
            smol::Timer::after(Duration::from_secs(2)).await;
            ignitor.signal(&1, ());
        })
        .detach();

        ex.spawn(async {
            ignitor
                .register(1, true, || {
                    output.set(output.get() + 1);
                    false
                })
                .await;
        })
        .detach();

        loop {
            ex.try_tick();
            if ex.is_empty() {
                break;
            }
        }
        assert!(output.take() > 0);
    }
    #[test]
    fn break_poll() {
        let ignitor: Ignitor<i32, ()> = Ignitor::default();
        let output = Mutex::new(0_usize);
        let ex = smol::LocalExecutor::new();

        ex.spawn(async {
            ignitor.register(1, true, || true).await;
            *output.lock().unwrap() = 1;
        })
        .detach();

        loop {
            ex.try_tick();
            if ex.is_empty() {
                break;
            }
        }

        assert_eq!(*output.lock().unwrap(), 1);
    }
}
