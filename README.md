# lwactors

[![Build Status](https://travis-ci.org/benashford/lwactors.svg?branch=master)](https://travis-ci.org/benashford/lwactors)
[![](http://meritbadge.herokuapp.com/lwactors)](https://crates.io/crates/lwactors)
[![](https://img.shields.io/crates/d/lwactors.svg)](https://crates.io/crates/lwactors)
[![](https://img.shields.io/crates/dv/lwactors.svg)](https://crates.io/crates/lwactors)
[![](https://docs.rs/lwactors/badge.svg)](https://docs.rs/lwactors/)

Lightweight actors for Rust using `futures-rs`.

## Introduction

The TL;DR is that you probably want to use [Actix](https://github.com/actix/actix) instead.

This library allows standalone "actors" to exist in Applications using Futures.

An actor is created with an initial state. From there the state can only be queried or modified by passing messages using the `invoke` function, receiving a future which will contain a result once the message has been processed. The messages are processed asynchronously.

See the ["counter" example](examples/counter.rs) for an suggestion of how to build actors around this library.

## Overview

### The actor itself

An application would contain a bespoke struct for each actor, which would own a reference to the underlying future:

```rust
#[derive(Clone)]
struct Counter {
    queue: ActorSender<CounterAction, i64, CounterError>,
}
```

When constructed `lwactors` `actor` function is called to start the actor running:

```rust
fn new(cpu_pool: &CpuPool) -> Counter {
    Counter {
        queue: actor(cpu_pool, 0),
    }
}
```

it requires a reference to a `CpuPool` (or another `Executor`) and an initial state, in this case `0`.

### Action - the set of possible messages

The application will then define the set of messages that can be sent to the actor:

```rust
#[derive(Debug)]
enum CounterAction {
    Add(i64),
    Subtract(i64),
}
```

and the code that the actor will run when processing the message:

```rust
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
```

### Usage

Finally, to expose a friendly high-level API, the application can implement specific functions which are responsible for sending messages to the actor:

```rust
impl Counter {
    // new function quoted above goes here

    fn add(&self, n: i64) -> CounterFuture {
        self.queue.invoke(CounterAction::Add(n))
    }

    fn subtract(&self, n: i64) -> CounterFuture {
        self.queue.invoke(CounterAction::Subtract(n))
    }
}
```

The struct that contains the reference to the actor `Counter` is cheaply cloneable, but all refer to the same running future that processes the messages, so all indirectly refer to the same shared state. Each thread that owns a `Counter` can call `add` or `subtract` to send messages, receiving a future that resolves to an `i64` which is the state of the counter after that message has been processed.

The running future that processes messages will complete successfully once all the `Counter`s have been dropped.
