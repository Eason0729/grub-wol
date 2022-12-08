use std::cell::RefCell;

use std::time::{self, Duration};

use super::coll::BTreeVec;
use super::ignitor;

#[derive(Default)]
pub struct EventHook<S, P>
where
    S: Ord + Clone,
    P: Ord + Default,
{
    ignitor: ignitor::Ignitor<S, P>,
}
 
impl<S, P> EventHook<S, P>
where
    S: Ord + Clone,
    P: Ord + Default,
{
    /// signal the caller for a ready event
    pub fn signal(&self, s: S, payload: P) {
        self.ignitor.signal(&s, payload);
    }
    /// wait for the signal
    pub async fn wait(&self, signal: S) -> P {
        self.ignitor.register(signal.clone(), false, || false).await
    }
    /// wait for the signal while keep invoking the function
    pub async fn poll_until<F>(&self, signal: S, f: F) -> P
    where
        F: Fn(),
    {
        self.ignitor
            .register(signal, true, || {
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
    pub async fn timeout(&self, singal: S, timeout: time::Duration) -> Result<P, ()> {
        let timeout = time::SystemTime::now().checked_add(timeout).unwrap();

        let payload = self
            .ignitor
            .register(singal, true, || timeout < time::SystemTime::now())
            .await;

        if timeout < time::SystemTime::now() {
            Err(())
        } else {
            Ok(payload)
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
    fn basic() {
        let event_q = EventHook::default();
        let output = AtomicUsize::new(0);
        let ex = smol::LocalExecutor::new();

        ex.spawn(async {
            smol::Timer::after(Duration::from_secs(2)).await;
            for _ in 0..200 {
                event_q.signal(1, 2);
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
