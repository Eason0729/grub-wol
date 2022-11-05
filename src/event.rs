use std::cell::{Cell, RefCell};
use std::pin::Pin;
use std::task::{self};
use std::{collections::*, future::Future};

struct BTreeVec<K, V>
where
    K: Ord + Clone,
{
    tree: BTreeMap<K, Vec<V>>,
}

impl<K, V> BTreeVec<K, V>
where
    K: Ord + Clone,
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

impl<K, V> Default for BTreeVec<K, V>
where
    K: Ord + Clone,
{
    fn default() -> Self {
        Self {
            tree: Default::default(),
        }
    }
}

struct Ignitor<S>
where
    S: Ord + Clone,
{
    registry_counter: usize,
    id_counter: usize,
    pending: BTreeSet<usize>,
    registry: BTreeVec<S, (usize, task::Waker)>,
}

impl<S> Ignitor<S>
where
    S: Ord + Clone,
{
    fn new() -> Self {
        Self {
            registry_counter: 0,
            id_counter: 0,
            pending: BTreeSet::default(),
            registry: BTreeVec::default(),
        }
    }
    fn signal(&mut self, s: &S) {
        if let Some((id, waker)) = self.registry.pop(s) {
            self.pending.remove(&id);
            waker.wake();
        }
    }
    async fn register(&mut self, signal: S) -> Result<(), Error> {
        // let id = self.id_counter;
        // self.id_counter += 1;
        // self.pending.insert(id);
        // SignalWait {
        //     ignitor: self as *mut _,
        //     _ignitor: self,
        //     signal,
        //     id,
        // }
        // .await?;
        // Ok(())
        todo!()
    }
}

impl<S> Default for Ignitor<S>
where
    S: Ord + Clone,
{
    fn default() -> Self {
        Self::new()
    }
}
enum Error {
    UserInterruption,
}

struct SignalWait<'a, S>
where
    S: Ord + Clone,
{
    ignitor: *mut Ignitor<S>,
    _ignitor: &'a mut Ignitor<S>,
    signal: S,
    id: usize,
}

impl<'a, S> Future for SignalWait<'a, S>
where
    S: Ord + Clone,
{
    type Output = Result<(), Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> task::Poll<Self::Output> {
        let waker = cx.waker().clone();
        let id = self.id;
        let signal = self.signal.clone();
        let ignitor = unsafe { &mut *self.ignitor };

        if ignitor.registry_counter == id {
            ignitor.registry_counter += 1;
            ignitor.registry.push(signal, (id, waker));
        } else if ignitor.registry_counter > id {
            if ignitor.pending.get(&id).is_none() {
                return task::Poll::Ready(Ok(()));
            }
        }

        task::Poll::Pending
    }
}

#[cfg(test)]
mod test {
    use std::future::join;

    use std::thread;
    use std::time::Duration;

    use super::*;
    use smol;
    use smol::lock::Mutex;

    #[test]
    fn basic() {
        // let mut ignitor = Ignitor::default();

        // let fa = async {
        //     smol::Timer::after(Duration::from_secs(2)).await;
        //     ignitor.signal(&1);
        // };
        // let fb = async {
        //     ignitor.register(1).await;
        //     println!("t");
        // };

        // smol::block_on(async {
        // });
    }
}
