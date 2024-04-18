#!/bin/sh

set -x
set -e

WINDIFF_CLI_URL=https://github.com/ergrelet/windiff/releases/download/v1.3.1/windiff_cli-x86_64-unknown-linux-musl
PROJECT_ROOT=$(git rev-parse --show-toplevel)

# Download a pre-built version of `windiff_cli`
wget -O windiff_cli ${WINDIFF_CLI_URL} && chmod +x windiff_cli

# Generate databases
./windiff_cli "$PROJECT_ROOT/ci/db_configuration.json" "$PROJECT_ROOT/windiff_frontend/public/" --low-storage-mode

# Build the frontend
cd "$PROJECT_ROOT/windiff_frontend"
npm ci
npm run build

cd "$PROJECT_ROOT"
