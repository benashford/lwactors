# lwactors

[![Build Status](https://github.com/benashford/lwactors/workflows/Rust/badge.svg)](https://github.com/benashford/lwactors/actions)
[![](http://meritbadge.herokuapp.com/lwactors)](https://crates.io/crates/lwactors)
[![](https://img.shields.io/crates/d/lwactors.svg)](https://crates.io/crates/lwactors)
[![](https://img.shields.io/crates/dv/lwactors.svg)](https://crates.io/crates/lwactors)
[![](https://docs.rs/lwactors/badge.svg)](https://docs.rs/lwactors/)

Lightweight actors for Rust using `futures-rs` and `tokio`.

## Introduction

The TL;DR is that you probably want to use [Actix](https://github.com/actix/actix) or [Tokio](https://tokio.rs) directly instead.

This library allows standalone "actors" to exist in Applications using Futures.

An actor is created with an initial state. From there the state can only be queried or modified by passing messages using the `invoke` function, receiving a future which will contain a result once the message has been processed. The messages are processed asynchronously.

See the ["counter" example](examples/counter.rs) or the ["ping_pong" example](examples/ping_pong.rs) for suggestions of how to build actors around this library.

## Overview

### The actor itself

An application would contain a bespoke struct for each actor, which would own a reference to the underlying future:

```rust
#[derive(Clone)]
struct Counter {
    queue: ActorSender<CounterAction, i64, CounterError>,
}
```

When constructed, the `lwactors::actor` function is called to start the actor running (this requires an existing Tokio runtime):

```rust
fn new() -> Counter {
    Counter {
        queue: lwactors::actor(0),
    }
}
```

Alternatively, `actor_with_executor` can be used to provide a specific executor (like a `tokio::runtime::Handle`).

### Action - the set of possible messages

The application will then define the set of messages that can be sent to the actor:

```rust
#[derive(Debug)]
enum CounterAction {
    Add(i64),
    Subtract(i64),
}
```

and the code that the actor will run when processing the message by implementing the `Action` trait:

```rust
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
```

### Usage

Finally, to expose a friendly high-level API, the application can implement specific functions which are responsible for sending messages to the actor:

```rust
impl Counter {
    // new function quoted above goes here

    async fn add(&self, n: i64) -> Result<i64, CounterError> {
        self.queue.invoke(CounterAction::Add(n)).await
    }

    async fn subtract(&self, n: i64) -> Result<i64, CounterError> {
        self.queue.invoke(CounterAction::Subtract(n)).await
    }
}
```

The struct that contains the reference to the actor `Counter` is cheaply cloneable, but all refer to the same running actor that processes the messages, so all indirectly refer to the same shared state. Each task that owns a `Counter` can call `add` or `subtract` to send messages, `await`ing a result which is the state of the counter after that message has been processed.

The running actor that processes messages will complete successfully once all the `Counter`s have been dropped.
