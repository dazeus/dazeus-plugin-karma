#!/usr/bin/env bash

# Create target dir
echo ">> Creating build dir..."
mkdir -p target/grammar

# Clone rust-peg git repo or update
echo ">> Updating rust-peg repository..."
if [ -d "target/grammar/rust-peg" ]; then
  (cd target/grammar/rust-peg && git pull)
else
  git clone https://github.com/kevinmehall/rust-peg target/grammar/rust-peg
fi

# Build rust-peg binary
echo ">> Build rust-peg..."
(cd target/grammar/rust-peg && cargo build)

# Build grammar
echo ">> Build grammar..."
(cd target/grammar/rust-peg && cargo run -- ../../../src/grammar.rustpeg | > ../../../src/grammar.rs)
# target/grammar/rust-peg/target/release/peg src/grammar.rustpeg > src/grammar.rs

echo ">> Done"
