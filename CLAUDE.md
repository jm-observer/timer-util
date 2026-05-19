# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**timer-util** is a Rust workspace with two crates:

1. **Core Library** (`timer-util`): A scheduling engine that computes next trigger times based on configurable days (weekday/month-day), hours, minutes, and seconds. Uses `u64` bitset-based configuration for efficient time-unit selection.

2. **Alarm Server** (`alarm-server`): A production-ready web service (Actix-web) and CLI that manages recurring/one-time alarms with HTTP callback execution, SQLite persistence, and a dashboard.

Both crates use Rust **edition = "2024** (enabling `let_chains` on stable).

## Build & Test Commands

### Core Library

```bash
# Build the library
cargo build

# Run all tests
cargo test --lib

# Run a single test (module::test_name pattern)
cargo test --lib conf::test::my_test_name

# Run doc tests
cargo test --doc

# Check, lint, format
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo fmt --all
cargo fmt --all -- --check   # check only
```

### Alarm Server

```bash
# Build both binaries
cargo build --manifest-path alarm-server/Cargo.toml

# Run the web server
cargo run --manifest-path alarm-server/Cargo.toml --bin alarm-server

# Run with custom workspace directory
cargo run --manifest-path alarm-server/Cargo.toml --bin alarm-server -- serve -w /path/to/workspace

# Run the CLI tool
cargo run --manifest-path alarm-server/Cargo.toml --bin alarm-cli -- [args]

# Production build (enables custom-utils prod logging)
cargo build --release --features prod --manifest-path alarm-server/Cargo.toml
```

### Cargo Make shortcuts (requires `cargo-make`)

```bash
cargo make check   # fmt + clippy + tests across workspace
cargo make prod    # release build with prod feature
```

### Release Builds

GitHub Actions (`.github/workflows/release.yml`) builds on version tags for:
- Windows x86_64 (MSVC)
- Linux ARM64 (aarch64-unknown-linux-gnu)

## Architecture

### Core Library (`src/`)

**Design Pattern:** `TimerConf` coordinates multiple `Computer` instances. Each computes the next matching value in its domain; `TimerConf` orchestrates the cycle: days → hours → minutes → seconds → repeat.

**Key types:**

- **`traits.rs`** — `ConfigOperator` (bitset config: `add`, `add_range`, `merge`, `intersection`), `Computer` (next-value interface), type conversion traits.

- **`conf.rs`** — `TimerConf` is the top-level scheduling struct. `Hours`/`Minutes`/`Seconds`/`WeekDays`/`MonthDays` wrap bitsets implementing `ConfigOperator`. `Days` enum supports either `WeekDays` or `MonthDays`. Key methods:
  - `TimerConf::next()` / `next_with_time(dt)` — find next trigger after now/given time
  - `TimerConf::datetimes(range)` — all scheduled times in a range
  - `TimerConf::from_cron(expr)` — parse cron (5 or 6 fields; month must be `*`)

- **`builder.rs`** — Fluent builder: `configure_weekday(WeekDays)` or `configure_monthday(MonthDays)` → `DayConfBuilder` → `DayHourConfBuilder` → `DayHourMinuteConfBuilder` → `TimerConf`.

- **`compute.rs`** — `TimeUnit<T>` (generic for hours/minutes/seconds), `DayUnit` (handles month boundaries, leap years, weekday/monthday union).

- **`data.rs`** — Time-unit enums (`Hour`, `Minute`, `Second`, `WeekDay`, `MonthDay`) auto-derived via `define_time_enum!` macro.

- **`cron.rs`** — Cron field parsing: lists (`1,3,5`), ranges (`9-17`), steps (`*/15`).

### Alarm Server (`alarm-server/src/`)

**Data Flow:**
1. API request → stored in SQLite with status `active`
2. Scheduler loop wakes up, computes next trigger per alarm, sleeps until nearest (max 1 hour)
3. On trigger → HTTP callback (`callback.rs` with retry); one-time alarms marked `completed`

**Key modules:**

- **`scheduler.rs`** — Single async loop receiving `SchedulerCommand::Reload`/`Shutdown` via mpsc channel. Each iteration: load active alarms, cancel orphaned callbacks, compute fire times, `tokio::select!` between sleep and command. Each alarm callback runs in its own `tokio::spawn` with a `CancellationToken`.

- **`db.rs`** — SQLite via `rusqlite`. Alarm types: `"cron"` (field `cron_expr`) and `"once"` (field `once_at`, format `%Y-%m-%dT%H:%M:%S`). Startup recovery marks expired one-time alarms as `completed`.

- **`config.rs`** — Loads `<workspace>/config.toml`. Default workspace: `~/.config/alarm-server`. Configurable field: `port` (default `8080`). DB lives alongside config as `alarms.db`.

- **`main.rs`** — Subcommands: `serve` (default), `install` (Linux systemd, requires root), `update` (self-update from GitHub releases).

## Feature Flags

- **`serde`** (library, optional): Serialization for `TimerConf` and data enums. Required by alarm-server.
- **`prod`** (alarm-server only): Enables production logging via `custom-utils`.

## Testing Strategy

Tests are `#[cfg(test)]` blocks embedded in each module. Key modules to check:
- `src/builder.rs` — Builder API contract
- `src/compute.rs` — Day/time unit computation, leap years, month boundaries
- `src/conf.rs` — `ConfigOperator` trait implementations, cron parsing
