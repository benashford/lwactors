use std::thread;

use futures::{
    executor::{block_on, ThreadPool},
    future,
};

use lwactors::{actor, Action, ActorError, ActorSender};

fn main() {
    let cpu_pool = ThreadPool::new().expect("Can't create ThreadPool");
    let counter = Counter::new(cpu_pool.clone());
    let counter1 = counter.clone();

    let a = thread::spawn(move || {
        let futs: Vec<_> = (0..100).map(|i| counter1.add(i)).collect();
        let results = block_on(future::join_all(futs));
        println!("ADD RESULTS: {:?}", results);
    });

    let counter2 = counter.clone();

    let b = thread::spawn(move || {
        let futs: Vec<_> = (0..100).map(|i| counter2.subtract(i)).collect();
        let results = block_on(future::join_all(futs));
        println!("SUB RESULTS: {:?}", results);
    });

    a.join().expect("Thread one failed");
    b.join().expect("Thread two failed");

    println!("Finished: {:?}", cpu_pool);
}

#[derive(Clone)]
struct Counter {
    queue: ActorSender<CounterAction, i64, CounterError>,
}

type CounterResult = Result<i64, CounterError>;

impl Counter {
    fn new(thread_pool: ThreadPool) -> Counter {
        Counter {
            queue: actor(thread_pool, 0),
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
