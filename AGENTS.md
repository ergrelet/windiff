# AGENTS.md

This file provides guidance to coding agents working with code in this repository. (`CLAUDE.md` is a symlink to this file, so Claude Code and any AGENTS.md-aware tool read the same instructions.)

## Project Overview

**WinDiff** is a web-based tool for browsing and comparing symbol, type, and syscall information across different versions of Microsoft Windows binaries. It has two components:

- **windiff_cli** — Rust CLI that generates compressed JSON databases by downloading PE binaries from Winbindex and extracting symbol/type/syscall data from matching PDBs.
- **windiff_frontend** — Next.js/React static site that loads those databases and provides Browse/Diff UI.

## Commands

### windiff_cli (Rust)

```bash
cd windiff_cli

# Build
cargo build --release

# Run (generates databases from config into output dir)
cargo run --release -- ../ci/db_configuration.json ../windiff_frontend/public/
# With low-storage mode (processes binaries sequentially, lower memory use)
cargo run --release -- --low-storage-mode ../ci/db_configuration.json ../windiff_frontend/public/
# Throttle concurrent downloads (default 64) for constrained environments
cargo run --release -- --concurrent-downloads 16 ../ci/db_configuration.json ../windiff_frontend/public/

# Lint & format
cargo fmt --check
cargo clippy

# Tests
cargo test
# Single test
cargo test <test_name>
```

### windiff_frontend (Node.js/TypeScript)

```bash
cd windiff_frontend

npm ci               # Clean install
npm run dev          # Dev server at http://localhost:3000
npm run build        # Production build
npm run lint         # ESLint check
```

### Full build (both components)

```bash
./ci/build_frontend.sh
```

### Local/dev files

The repo's root `local/` folder is git-ignored — keep machine-local, never-committed
files there (scratch data, experiment output, and trimmed-down test configs). For
example, a small DB config for quick CLI runs lives at `local/db_configuration_mini.json`:

```bash
cd windiff_cli
cargo run --release -- ../local/db_configuration_mini.json ../windiff_frontend/public/
```

Do not commit anything under `local/`, and do not reference it from CI or committed code.

## Architecture

### Data generation (windiff_cli)

The CLI takes a JSON config (`ci/db_configuration.json`) that specifies Windows OS versions and binaries to track, then:

1. Fetches PE binary indexes from Winbindex
2. Downloads PEs and matching PDBs (from MSDL symbol server)
3. Extracts exported symbols, debug symbols, modules, reconstructed types (via `resym_core`), and syscalls
4. Writes gzip-compressed JSON databases to the output directory plus an index file

Key source files in `windiff_cli/src/`:
- `main.rs` — orchestrates normal vs. low-storage processing modes
- `cli.rs` — command-line argument parsing (structopt)
- `configuration.rs` — JSON config schema (OS versions, binaries, extraction flags)
- `winbindex.rs` — Winbindex API integration
- `pdb.rs` — PDB parsing and symbol/module extraction
- `resym_frontend.rs` — wrapper around `resym_core` for type reconstruction
- `syscalls.rs` — syscall extraction from ntdll, win32u, ntoskrnl, win32k
- `database.rs` — database serialization, gzip compression, index creation
- `download.rs` — concurrent PE/PDB download logic
- `error.rs` — crate-wide error types

### Frontend (windiff_frontend)

A Next.js app that fetches and decompresses the pre-generated databases in the browser. Key files in `windiff_frontend/src/app/`:
- `data_explorer.tsx` — core data UI: OS/binary selection, tab navigation, diff logic
- `windiff_types.ts` — TypeScript interfaces for index and database file schemas
- `page.tsx` — top-level routing between Browse and Diff modes

The frontend never generates data at runtime; it only reads the static JSON databases placed in `windiff_frontend/public/` by `windiff_cli`.

### CI/CD

GitHub Actions workflows live in `.github/workflows/`:

- **Push (`rust.yml`):** runs `cargo fmt --check` + `cargo clippy` (Rust)
- **Push (`typescript.yml`):** runs `npm run lint` + `npm run build` (TypeScript)
- **Git tag `v*` (`release.yml`):** builds release binaries for `x86_64-unknown-linux-musl` and `x86_64-pc-windows-msvc`
- **Daily cron (`scheduled.yml`):** runs `ci/fetch_update.py` to sync `ci/db_configuration.json` with the latest Winbindex data

## Agent Skill: version diff analysis

This repo bundles a Claude Code skill at `.claude/skills/windiff-version-diff-analysis/`
for security-research diffing of two Windows versions. **Prefer it over ad-hoc
analysis** whenever the user wants to compare Windows builds — e.g. "diff ntoskrnl
between 21H2 and 24H2", "what new syscalls/mitigations/ETW providers appeared",
"what changed in win32k.sys / ci.dll", or "what's new that matters for EDR /
anti-cheat / vulnerability research". The skill auto-triggers on these, but if it
hasn't, read its `SKILL.md` and follow it rather than reinventing the workflow.

What it provides:

- `SKILL.md` — 5-step workflow: scope versions/binaries → generate databases with
  `windiff_cli` → diff → interpret with Windows-internals knowledge → write an
  audience-tagged report (EDR / anti-cheat / vulnerability research).
- `scripts/make_config.py` — builds a minimal `windiff_cli` config for just the two
  versions/binaries being compared (avoids regenerating the whole live config).
- `scripts/windiff_diff.py` — deterministic diff of one binary across two OS
  suffixes (`version_update_architecture`). Emits a stderr summary and JSON on
  stdout: added/removed exports, symbols, modules, syscalls (with renumbering
  detection), and types. It hides anonymous-type id churn as noise but resolves
  real anonymous-bitfield changes back to `<parent>::<member>` (e.g. new
  `_EPROCESS::MitigationFlags2Values` bits) under `types.resolved_member_changes`.
- `references/` — `windows-internals.md` (API prefixes, mitigation/feature/telemetry
  reasoning), `windows-components.md` (per-binary roles), `report-template.md`.

Use `scripts/windiff_diff.py --list <db_dir>` to discover available OS suffixes.
When databases already exist (e.g. in `windiff_frontend/public/`), point the diff
script there instead of regenerating. Skill artifacts live under
`.claude/skills/...`; iteration/eval scratch output belongs in the git-ignored
`local/` folder, not committed.

## External Data Sources

- **Winbindex** (`https://github.com/m417z/winbindex`) — binary index and download infrastructure
- **Microsoft MSDL** (`https://msdl.microsoft.com/download/symbols/`) — PDB symbol server
- **resym** (`https://github.com/ergrelet/resym`) — type/symbol reconstruction (used as a library crate)
