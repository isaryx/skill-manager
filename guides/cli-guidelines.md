# Rust CLI Guidelines

General guidelines for building command-line tools in Rust. Sourced from the [Command Line Applications in Rust](https://rust-cli.github.io/book/) book, clap documentation, and common practice in tools like `ripgrep`, `fd`, `cargo`, and `gh`.

This document describes **how to build CLIs well**. It is not a product spec. Project-specific decisions (store layout, commands, domain model) belong in [docs/SPEC.md](../docs/SPEC.md) and [docs/DESIGN.md](../docs/DESIGN.md).

---

## Primary references

| Resource | URL |
|----------|-----|
| Command Line Applications in Rust | https://rust-cli.github.io/book/ |
| Rust Book — lib/bin split | https://doc.rust-lang.org/book/ch12-03-improving-error-handling-and-modularity.html |
| clap derive tutorial | https://docs.rs/clap/latest/clap/_derive/_tutorial/index.html |
| CLI book — testing | https://rust-cli.github.io/book/tutorial/testing.html |
| CLI book — output (stdout/stderr) | https://rust-cli.github.io/book/tutorial/output.html |
| CLI book — human communication | https://rust-cli.github.io/book/in-depth/human-communication.html |
| CLI book — machine communication | https://rust-cli.github.io/book/in-depth/machine-communication.html |
| Error handling patterns | https://andrewodendaal.com/rust-error-handling-patterns-production/ |
| NO_COLOR standard | https://no-color.org/ |
| The CLI Spec (emerging) | https://clispec.dev/ |

---

## Project structure

### lib + bin split (recommended, not required)

The Rust Book and CLI book recommend splitting binaries so logic is testable:

```
src/
├── lib.rs          # domain logic, public API
├── main.rs         # parse args → run() → map errors → exit
└── cli.rs          # optional: clap structs only
tests/
└── cli.rs          # integration tests (assert_cmd)
```

**Why:** `main()` cannot be unit-tested directly. Moving logic into `lib.rs` lets you test it without spawning a process.

**When to skip:** tiny one-off tools where the entire program fits comfortably in one file. Split when `main.rs` grows past trivial dispatch or you need integration tests against real behavior.

**Rules:**

- `main.rs` should read as dispatch only: parse → `run(args) -> Result<()>` → print error → exit.
- Do not call `std::process::exit` inside library code.
- Start as a **single crate**. Use a workspace when you have multiple binaries sharing substantial code or need to publish the library separately.

There is no single "correct" module tree inside `lib.rs`. Organize by domain, not by layer, unless layering helps clarity.

---

## Argument parsing (clap)

**Default choice:** clap with the **derive API**. The builder API is for dynamic commands or runtime-constructed parsers.

```rust
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "myapp", version, about)]
struct Cli {
    #[arg(short, long, env = "MYAPP_VERBOSE")]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Process a file
    Run {
        path: PathBuf,
        #[arg(long, default_value = "text")]
        format: OutputFormat,
    },
}

#[derive(Clone, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
}
```

**Conventions:**

- Model the CLI as structs and enums, not raw `std::env::args()`.
- Use `ValueEnum` (or `value_parser`) so invalid inputs fail at parse time with a useful message.
- Doc comments on fields become `--help` text — write them for users, not for developers.
- Use `#[arg(env = "...")]` for common overrides; document env vars in `--help`.
- Generate shell completions (`clap_complete`) and man pages (`clap_mangen` + `build.rs`) when you ship to others.

**Performance:** parse arguments before reading config files, opening databases, or doing network I/O. Heavy work belongs after clap returns.

**Complex CLIs:** when flags interact (mutual exclusion, path existence, regex validity), consider two stages:

1. **Parse stage** — clap fills a struct mirroring the CLI surface.
2. **Validate stage** — one function converts that struct into a typed `Config`, running all semantic checks in one place.

This avoids scattering validation across command handlers. Worth it once flag interactions become non-trivial; overkill for a handful of flags.

---

## stdout, stderr, and logging

| Stream | Purpose |
|--------|---------|
| **stdout** | Output the user or script asked for — results, JSON, pipeable data |
| **stderr** | Diagnostics, progress, warnings, errors |

This is long-standing Unix convention. Scripts pipe stdout; mixing log lines into stdout breaks them.

```rust
println!("{}", result);      // data
eprintln!("error: {e:#}");   // failure
```

**Logging:** `log` + `env_logger` is enough for most tools; `tracing` scales better when the tool grows or has async internals.

```bash
RUST_LOG=myapp=debug myapp run ./file
```

Typical levels: `info` for normal operation, `warn`/`error` for problems, `debug`/`trace` behind `--verbose` or `RUST_LOG`.

**Human vs machine output:**

- Use `std::io::IsTerminal` to adapt formatting (tables, colors) when stdout is a TTY.
- Do not rely on TTY detection alone for automation — offer explicit `--json` or `-o json`.
- Accept `-` as a path meaning stdin where file arguments make sense.

**High-volume output:** lock stdout once to avoid per-line mutex cost:

```rust
let stdout = std::io::stdout().lock();
writeln!(stdout, "{line}")?;
```

---

## Colors and terminal UX

Most modern tools follow this precedence:

1. Explicit flag: `--color always|never|auto` or `--no-color`
2. `NO_COLOR` — any non-empty value disables color ([no-color.org](https://no-color.org/))
3. `CLICOLOR_FORCE` — force color even when piped
4. `CLICOLOR=0` — disable color (BSD convention)
5. `is_terminal()` on the stream you are coloring

**Never colorize machine-readable output** (JSON, CSV, etc.).

Check stdout and stderr **separately**. Coloring stderr based on stdout's TTY status is a common bug when users redirect `2> errors.log`.

Libraries: `termcolor`, `owo-colors`, or `anstream` (respects NO_COLOR).

---

## Error handling

| Layer | Crate | Role |
|-------|-------|------|
| Library | `thiserror` | Typed error enums callers can match on |
| Binary | `anyhow` | Context chains for human-readable messages at the edge |

```rust
use std::path::PathBuf;

// lib.rs
#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("file not found: {0}")]
    NotFound(PathBuf),
    #[error("invalid config: {0}")]
    InvalidConfig(String),
}

// main.rs
fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}
```

**Rules:**

- No `.unwrap()` on production paths (tests are fine).
- Add `.context("…")` or `.with_context(|| …)` at every `?` crossing I/O, parsing, or network boundaries.
- Do not panic for user mistakes — return `Result`.
- Do not expose `anyhow::Error` from a library's public API.
- Consider `human-panic` so unexpected panics do not dump stack traces on end users.
- `miette` is optional; useful when errors reference file locations or need rich formatting.

---

## Exit codes

Exit codes are part of the public contract when scripts wrap your tool.

| Code | Common meaning |
|------|----------------|
| 0 | Success |
| 1 | General failure |
| 2 | Misuse / invalid usage (clap often exits 2 before your code runs) |
| 64–78 | Domain-specific (BSD `sysexits.h`; `exitcode` crate provides constants) |

**Be honest:** there is no universal Rust standard beyond "0 = ok, non-zero = failed." Document what your tool uses. Tools like `grep` and `diff` use exit 1 for "found a difference" or "no match" — that is intentional, not an error.

Map errors to codes in `main`, not deep in library functions. In verbose mode, print the full cause chain (`{:#}` or `.chain()`).

[The CLI Spec](https://clispec.dev/) proposes structured JSON errors on stderr and declared codes per error kind. That is a worthwhile target for tools with heavy automation; not mandatory for every CLI.

---

## Sync vs async

**Default: synchronous code.** Most CLIs run one task and exit. Async adds a runtime, debugging complexity, and dependency weight without benefit unless you need concurrent I/O.

| Situation | Reasonable choice |
|-----------|-------------------|
| Sequential file/network work | Sync (`std::fs`, `reqwest::blocking`) |
| Many concurrent network calls | Tokio + async client |
| CPU-bound parallelism | `rayon` or `std::thread` |
| Async I/O plus CPU work | Tokio for I/O; `spawn_blocking` / `rayon` for CPU — keep pools separate |

Decision rule: waiting on many things at once → async; computing on many cores → `rayon`; otherwise → sync.

Do not make everything async "for consistency." Async in Rust is an optimization for I/O concurrency, not a default architecture.

---

## Testing

### Unit tests

Test logic in `lib.rs` directly. Prefer pure functions. Inject `impl Write` instead of hardcoding `println!` when testing output formatting.

### Integration tests

Use `assert_cmd` + `predicates` to spawn the compiled binary and assert on exit code, stdout, and stderr. Use `assert_fs` or `tempfile` for filesystem setup.

```toml
[dev-dependencies]
assert_cmd = "2"
predicates = "3"
assert_fs = "1"
tempfile = "3"
```

```rust
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn rejects_missing_file() {
    Command::cargo_bin("myapp")
        .unwrap()
        .args(["run", "/nonexistent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}
```

**Scope:**

- **Integration tests:** observable behavior at the process boundary — not every edge case.
- **Unit tests:** edge cases, parsing, algorithms.
- Test exit codes when scripts depend on them.
- Do not snapshot entire `--help` output (clap generates it); assert that important subcommands or flags appear if needed.
- `insta` snapshots help for stable human-readable output that rarely changes.
- `proptest` helps for idempotent or deterministic operations (rebuild index, regenerate lockfile).

---

## Configuration

If your tool reads config, pick a layering model and document it. A common pattern (lowest → highest priority):

1. Built-in defaults
2. Global config file (often `~/.config/<app>/config.toml` via the `directories` crate)
3. Project-local config (e.g. `./.myapprc` or `./myapp.toml`)
4. Environment variables
5. CLI flags

Validate once when loading config. Pass typed structs to the rest of the program; do not re-parse or re-validate in every handler.

**Tradeoffs:**

- XDG config (`~/.config/`) vs dotdir in `$HOME` — both are common; pick one and stay consistent.
- `figment` helps when merging multiple sources; hand-rolling with `serde` is fine for simple tools.
- Config file format (TOML, YAML, JSON) matters less than clear precedence rules and good error messages on parse failure.

---

## UX principles

1. **Sensible defaults** — the tool should do something useful with zero config.
2. **Non-interactive by default** — do not block on stdin without a TTY. Use flags; add prompts only when explicitly requested (`--interactive`).
3. **Good `--help`** — clap derive + doc comments; users should not need the README for basic usage.
4. **Actionable errors** — what failed, why, and what to try next.
5. **Idempotent where possible** — sync/install commands safe to re-run.
6. **`--dry-run`** — for destructive or mutating operations.
7. **`--verbose`** — debug detail on stderr; do not change stdout format when verbose.
8. **Stable machine output** — if scripts parse your output, treat the format as an API; document and version it.
9. **Shell completions** — low effort via `clap_complete`; high value for users.
10. **Fast startup** — defer work until after parsing.
11. **Signals** — long-running commands should handle `Ctrl+C` (`ctrlc` or `signal-hook`).

---

## Distribution

- **Release profile:** `strip = true`, `lto = true`, `codegen-units = 1` for smaller binaries.
- **Cross-compile** in CI for platforms you support (macOS arm64/x64, Linux amd64, etc.).
- **Checksums** on release artifacts.
- **Shell completions** and **man pages** bundled or documented.
- **Semantic versioning** and a changelog.
- **macOS:** sign and notarize if distributing binaries outside `cargo install`.

```toml
[profile.release]
strip = true
lto = true
codegen-units = 1
```

Install paths vary: `cargo install`, package managers (Homebrew, apt), or GitHub Releases. Pick what your audience uses.

---

## Common dependencies

A typical starting set — add only what you need:

```toml
[dependencies]
clap = { version = "4", features = ["derive", "env"] }
anyhow = "1"
thiserror = "1"
serde = { version = "1", features = ["derive"] }
log = "0.4"
env_logger = "0.11"
directories = "5"    # XDG config/data paths

[dev-dependencies]
assert_cmd = "2"
predicates = "3"
assert_fs = "1"
tempfile = "3"
```

Add as needed: `toml`/`serde_json` (config), `indicatif` (progress), `owo-colors`/`anstream` (color), `clap_complete`, `clap_mangen`, `reqwest`/`ureq` (HTTP), `rayon` (CPU parallelism), `tokio` (async I/O).

Avoid pulling in heavy stacks (async runtime, DB driver, git bindings) until the tool actually needs them.

---

## Code style checklist

- [ ] `Result<T, E>` in library code; no panics for expected failures
- [ ] Thin `main`; domain logic in `lib.rs`
- [ ] Errors get context at I/O and parsing boundaries
- [ ] Exit only in `main`
- [ ] stdout for data, stderr for diagnostics
- [ ] NO_COLOR respected when using color
- [ ] CI runs `cargo test`, `cargo clippy`, `cargo fmt --check`
- [ ] `#![forbid(unsafe_code)]` unless there is a documented reason

---

## What this document does not prescribe

- Specific command names, subcommands, or flags
- Store layout, database choice, or symlink-vs-copy policy
- Whether to use global profiles, project manifests, or lockfiles
- Which agents or platforms to support

Those are product decisions. Apply the guidelines above when implementing whatever design you choose.
