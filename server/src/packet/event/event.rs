use std::cell::{Cell, UnsafeCell, RefCell};

use std::collections::BTreeMap;
use std::time::{self, Duration};

use super::ignitor;

pub struct EventHook<S>
where
    S: Ord + Clone,
{
    ignitor: RefCell<ignitor::Ignitor<S>>,
}

impl<S> EventHook<S>
where
    S: Ord + Clone,
{
    pub fn new() -> Self {
        Self {
            ignitor: RefCell::new(ignitor::Ignitor::default()),
        }
    }
    /// signal the caller for a ready event
    pub fn signal(&self, s: &S) {
        let mut ignitor=self.ignitor.borrow_mut();
        ignitor.signal(s);
    }
    /// wait for the signal
    pub async fn wait(&self, signal: S) {
        let mut ignitor=self.ignitor.borrow_mut();
        ignitor.register(signal, false,|| false).await
    }
    /// wait for the signal while keep invoking the function
    pub async fn poll_until<F>(&self, signal: S, f: F)
    where
        F: Fn(),
    {
        let mut ignitor=self.ignitor.borrow_mut();
        ignitor
            .register(signal, true,|| {
                f();
                false
            })
            .await
    }
    /// Wait for the singal till timeout
    ///
    /// Return true if it timeout
    ///
    /// # Panics
    ///
    /// Panics if SystemTime plus timeout excess the limitation
    pub async fn timeout(&self, singal: S, timeout: time::Duration) -> bool {
        let content = unsafe { &mut *self.ignitor.as_ptr() };
        let timeout = time::SystemTime::now().checked_add(timeout).unwrap();

        content
            .register(singal, true,|| timeout < time::SystemTime::now())
            .await;

        timeout < time::SystemTime::now()
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
}
