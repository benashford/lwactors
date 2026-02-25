---
trigger: glob
globs: src/**/*.rs
---

## Rust 2024

This repo uses Rust 2024 edition. So should follow that wherever possible, e.g. `Future` is part of the standard prelude and doesn't need to be imported.

## Imports

Never use fully qualified names for items (structs, traits, functions, etc.). Instead import the item with `use` if the name of the item is unambiguous; if the name is ambiguous, import the module (e.g. `use tokio::sync::mpsc`, `let (tx, rs) = mpsc::channel(size)`, needed to disambiguate with `oneshot` channels). Exception: macros should always be referred to in-full.