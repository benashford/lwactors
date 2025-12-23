use std::thread;

use futures::future;
use tokio::runtime::Builder;

use lwactors::{Action, ActorError, ActorSender};

fn block_on<F>(f: F) -> F::Output
where
    F: Future,
{
    let rt = Builder::new_current_thread()
        .build()
        .expect("Failed to build tokio runtime");
    rt.block_on(f)
}

fn main() {
    let rt = Builder::new_multi_thread()
        .build()
        .expect("Failed to build tokio runtime");
    let _guard = rt.enter();
    let counter = Counter::new();
    do_counter(counter);
}

fn do_counter(counter: Counter) {
    let counter1 = counter.clone();

    let a = thread::spawn(move || {
        let futs: Vec<_> = (0..100).map(|i| counter1.add(i)).collect();
        let results = block_on(future::join_all(futs));
        println!("ADD RESULTS: {:?}", results);
    });

    let b = thread::spawn(move || {
        let futs: Vec<_> = (0..100).map(|i| counter.subtract(i)).collect();
        let results = block_on(future::join_all(futs));
        println!("SUB RESULTS: {:?}", results);
    });

    a.join().expect("Thread one failed");
    b.join().expect("Thread two failed");
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

#[derive(Debug)]
struct CounterError;

impl From<ActorError> for CounterError {
    fn from(_: ActorError) -> Self {
        CounterError
    }
}
