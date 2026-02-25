use lwactors::{actor, Action, ActorError, ActorSender};

#[derive(Debug, thiserror::Error)]
pub enum PingPongError {
    #[error("Actor error: {0}")]
    Actor(#[from] ActorError),
}

// Ping Actor
struct PingState {
    count: u32,
    limit: u32,
    pong_tx: Option<ActorSender<PongAction, (), PingPongError>>,
}

enum PingAction {
    SetPong(ActorSender<PongAction, (), PingPongError>),
    Ping,
}

impl Action for PingAction {
    type State = PingState;
    type Result = ();
    type Error = PingPongError;

    fn act(self, state: &mut Self::State) -> Result<Self::Result, Self::Error> {
        match self {
            PingAction::SetPong(tx) => {
                state.pong_tx = Some(tx);
            }
            PingAction::Ping => {
                state.count += 1;
                println!("Ping! ({})", state.count);
                if state.count < state.limit {
                    if let Some(ref pong) = state.pong_tx {
                        // Fire and forget: we don't await the result here.
                        // In a real application, you might want to handle errors
                        // if the other actor has died.
                        let _ = pong.invoke(PongAction::Pong);
                    }
                } else {
                    println!("Ping limit reached, stopping.");
                }
            }
        }
        Ok(())
    }
}

// Pong Actor
struct PongState {
    ping_tx: ActorSender<PingAction, (), PingPongError>,
}

enum PongAction {
    Pong,
}

impl Action for PongAction {
    type State = PongState;
    type Result = ();
    type Error = PingPongError;

    fn act(self, state: &mut Self::State) -> Result<Self::Result, Self::Error> {
        match self {
            PongAction::Pong => {
                println!("Pong!");
                let _ = state.ping_tx.invoke(PingAction::Ping);
            }
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create Ping actor with an initial state, but no Pong sender yet.
    let ping_tx = actor::<PingAction, _, _, _>(PingState {
        count: 0,
        limit: 5,
        pong_tx: None,
    });

    // 2. Create Pong actor, giving it Ping's sender.
    let pong_tx = actor::<PongAction, _, _, _>(PongState {
        ping_tx: ping_tx.clone(),
    });

    // 3. Tell Ping about Pong.
    ping_tx.invoke(PingAction::SetPong(pong_tx)).await?;

    // 4. Start the game.
    println!("Starting Ping-Pong game...");
    ping_tx.invoke(PingAction::Ping).await?;

    // Wait a bit for the messages to finish processing as we are using fire-and-forget
    // for the subsequent pings/pongs.
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    Ok(())
}
