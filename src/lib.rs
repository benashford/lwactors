#[cfg(feature = "async-global-executor14")]
extern crate async_global_executor14 as async_global_executor;
#[cfg(feature = "tokio02")]
extern crate tokio02 as tokio;
#[cfg(feature = "tokio10")]
extern crate tokio10 as tokio;

use std::future::Future;

use futures_channel::{mpsc, oneshot};
use futures_util::{
    future::{self, Either, FutureExt},
    stream::StreamExt,
    task::SpawnExt,
};

use thiserror::Error;

/// Construct a new actor, requires an `Executor` and an initial state.  Returns a reference that can be cheaply
/// cloned and passed between threads.  A specific implementation is expected to wrap this return value and implement
/// the required custom logic.
pub fn actor_with_executor<A, S, R, E>(
    executor: impl SpawnExt,
    initial_state: S,
) -> ActorSender<A, R, E>
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

#[cfg(feature = "__global_executor")]
pub fn actor<A, S, R, E>(initial_state: S) -> ActorSender<A, R, E>
where
    A: Action<State = S, Result = R, Error = E> + Send + 'static,
    S: Send + 'static,
    R: Send + 'static,
    E: Send + 'static,
{
    let (tx, rx) = mpsc::unbounded();

    #[cfg(feature = "async-global-executor14")]
    async_global_executor::spawn(actor_future(rx, initial_state)).detach();

    #[cfg(feature = "__tokio")]
    tokio::spawn(actor_future(rx, initial_state));

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
    /// performed the action.
    ///
    /// Specifically not an `async` function to allow it to be used correctly in a fire-and-forget
    /// manner, although this is not advised.
    pub fn invoke(&self, action: A) -> impl Future<Output = ActResult<R, E>> {
        let (tx, rx) = oneshot::channel();
        if let Err(e) = self.0.unbounded_send((action, tx)) {
            Either::Right(future::err(ActorError::from(e).into()))
        } else {
            Either::Left(rx.then(|result| match result {
                Ok(Ok(r)) => future::ok(r),
                Ok(Err(e)) => future::err(e),
                Err(e) => future::err(ActorError::from(e).into()),
            }))
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

#[derive(Debug, Error)]
pub enum ActorError {
    /// Cannot send message to the actor
    #[error("Cannot send message to actor")]
    InvokeError,
    /// Response was cancelled before being received.
    #[error("Cannot wait for an answer")]
    WaitError,
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
