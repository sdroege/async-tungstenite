# async-tungstenite

Asynchronous WebSockets for [async-std](https://async.rs) and `std` `Future`s.

[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)
[![Crates.io](https://img.shields.io/crates/v/async-tungstenite.svg?maxAge=2592000)](https://crates.io/crates/async-tungstenite)
[![Build Status](https://travis-ci.org/sdroege/async-tungstenite.svg?branch=master)](https://travis-ci.org/sdroege/async-tungstenite)

[Documentation](https://docs.rs/async-tungstenite)

## Usage

Add this in your `Cargo.toml`:

```toml
[dependencies]
async-tungstenite = "*"
```

Take a look at the `examples/` directory for client and server examples. You may also want to get familiar with
[`async-std`](https://async.rs/) if you don't have any experience with it.

## What is async-tungstenite?

This crate is based on `tungstenite-rs` Rust WebSocket library and provides async-std bindings and wrappers for it, so you
can use it with non-blocking/asynchronous `TcpStream`s from and couple it together with other crates from the async-std stack.

## tokio-tungstenite

Originally this crate was created as a fork of [tokio-tungstenite](https://github.com/snapview/tokio-tungstenite)
and ported to [async-std](https://async.rs).
