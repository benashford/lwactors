extern crate futures;
extern crate futures_cpupool;
extern crate lwactors;

use std::thread;

use futures::{future, Future, Sink};
use futures::sink::Send;
use futures::sync::mpsc;

use futures_cpupool::CpuPool;

use lwactors::{actor, Action};

fn main() {
    let cpu_pool = CpuPool::new_num_cpus();
    let counter = Counter::new(&cpu_pool);
    let counter1 = counter.clone();

    let a = thread::spawn(move || {
        let futs: Vec<_> = (0..100).map(|i| counter1.add(i)).collect();
        future::join_all(futs).wait().expect("Futures failed 1");
    });

    let counter2 = counter.clone();

    let b = thread::spawn(move || {
        let futs: Vec<_> = (0..100).map(|i| counter2.subtract(i)).collect();
        future::join_all(futs).wait().expect("Futures failed 2");
    });

    a.join().expect("Thread one failed");
    b.join().expect("Thread two failed");

    thread::sleep_ms(1000);

    println!("Finished: {:?}", cpu_pool);
}

#[derive(Clone)]
struct Counter {
    queue: mpsc::Sender<CounterAction>,
}

impl Counter {
    fn new(cpu_pool: &CpuPool) -> Counter {
        Counter {
            queue: actor(cpu_pool, 0),
        }
    }

    fn add(&self, n: i64) -> Send<mpsc::Sender<CounterAction>> {
        self.clone().queue.send(CounterAction::Add(n))
    }

    fn subtract(&self, n: i64) -> Send<mpsc::Sender<CounterAction>> {
        self.clone().queue.send(CounterAction::Subtract(n))
    }
}

#[derive(Debug)]
enum CounterAction {
    Add(i64),
    Subtract(i64),
}

impl Action<i64> for CounterAction {
    fn act(self, state: &mut i64) {
        println!("Acting {:?} on {}", self, state);
        match self {
            CounterAction::Add(i) => *state += i,
            CounterAction::Subtract(i) => *state -= i,
        }
    }
}
