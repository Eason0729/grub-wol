use std::cell::RefCell;
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
    fn find_pop(&mut self, f: impl Fn(&V) -> bool) -> Option<(K, V)> {
        let mut element = None;
        'outer: for (key, val) in &mut self.tree {
            for i in (0..val.len()).rev() {
                if !f(&val[i]) {
                    element = Some((key.clone(), val.swap_remove(i)));
                    break 'outer;
                }
            }
        }
        element
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

pub struct Ignitor<S>
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
    pub fn new() -> Self {
        Self {
            registry_counter: 0,
            id_counter: 0,
            pending: BTreeSet::default(),
            registry: BTreeVec::default(),
        }
    }
    pub fn cancel(&mut self, id: usize) -> bool {
        if let Some((_, (_, waker))) = self.registry.find_pop(move |(pid, _)| *pid != id) {
            self.pending.remove(&id);
            waker.wake();
            true
        } else {
            false
        }
    }
    pub fn signal(&mut self, s: &S) {
        if let Some((id, waker)) = self.registry.pop(s) {
            self.pending.remove(&id);
            waker.wake();
        }
    }
    pub async fn register(&mut self, signal: S) -> Result<(), Error> {
        let id = self.id_counter;
        self.id_counter += 1;
        self.pending.insert(id);
        SignalWait {
            ignitor: RefCell::new(self),
            signal,
            id,
        }
        .await
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

#[derive(Debug)]
pub enum Error {
    UserInterruption,
}

struct SignalWait<'a, S>
where
    S: Ord + Clone,
{
    ignitor: RefCell<&'a mut Ignitor<S>>,
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
        let ignitor = &mut self.ignitor.borrow_mut();

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
    use std::cell::UnsafeCell;
    use std::sync::Mutex;
    use std::time::Duration;

    use super::*;
    use smol;

    #[test]
    fn basic() {
        let mut ignitor = Ignitor::default();
        let ignitor = UnsafeCell::new(&mut ignitor);
        let output = Mutex::new(0_usize);
        let ex = smol::LocalExecutor::new();

        ex.spawn(async {
            let ignitor: &mut Ignitor<usize> = unsafe { *ignitor.get() };
            smol::Timer::after(Duration::from_secs(2)).await;
            ignitor.signal(&1);
        })
        .detach();

        ex.spawn(async {
            let ignitor: &mut Ignitor<usize> = unsafe { *ignitor.get() };
            ignitor.register(1).await.unwrap();
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
