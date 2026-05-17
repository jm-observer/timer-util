# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**timer-util** is a Rust library and server for flexible scheduling and timer computation. It provides:

1. **Core Library** (`timer-util`): A scheduling engine that computes next trigger times based on configurable days (weekday/month-day), hours, minutes, and seconds. Uses bitset-based configuration for efficient time-unit selection.

2. **Alarm Server** (`alarm-server`): A production-ready web service (Actix-web) and CLI that manages recurring/one-time alarms with callback execution, SQLite persistence, and a dashboard.

The project uses a **workspace structure** with two crates: the library and the alarm-server application.

## Build & Test Commands

### Core Library

```bash
# Build the library
cargo build

# Run all tests (52 tests covering builder, compute, conf modules)
cargo test --lib

# Run doc tests
cargo test --doc

# Check code without building
cargo check --all-targets

# Lint with clippy (strict warnings)
cargo clippy --all-targets -- -D warnings

# Format check
cargo fmt --all -- --check

# Format code
cargo fmt --all
```

### Alarm Server

```bash
# Build alarm-server and alarm-cli binaries
cargo build --manifest-path alarm-server/Cargo.toml

# Run the web server (listens on port from config/env)
cargo run --manifest-path alarm-server/Cargo.toml --bin alarm-server

# Run the CLI tool
cargo run --manifest-path alarm-server/Cargo.toml --bin alarm-cli -- [args]

# Build with production features
cargo build --release --features prod --manifest-path alarm-server/Cargo.toml
```

### Release Builds

The project publishes binaries for:
- Windows x86_64 (MSVC)
- Linux ARM64 (aarch64-unknown-linux-gnu)

GitHub Actions (`.github/workflows/release.yml`) automatically builds and creates GitHub releases on version tags.

## Architecture

### Core Library Architecture (`src/`)

**Key Modules:**

- **`data.rs`**: Time-unit enums (Hour, Minute, Second, WeekDay, MonthDay) with auto-derived traits via `define_time_enum!` macro. Implements `FromData`/`AsBizData` for type conversion.

- **`traits.rs`**: Core abstractions:
  - `ConfigOperator`: Bitset-based configuration for time-unit selection (e.g., Hours stores 24 bits for hours 0-23). Provides builder methods (`add`, `add_range`, `add_array`, `merge`, `intersection`).
  - `Computer`: Generic interface for computing next matching values. Implemented by `DayUnit` and `TimeUnit<T>`.
  - `FromData`/`AsBizData`/`TryFromData`: Type conversion traits.

- **`conf.rs`**: `TimerConf` struct and configuration containers:
  - `Hours`, `Minutes`, `Seconds`, `WeekDays`, `MonthDays`: Wrappers around bitsets that implement `ConfigOperator`.
  - `Days`: Enum supporting both `WeekDays` and `MonthDays` patterns.
  - `TimerConf::next()`: Core algorithm—iterates through days, hours, minutes, seconds to find next trigger time.
  - `TimerConf::datetimes(range)`: Returns all scheduled times in a range.
  - `TimerConf::from_cron(expr)`: Parses standard cron expressions (5 or 6 fields; month field must be `*`).

- **`compute.rs`**: Computing engines:
  - `TimeUnit<T>`: Generic time-unit computer for Hours/Minutes/Seconds.
  - `DayUnit`: Complex day computer handling month boundaries, leap years, and day-of-week/day-of-month unions.

- **`builder.rs`**: Fluent builder API:
  - `configure_weekday(WeekDays)` → `DayConfBuilder` → `DayHourConfBuilder` → `DayHourMinuteConfBuilder` → `TimerConf`
  - `configure_monthday(MonthDays)` → same chain
  - Chainable methods: `.build_with_hours()`, `.build_with_minute()`, `.build_with_second()`
  - Optional: `.conf_week_days()`, `.conf_month_days()` to add constraints mid-chain

- **`cron.rs`**: Cron expression parsing. Implements field parsing (lists `1,3,5`, ranges `9-17`, steps `*/15`) for each time unit.

- **`error.rs`**: Error types (`TimerError`) for invalid ranges, out-of-range values, and cron syntax issues.

- **`iter.rs`**: `TimerIter` trait for iteration support.

**Design Pattern:** The library uses a **composition-based scheduling engine** where `TimerConf` coordinates multiple `Computer` instances (DayUnit, TimeUnit for hours/minutes/seconds). Each computes the next matching value in its domain, and `TimerConf` orchestrates the cycle through days → hours → minutes → seconds → repeat.

### Alarm Server Architecture (`alarm-server/src/`)

**Key Modules:**

- **`main.rs`**: Server bootstrap. Initializes database, starts Tokio runtime, spawns scheduler task, and configures Actix-web routes.

- **`scheduler.rs`**: Background task that processes alarms. Receives commands via channel, computes next trigger time using timer-util, sleeps until trigger, and executes callbacks (HTTP requests).

- **`db.rs`**: SQLite persistence layer. Manages alarm CRUD, status updates, and queries. Recovers expired one-time alarms on startup.

- **`handlers.rs`**: Actix-web route handlers for alarm API (create, list, update, delete, trigger).

- **`models.rs`**: Alarm struct and serialization.

- **`callback.rs`**: HTTP callback execution with retry logic.

- **`dashboard.rs`**: Web UI endpoints.

- **`config.rs`**: Environment-based configuration (port, database path, etc.).

- **`bin/alarm-cli.rs`**: Command-line interface for managing alarms.

**Data Flow:**
1. API receives alarm request → stored in SQLite
2. Scheduler pulls active alarms → computes next trigger with timer-util
3. On trigger → HTTP callback to user's endpoint
4. Completion recorded in database

## Feature Flags

- **`serde`** (optional): Enables serialization for `TimerConf` and data enums. Required by alarm-server.
- **`prod`** (alarm-server only): Production features for custom-utils logging.

## Testing Strategy

- **Unit tests**: Embedded in each module (`#[cfg(test)]` blocks). Cover builder chains, compute logic, edge cases (leap years, month boundaries), cron parsing.
- **Doc tests**: In README and `lib.rs` examples.
- **Integration**: CI runs all tests on push/PR to main.

Test modules worth examining:
- `src/builder.rs::test`: Builder API contract validation
- `src/compute.rs::test`: Day/time unit computation correctness
- `src/conf.rs::test`: ConfigOperator trait implementations

## Dependencies Overview

**Core Library:**
- `chrono`: Date/time computation
- `log`: Logging interface
- `serde` (optional): Serialization

**Alarm Server:**
- `actix-web`: HTTP server framework
- `tokio`: Async runtime with multi-threading
- `rusqlite`: SQLite with bundled library
- `reqwest`: HTTP client for callbacks
- `clap`: CLI argument parsing
- `custom-utils`: Shared logging utilities

## CI/CD

**Workflows (`.github/workflows/`):**

- **`ci.yml`**: Runs on push/PR to main. Checks, tests, fmt, clippy.
- **`release.yml`**: Triggered on version tags. Builds x86_64 and ARM64 binaries, uploads to GitHub Releases.

## Development Notes

- **Bitset Optimization**: ConfigOperator uses `u64` bitsets for compact time-unit representation (supports 0-63 values). Suitable for hours (0-23), minutes (0-59), seconds (0-59), but day-of-month (1-31) and day-of-week (0-6) fit within limits.

- **Leap Year Handling**: DayUnit in compute.rs handles February 29 automatically via `NaiveDate` transitions.

- **Timezone**: Uses `Local` time via chrono; no explicit UTC handling in core library.

- **Cron Parsing Limitation**: Month field is not supported (`*` only); use weekday or day-of-month instead.

- **Performance**: Next trigger calculation is O(n) iterations through configured values per cycle, not O(1). Suitable for reasonable scheduling intervals.
