#![feature(async_await)]

use std::{error::Error, fmt};

use futures::{
    channel::{mpsc, oneshot},
    stream::StreamExt,
    task::SpawnExt,
};

/// Construct a new actor, requires an `Executor` and an initial state.  Returns a reference that can be cheaply
/// cloned and passed between threads.  A specific implementation is expected to wrap this return value and implement
/// the required custom logic.
pub fn actor<A, S, R, E>(mut executor: impl SpawnExt, initial_state: S) -> ActorSender<A, R, E>
where
    A: Action<State = S, Result = R, Error = E> + Send + 'static,
    S: Send + 'static,
    R: Send + 'static,
    E: Send + 'static,
{
    let (tx, rx) = mpsc::unbounded();
    executor
        .spawn(actor_future(rx, initial_state))
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
    pub async fn invoke(&self, action: A) -> ActResult<R, E> {
        let (tx, rx) = oneshot::channel();
        if let Err(e) = self.0.unbounded_send((action, tx)) {
            return Err(ActorError::from(e).into());
        }
        match rx.await {
            Ok(Ok(r)) => Ok(r),
            Ok(Err(e)) => Err(e),
            Err(e) => Err(ActorError::from(e).into()),
        }
    }
}

async fn actor_future<A, S, R, E>(
    mut receiver: mpsc::UnboundedReceiver<(A, oneshot::Sender<Result<R, E>>)>,
    mut state: S,
) where
    A: Action<State = S, Result = R, Error = E>,
{
    while let Some((a, tx)) = receiver.next().await {
        let _ = tx.send(a.act(&mut state));
    }
}

pub trait Action {
    type State;
    type Result;
    type Error;

    fn act(self, s: &mut Self::State) -> Result<Self::Result, Self::Error>;
}

pub type ActResult<R, E> = Result<R, E>;

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

impl<T> From<mpsc::TrySendError<T>> for ActorError {
    fn from(_from: mpsc::TrySendError<T>) -> Self {
        ActorError::InvokeError
    }
}

impl From<oneshot::Canceled> for ActorError {
    fn from(_from: oneshot::Canceled) -> Self {
        ActorError::WaitError
    }
}
