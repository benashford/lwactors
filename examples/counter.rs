extern crate futures;
extern crate futures_cpupool;
extern crate lwactors;

use std::thread;

use futures::{future, Future};

use futures_cpupool::CpuPool;

use lwactors::{actor, Action, ActorError, ActorSender};

fn main() {
    let cpu_pool = CpuPool::new_num_cpus();
    let counter = Counter::new(&cpu_pool);
    let counter1 = counter.clone();

    let a = thread::spawn(move || {
        let futs: Vec<_> = (0..100).map(|i| counter1.add(i)).collect();
        let results = future::join_all(futs).wait().expect("Futures failed 1");
        println!("ADD RESULTS: {:?}", results);
    });

    let counter2 = counter.clone();

    let b = thread::spawn(move || {
        let futs: Vec<_> = (0..100).map(|i| counter2.subtract(i)).collect();
        let results = future::join_all(futs).wait().expect("Futures failed 2");
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

type CounterFuture = Box<Future<Item = i64, Error = CounterError>>;

impl Counter {
    fn new(cpu_pool: &CpuPool) -> Counter {
        Counter {
            queue: actor(cpu_pool, 0),
        }
    }

    fn add(&self, n: i64) -> CounterFuture {
        self.queue.invoke(CounterAction::Add(n))
    }

    fn subtract(&self, n: i64) -> CounterFuture {
        self.queue.invoke(CounterAction::Subtract(n))
    }
}

#[derive(Debug)]
enum CounterAction {
    Add(i64),
    Subtract(i64),
}

impl Action<i64, i64, CounterError> for CounterAction {
    fn act(self, state: &mut i64) -> Result<i64, CounterError> {
        println!("Acting {:?} on {}", self, state);
        match self {
            CounterAction::Add(i) => *state += i,
            CounterAction::Subtract(i) => *state -= i,
        }
        return Ok(*state);
    }
}

#[derive(Debug)]
struct CounterError;

impl From<ActorError> for CounterError {
    fn from(_: ActorError) -> Self {
        CounterError
    }
}
