# Discard

A messaging app that replicates Discord's features but operates entirely on a peer-to-peer (P2P) network. All data is transferred directly 
between users and stored locally on their devices, without relying on centralized servers.

## FYI
The project is still in very early stages of development.
The tui directory is a place holder for future front-end UIs. The Discard engine operates off of TCP messages allowing a variety
of front-ends to interact with the Discard API.

## Installation

To install this project, follow these steps:

```bash
# Clone the repository
git clone https://github.com/jhideki/discard.git

# Navigate to the project directory
cd discard/core

# Build or install the project
cargo build
# Or to run tests
cargo test
```


