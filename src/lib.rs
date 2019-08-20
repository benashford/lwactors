extern crate futures;

use std::{error::Error, fmt};

use futures::{
    future::{self, Executor},
    sync::{mpsc, oneshot},
    Async, Future, Poll, Stream,
};

/// Construct a new actor, requires an `Executor` and an initial state.  Returns a reference that can be cheaply
/// cloned and passed between threads.  A specific implementation is expected to wrap this return value and implement
/// the required custom logic.
pub fn actor<EX, A, S, R, E>(executor: &EX, initial_state: S) -> ActorSender<A, R, E>
where
    A: Action<State = S, Result = R, Error = E> + Send + 'static,
    S: Send + 'static,
    R: Send + 'static,
    E: Send + 'static,
    EX: Executor<Box<dyn Future<Item = (), Error = ()> + Send>> + 'static,
{
    let (tx, rx) = mpsc::unbounded();
    let actor = Box::new(Actor {
        receiver: rx,
        state: initial_state,
    });
    executor
        .execute(actor)
        .expect("Cannot schedule actor on executor");
    ActorSender(tx)
}

type ActorSenderInner<A, R, E> = mpsc::UnboundedSender<(A, oneshot::Sender<Result<R, E>>)>;

#[derive(Debug)]
pub struct ActorSender<A, R, E>(ActorSenderInner<A, R, E>);

impl<A, R, E> Clone for ActorSender<A, R, E> {
    fn clone(&self) -> Self {
        ActorSender(self.0.clone())
    }
}

impl<A, R, E> ActorSender<A, R, E>
where
    A: Send + 'static,
    R: Send + 'static,
    E: Send + From<ActorError> + 'static,
{
    /// Invokes a specific action on the actor.  Returns a future that completes when the actor has
    /// performed the action
    pub fn invoke(&self, action: A) -> ActFuture<R, E> {
        let (tx, rx) = oneshot::channel();
        if let Err(e) = self.0.unbounded_send((action, tx)) {
            return Box::new(future::err(ActorError::from(e).into()));
        }
        let recv_f = rx.then(|r| {
            future::result(match r {
                Ok(Ok(r)) => Ok(r),
                Ok(Err(e)) => Err(e),
                Err(e) => Err(ActorError::from(e).into()),
            })
        });
        Box::new(recv_f)
    }
}

struct Actor<A, S, R, E> {
    receiver: mpsc::UnboundedReceiver<(A, oneshot::Sender<Result<R, E>>)>,
    state: S,
}

impl<A, S, R, E> Future for Actor<A, S, R, E>
where
    A: Action<State = S, Result = R, Error = E>,
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

pub trait Action {
    type State;
    type Result;
    type Error;

    fn act(self, s: &mut Self::State) -> Result<Self::Result, Self::Error>;
}

pub type ActFuture<R, E> = Box<dyn Future<Item = R, Error = E> + Send>;

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
            ActorError::InvokeError => "Cannot send message to actor",
            ActorError::WaitError => "Cannot wait for an answer",
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
