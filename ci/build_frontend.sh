#!/bin/sh

set -x
set -e

PROJECT_ROOT=$(git rev-parse --show-toplevel)

# Generate databases
cd "$PROJECT_ROOT/windiff_cli"
cargo build --release
./target/release/windiff_cli "$PROJECT_ROOT/ci/db_configuration.json" "$PROJECT_ROOT/windiff_frontend/public/" --low-storage-mode

# Build the frontend
cd "$PROJECT_ROOT/windiff_frontend"
npm ci
npm run build

cd "$PROJECT_ROOT"
