# Design

**CLI:** `skm` · **Version:** 0.2.2

Contributor-facing architecture. User-visible behavior lives in [SPEC.md](SPEC.md). CLI conventions: [../guides/cli-guidelines.md](../guides/cli-guidelines.md).

---

## Domain model

| Concept | Definition |
|---------|------------|
| **Store** | Root directory (`~/.skill-store` default). Skills + `.skm/` metadata. |
| **Library** | All skill dirs under `$STORE/` (flat or nested). |
| **Skill ID** | Store-relative path to a skill dir (`docx`, `engineering/tdd`). |
| **Bundle** | Skill tree rooted at one store prefix; one meta file at bundle root (`local.toml` covers `local/*`). |
| **Profile** | Named skill selection in `.skm/profiles/<name>.toml`. |
| **Enabled skill** | Not listed in `.skm/disabled.toml`. |
| **Target** | Agent skills directory (adapter-specific). |
| **Placement** | One symlink: flat `name` in target → canonical store path. |

---

## End-to-end flow

```
skm import / cp+scan
        │
        ▼
     STORE  ── discover_skill_ids() ──▶  index.db (cache)
        │
        │  active profile + disabled set
        ▼
    resolver::resolve()  ──▶  Vec<SkillPlacement>
        │
        ▼
    sync::reconcile()  ──▶  agent target dir (symlinks)
```

**`reconcile()`** is the only code path that creates or removes agent symlinks. **`resolver`** has no I/O (unit-tested in isolation).

**Ownership rule:** a symlink is `skm`-owned iff its target resolves inside the canonical store root. Hand-installed or project-bundled skills in the agent dir are never deleted or overwritten. A top-level entry that is not store-owned is a **foreign occupant**; reconcile skips wiring that name and reports it as a conflict in `status` and `doctor` (`link.conflict`). Shared logic: `sync/links.rs` (`is_foreign_occupant`, `walk_store_owned_symlinks`, `is_store_owned_symlink`).

---

## On-disk layout

```
~/.skill-store/
├── .skm/
│   ├── profiles/work.toml
│   ├── meta/
│   │   ├── docx.toml              # single-skill provenance
│   │   └── local.toml             # bundle meta for local/*
│   ├── disabled.toml              # optional
│   └── index.db                   # rebuildable SQLite cache
├── docx/SKILL.md
└── local/foo/SKILL.md             # skill id: local/foo
```

- Tool metadata only under `.skm/`. Top-level store names must not be `.skm` or start with `.`.
- **`skm import`** writes meta (`source_type` `local` / `local-bundle`, `transfer` `copy`/`move`).
- **`refresh_store_index`** (scan/sync) adopts skills without meta: create-if-missing only, `source_type = "store"`, `transfer = "adopted"`. Never overwrites import meta.

Meta owner for nested id `local/foo` → `local` (`store::meta_owner_id`). `has_skill_meta` accepts per-skill or bundle-level meta.

---

## Configuration

| Layer | Path | Contents |
|-------|------|----------|
| App | `~/.config/skm/config.toml` | `[store].path` |
| Setup | `./.skm.toml` or `~/.skm.toml` | `placement.agent`, `[profile].active` |
| Profile | `$STORE/.skm/profiles/*.toml` | `[[skill]] id = "…"` |

Store path resolution: `--store` → `SKM_STORE` → app config → `~/.skill-store`.

### Setup file selection

| API | Validates agent | Used by |
|-----|-----------------|---------|
| `select_setup` | yes (`read_setup`) | sync, use-profile, status |
| `select_setup_lenient` | no (`read_setup_raw`) | doctor (report `unknown_agent` instead of failing early) |
| `select_project_setup` | yes | status without `--user` (requires `./.skm.toml`) |

`--user` / `-u` always loads `~/.skm.toml`. Default: `./.skm.toml` if present, else user setup.

Agents: [SPEC-AGENTS.md](SPEC-AGENTS.md).

---

## Placement naming

Agent skill dirs are **flat**. `resolver::assign_placement_names`:

- Unique leaf → leaf name (`engineering/tdd` → `tdd`)
- Colliding leaves → `__` between segments (`a/tdd` + `b/tdd` → `a__tdd`, `b__tdd`)
- Duplicate store IDs in one profile → error
- Disabled skills filtered before naming and existence check; empty result after filter is allowed in resolver (`use-profile` still rejects a profile with zero skill entries)

---

## Implementation

### Entry point

```
main.rs  →  lib::run(Cli)  →  cli/<command>.rs handlers
```

`SkmError` in `error.rs` (`thiserror`). `main` maps to exit codes via `exit_code_from_error`. `doctor` returns `i32` from `Report::exit_code`.

### Module layers

```
cli/          Command handlers, clap (cli/mod.rs), JSON (cli/output.rs)
setup.rs      Setup file selection, active profile writes
store/        StorePaths, discovery, profiles, pool import, disabled, validate
resolver/     Pure profile → SkillPlacement
sync/         reconcile(), symlink walk/apply (sync/links.rs)
doctor/       Read-only checks → Issue list
db/           SQLite index; refresh_store_index = adopt + rebuild
adapters/     AgentAdapter trait, agent pickers
tui/          Reusable full-screen widgets (MultiSelect)
config/       App config + SetupFile / ProfileFile types
util/         SKILL.md discovery, hashing, validation
progress.rs   stderr steps; colored +/- on TTY
```

### Interactive TUI (`src/tui/`)

`tui::MultiSelect` is the searchable checkbox list behind `profile setup` and `skill setup`
(keys and modes: [SPEC.md](SPEC.md#interactive-picker)). Built on `console` (already a
`dialoguer` dependency) rather than a TUI framework.

```rust
let keys = MultiSelect::new("Skills for profile `work`")
    .items(pool.iter().map(|id| MultiSelectItem::new(id).selected(is_selected(id))))
    .interact()?;   // Err(SelectionCancelled) on q / Esc / Ctrl-C
```

Items carry a stable `key` (returned on confirm), an optional dim `note`, and their initial
checked state. Selection lives on the items, so filtering can never disturb it.

Four things worth knowing before extending it:

- **`Screen` owns the alternate screen.** `Drop` restores the terminal, so an error return or a
  panic inside the event loop cannot strand the user in the alternate buffer.
- **`read_key_raw`, not `read_key`.** The latter raises `SIGINT` on `Ctrl-C`, which would skip
  that teardown. `Ctrl-C` arrives as `Key::CtrlC` and cancels like `q`.
- **State is pure.** `State::handle` and `State::render` are unit-tested without a TTY; only
  `MultiSelect::interact` touches the terminal. Keep new behavior on that side of the line.
- **Drawn text is sanitized, returned text is not.** `tui::sanitize` replaces control characters
  with `U+FFFD` in the title, item `display` and `note` at construction, so caller text cannot
  move the cursor or clear the screen. `key` is kept verbatim — it is what gets written to the
  profile file. Anything new that draws caller text goes through `sanitize`; anything that
  returns it must not.

`render` returns exactly one string per screen row and never emits a trailing newline: writing
past the last row would scroll the alternate screen and desynchronize the next repaint from the
top-left origin `Screen::draw` assumes. It sheds the title and spacer rows in short terminals to
keep the search field, one item row, the status line, and the hint bar.

### `reconcile_with_setup` pipeline

1. Load active profile (or override)
2. `resolver::resolve(profile, store, disabled)`
3. If `--dry-run`: `compute_link_changes` → print `(dry-run) +/-` on stderr; return
4. `ensure_store_subdirs`
5. `db::refresh_store_index`
6. Resolve agent target; create target dir
7. `remove_dangling_store_symlinks` → `clean_target` → `prune_empty_skill_dirs` → `apply_placements` (skips foreign occupants; does not fail reconcile)

Human output uses `src/color.rs` (`--color`, `NO_COLOR`, `CLICOLOR`). JSON output is never colored.

**Platform:** symlinks are Unix-only today (`#[cfg(unix)]` in `apply_placements`). Non-Unix returns a usage error.

### Key types

| Type | Module | Role |
|------|--------|------|
| `StorePaths` | `store` | All paths under `$STORE` |
| `SelectedSetup` | `setup` | Loaded `.skm.toml` + project/user level |
| `ProfileFile` | `config` | `[[skill]]` entries |
| `SkillPlacement` | `resolver` | `store_id`, flat `name`, canonical `source` |
| `Issue` / `Report` | `doctor` | Health findings + JSON shape |

### Doctor

Read-only. Runs store → index → disk skills → meta → profiles → config → links (if active profile resolves). Uses `select_setup_lenient` so invalid agents surface as `config.unknown_agent`. Link checks reuse store-ownership rules from `sync/links.rs`. Does not call `reconcile` or mutate disk.

---

## Errors & I/O

| Code | Typical cause |
|------|----------------|
| 0 | Success; doctor with info-only issues |
| 1 | I/O, store, placement, doctor warn/error |
| 2 | Clap usage, resolve conflict, duplicate profile skill |

Stdout = data (including `--json`). Stderr = progress, errors, logs. See SPEC for per-command output.

---

## Testing

| Level | Location | Notes |
|-------|----------|-------|
| Unit | `#[cfg(test)]` in modules | resolver, profiles, discovery, adapters |
| Integration | `tests/cli.rs` | `assert_cmd`; isolate with temp `HOME` + `SKM_STORE` |

```bash
cargo test
cargo clippy -- -D warnings
cargo fmt --check
```

Helper pattern in `tests/cli.rs`: `with_env(home, store)` sets `HOME`, `XDG_CONFIG_HOME`, `SKM_STORE`, `current_dir`.

---

## Extending the codebase

### New agent adapter

1. Implement `AgentAdapter` in `src/adapters/mod.rs`
2. Register in `get_adapter()` and `INIT_AGENTS` (init/switch-agent pickers)
3. Unit test `target_dir` for project + user levels
4. Integration test: `init --agent <id>` + `use-profile` → symlink under expected path
5. Document in [SPEC-AGENTS.md](SPEC-AGENTS.md) and README agent table

### New doctor check

1. Add function in `doctor/checks.rs` returning `Vec<Issue>` with stable `code`
2. Wire into `doctor::run_checks`
3. Integration test in `tests/cli.rs` for human + `--json` output
4. Document code in [SPEC.md](SPEC.md) doctor table

### New interactive picker

Reuse `tui::MultiSelect` rather than `dialoguer::MultiSelect` so the keys and hint bar stay
consistent. Add widget behavior as pure `State` methods with unit tests; gate the command on
`io::stdin().is_terminal()` and return `SkmError::NotATty` off-TTY.

### New mutating command

Prefer routing placement changes through `reconcile()` or `refresh_store_index` rather than ad-hoc symlink or index updates.

---

## Roadmap

| Version | Status | Scope |
|---------|--------|-------|
| 0.1.0 | Shipped | Core CLI, 3 agents |
| 0.2.0 | Shipped | `doctor`, `--json`, Tier 1 agents, scan adopt |
| 0.2.1 | Shipped | Remove `codex` adapter; `switch-agent` same-target fix; `generic` client docs |
| 0.2.2 | Shipped | Drop `skm add`; engineer-oriented `--help` |
| 0.3.0 | Planned | Windows binary, GitHub import |
| Later | — | Tier 2 agents, `freeze`, variants, skill groups |

---

## Settled decisions

- Unified `skm init` (no `init store` subcommands)
- One agent per setup file
- Symlink-only placements (no copy-mode ledger yet)
- Nested skill discovery at any depth; skip walking inside valid skill dirs when scanning for orphans
- `use-profile` replaces `skm use`; writes active profile only after reconcile succeeds
- Index is disposable — always rebuildable from store + meta on disk
