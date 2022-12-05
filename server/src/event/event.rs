use std::cell::{Cell, UnsafeCell};

use super::ignitor;

pub struct EventHook<S>
where
    S: Ord + Clone,
{
    content: Cell<ignitor::Ignitor<S>>,
}

impl<S> EventHook<S>
where
    S: Ord + Clone,
{
    pub fn new() -> Self {
        Self {
            content: Cell::new(ignitor::Ignitor::default()),
        }
    }
    pub fn signal(&self, s: &S) {
        let content = unsafe { &mut *self.content.as_ptr() };
        content.signal(s);
    }
    pub async fn register(&self, s: S) {
        let content = unsafe { &mut *self.content.as_ptr() };
        content.register(s).await
    }
    pub async fn poll_until(&self, signal: S, f: impl Fn() + 'static) {
        let content = unsafe { &mut *self.content.as_ptr() };
        content.poll_until(signal, f).await
    }
}

impl<S> Default for EventHook<S>
where
    S: Ord + Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use super::*;
    use smol;

    #[test]
    fn basic() {
        let event_q = EventHook::default();
        let output = AtomicUsize::new(0);
        let ex = smol::LocalExecutor::new();

        ex.spawn(async {
            smol::Timer::after(Duration::from_secs(2)).await;
            for _ in 0..200 {
                event_q.signal(&1);
            }
        })
        .detach();

        for _ in 0..100 {
            ex.spawn(async {
                event_q.register(1).await;
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
}
