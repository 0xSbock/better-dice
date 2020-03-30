# Better Dice
A simple **synchronized** dice roll service for online pen and paper players.

![Preview of the better dice interface](preview.png)
## Why?
Recently i played my first round of online pen and paper.
Most of the time the person rolling the dice shared the screen while rolling the dice to "proof" the result.
Hence i wanted to provide a better solution for my group. 

## How?

For a minimal Javascript footprint, [Server Sent Events (SSE)](https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events/Using_server-sent_events) are used to synchronize dice roll results.

Backend is written in Rust with [warp](https://docs.rs/warp/0.2.2/warp/).

## Getting Started
[Install rust](https://www.rust-lang.org/tools/install) and its tooling if you didn't already.
Clone the repository and run `cargo run --release`.

