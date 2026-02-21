#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

echo "==> Building frontend (WASM)..."
cd crates/todo-frontend
trunk build --release
cd ../..

echo "==> Building server..."
cargo build --release -p todo-server

echo ""
echo "Build complete!"
echo "  Frontend: frontend-dist/"
echo "  Server:   target/release/todo-server"
echo ""
echo "Run with:"
echo "  ./target/release/todo-server"
echo "  Then open http://localhost:8080"
