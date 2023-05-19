#!/bin/sh

set -x
set -e

PROJECT_ROOT=$(git rev-parse --show-toplevel)

# Generate databases
cd "$PROJECT_ROOT/windiff_cli"
cargo run --release "$PROJECT_ROOT/ci/db_configuration.json" "$PROJECT_ROOT/windiff_frontend/public/"

# Build the frontend
cd "$PROJECT_ROOT/windiff_frontend"
npm run build

cd "$PROJECT_ROOT"
