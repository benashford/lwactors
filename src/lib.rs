extern crate futures;
extern crate futures_cpupool;

use std::error::Error;
use std::fmt;

use futures::{Async, Future, Poll, Sink, Stream};
use futures::future::Executor;
use futures::sync::{mpsc, oneshot};
use futures_cpupool::CpuPool;

const DEFAULT_SIZE: usize = 8;

pub fn actor<A, S, R>(cpu_pool: &CpuPool, initial_state: S) -> ActorSender<A, R>
where
    A: Action<S, R> + Send + 'static,
    S: Send + 'static,
    R: Send + 'static,
{
    let (tx, rx) = mpsc::channel(DEFAULT_SIZE);
    let actor = Actor {
        receiver: rx,
        state: initial_state,
    };
    cpu_pool
        .execute(actor)
        .expect("Cannot schedule actor on executor");
    ActorSender(tx)
}

type ActorSenderInner<A, R> = mpsc::Sender<(A, oneshot::Sender<R>)>;

#[derive(Debug)]
pub struct ActorSender<A, R>(ActorSenderInner<A, R>);

impl<A, R> Clone for ActorSender<A, R> {
    fn clone(&self) -> Self {
        ActorSender(self.0.clone())
    }
}

impl<A, R> ActorSender<A, R>
where
    A: 'static,
    R: 'static,
{
    pub fn invoke<E>(&self, action: A) -> ActFuture<R, E>
    where
        E: From<ActorError>,
    {
        let (tx, rx) = oneshot::channel();
        let send_f = self.0.clone().send((action, tx));
        let act_f = send_f.map_err(|e| e.into()).join(rx.map_err(|e| e.into()));
        Box::new(
            act_f
                .map(|(_, result)| result)
                .map_err(|e: ActorError| e.into()),
        )
    }
}

pub struct Actor<A, S, R> {
    receiver: mpsc::Receiver<(A, oneshot::Sender<R>)>,
    state: S,
}

impl<A, S, R> Future for Actor<A, S, R>
where
    A: Action<S, R>,
{
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            match self.receiver.poll() {
                Ok(Async::Ready(Some((a, tx)))) => {
                    // Not checking the result, as nothing may be waiting
                    let _ = tx.send(a.act(&mut self.state));
                }
                Ok(Async::Ready(None)) => return Ok(Async::Ready(())),
                Ok(Async::NotReady) => return Ok(Async::NotReady),
                Err(()) => return Err(()),
            }
        }
    }
}

pub trait Action<S, R> {
    fn act(self, s: &mut S) -> R;
}

pub type ActFuture<R, E> = Box<Future<Item = R, Error = E>>;

#[derive(Debug)]
pub enum ActorError {
    /// Cannot send message to the actor
    InvokeError,
    /// Response was cancelled before being received.
    WaitError,
}

impl Error for ActorError {
    fn description(&self) -> &str {
        match self {
            &ActorError::InvokeError => "Cannot send message to actor",
            &ActorError::WaitError => "Cannot wait for an answer",
        }
    }
}

impl fmt::Display for ActorError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self.description())
    }
}

impl<T> From<mpsc::SendError<T>> for ActorError {
    fn from(_from: mpsc::SendError<T>) -> Self {
        ActorError::InvokeError
    }
}

impl From<oneshot::Canceled> for ActorError {
    fn from(_from: oneshot::Canceled) -> Self {
        ActorError::WaitError
    }
}
