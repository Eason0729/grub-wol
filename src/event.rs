use smol;
use std::{collections::VecDeque, future::Future};

struct EventQueue<'a, S, F>
where
    F: Future<Output = ()> + 'static,
{
    que: Vec<Events<S, F>>,
    ex: smol::LocalExecutor<'a>,
}

impl<'a, S, F> EventQueue<'a, S, F>
where
    S: Eq,
    F: Future<Output = ()> + 'static,
{
    async fn singal(&mut self, signal: S) {
        let mut finished_event = VecDeque::default();
        for i in (0..self.que.len()).rev() {
            let item = &mut self.que[i];
            let fulfilled = match item.next_signal() {
                Some(s) => &signal == s,
                None => true,
            };
            if fulfilled {
                item.execute(&self.ex).await;
                if item.is_empty() {
                    finished_event.push_back(i);
                }
            }
        }
        for delete_index in finished_event {
            self.que.swap_remove(delete_index);
        }
    }
    async fn schedule(){

    }
}

struct Event<S, F>
where
    F: Future<Output = ()> + 'static,
{
    signal: Option<S>,
    affair: Box<F>,
}

struct Events<S, F>(VecDeque<Event<S, F>>)
where
    F: Future<Output = ()> + 'static;

impl<S, F> Events<S, F>
where
    F: Future<Output = ()> + 'static,
{
    fn new() -> Self {
        Self(VecDeque::default())
    }
    fn chain(mut self, signal: S, event: F) -> Self {
        let event = Event {
            signal: Some(signal),
            affair: Box::new(event),
        };
        self.0.push_back(event);
        self
    }
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    fn next_signal(&self) -> &Option<S> {
        &self.0.front().unwrap().signal
    }
    async fn execute<'a>(&mut self, ex: &smol::LocalExecutor<'a>) {
        let affair = self.0.pop_front().unwrap();
        ex.run(*affair.affair).await;
    }
}
