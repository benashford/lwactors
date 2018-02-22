extern crate futures;
extern crate futures_cpupool;

use futures::{Async, Future, Poll, Stream};
use futures::future::Executor;
use futures::sync::mpsc;
use futures_cpupool::CpuPool;

const DEFAULT_SIZE: usize = 8;

pub fn actor<A, S>(cpu_pool: &CpuPool, initial_state: S) -> mpsc::Sender<A>
where
    A: Action<S> + Send + 'static,
    S: Send + 'static,
{
    let (tx, rx) = mpsc::channel(DEFAULT_SIZE);
    let actor = Actor {
        receiver: rx,
        state: initial_state,
    };
    cpu_pool
        .execute(actor)
        .expect("Cannot schedule actor on executor");
    tx
}

pub struct Actor<A, S> {
    receiver: mpsc::Receiver<A>,
    state: S,
}

impl<A, S> Future for Actor<A, S>
where
    A: Action<S>,
{
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            match self.receiver.poll() {
                Ok(Async::Ready(Some(a))) => a.act(&mut self.state),
                Ok(Async::Ready(None)) => return Ok(Async::Ready(())),
                Ok(Async::NotReady) => return Ok(Async::NotReady),
                Err(()) => return Err(()),
            }
        }
    }
}

pub trait Action<S> {
    fn act(self, s: &mut S);
}
