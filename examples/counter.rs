use futures::future;

use lwactors::{Action, ActorError, ActorSender};

#[tokio::main]
async fn main() {
    let counter = Counter::new();
    do_counter(counter).await;
}

async fn do_counter(counter: Counter) {
    let counter1 = counter.clone();

    let a = tokio::spawn(async move {
        let futs: Vec<_> = (0..100).map(|i| counter1.add(i)).collect();
        let results = future::join_all(futs).await;
        println!("ADD RESULTS: {:?}", results);
    });

    let b = tokio::spawn(async move {
        let futs: Vec<_> = (0..100).map(|i| counter.subtract(i)).collect();
        let results = future::join_all(futs).await;
        println!("SUB RESULTS: {:?}", results);
    });

    a.await.expect("Task one failed");
    b.await.expect("Task two failed");
}

#[derive(Clone)]
struct Counter {
    queue: ActorSender<CounterAction, i64, CounterError>,
}

type CounterResult = Result<i64, CounterError>;

impl Counter {
    fn new() -> Counter {
        Counter {
            queue: lwactors::actor(0),
        }
    }

    async fn add(&self, n: i64) -> CounterResult {
        self.queue.invoke(CounterAction::Add(n)).await
    }

    async fn subtract(&self, n: i64) -> CounterResult {
        self.queue.invoke(CounterAction::Subtract(n)).await
    }
}

#[derive(Debug)]
enum CounterAction {
    Add(i64),
    Subtract(i64),
}

impl Action for CounterAction {
    type State = i64;
    type Result = i64;
    type Error = CounterError;

    fn act(self, state: &mut Self::State) -> Result<Self::Result, Self::Error> {
        println!("Acting {:?} on {}", self, state);
        match self {
            CounterAction::Add(i) => *state += i,
            CounterAction::Subtract(i) => *state -= i,
        }
        Ok(*state)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CounterError {
    #[error("Counter error: {0}")]
    General(String),
}

impl From<ActorError> for CounterError {
    fn from(e: ActorError) -> Self {
        CounterError::General(e.to_string())
    }
}
