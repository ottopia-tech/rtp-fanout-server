#!/bin/bash
set -e

echo "Building RTP Fanout Server..."

cargo build --release

echo "Running tests..."
cargo test

echo "Building Docker image..."
docker build -t ottopia-tech/rtp-fanout-server:latest -f build/Dockerfile .

echo "Build complete!"
