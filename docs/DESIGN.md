# Design

**CLI:** `skm` · **Version:** 0.3.2

Contributor-facing architecture. User-visible behavior lives in [SPEC.md](SPEC.md). CLI conventions: [../guides/cli-guidelines.md](../guides/cli-guidelines.md).

---

## Domain model

| Concept | Definition |
|---------|------------|
| **Store** | Root directory (`~/.skill-store` default). Skills + `.skm/` metadata. |
| **Library** | All skill dirs under `$STORE/` (flat or nested). |
| **Skill ID** | Store-relative path to a skill dir (`docx`, `engineering/tdd`). |
| **Bundle** | Skill tree rooted at one store prefix; one meta file at bundle root (`local.toml` covers `local/*`). |
| **Profile** | Named skill selection in `.skm/profiles/<name>.toml`. May `extends` other profiles. |
| **Flattened profile** | A profile's own skills plus everything it extends, deduplicated by ID. |
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
| Setup | `./.skm.toml` or `~/.skm.toml` | `placement.agents` (list; legacy `agent = "…"` still read), `placement.ignore_links` (default true), `[profile].active` |
| Profile | `$STORE/.skm/profiles/*.toml` | `[[skill]] id = "…"` |

Store path resolution: `--store` → `SKM_STORE` → app config → `~/.skill-store`.

### Setup file selection

| API | Validates agent | Used by |
|-----|-----------------|---------|
| `select_setup` | yes (`read_setup`) | doctor with `--user` (lenient path uses `select_setup_lenient`) |
| `select_setup_lenient` | no (`read_setup_raw`) | doctor (report `unknown_agent` instead of failing early) |
| `select_command_setup` | yes | status, sync, use-profile, setup-agents, profile show (requires `./.skm.toml` unless `--user`) |
| `select_project_setup` | yes | destroy (requires `./.skm.toml`) |
| `select_project_setup_raw` | no | `destroy` (tear down even a broken agent list; known agent dirs are still unwired) |

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
store/        StorePaths, discovery, profiles, extends graph, pool import, disabled, validate
resolver/     Pure profile → SkillPlacement
sync/         reconcile(), symlink walk/apply (sync/links.rs), managed local exclude
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

### Profile inheritance (`store/extends.rs`)

`extends` is a live reference (semantics and limits: [SPEC.md](SPEC.md#profile-inheritance-extends)).
`flatten_profile` walks the graph and returns `FlatSkill { id, from }`, where `from` is the profile
a skill was inherited from — that attribution is what `profile show` prints.

**The resolver stays I/O-free.** Rather than teach `resolve` about the graph, callers use
`extends::load_flattened_profile`, which returns a plain `ProfileFile` with the graph already
collapsed into `skill`. Keeps `resolver` unit-testable in isolation.

**Which view to use.** Operations that *edit* profile files (`profile setup`, `skill rm`) work on
the direct list; operations that *resolve placements* (`sync`, `use-profile`, `status`, `doctor`
link checks) work on the flattened list. `set_profile_skills` and `set_profile_extends` are both
read-modify-write for this reason — rebuilding the file from scratch would drop the other field.

**Tree view.** `build_tree` is a second, deliberately error-tolerant walk returning a `Tree`
(`root`, `resolved`, `disabled`, `error`). It marks a cycle, missing profile or over-deep chain in
place and keeps going, so `--tree` can render a graph that `flatten_profile` rejects outright.
Returning the first error rather than swallowing it lets `profile show --tree` print the tree and
*then* fail with the same exit code the flat listing gives for that graph. `render_tree` is pure
(`node`, `color`) and unit-tested against exact expected lines.

`TreeNode::notes` is `Vec<&'static str>` on purpose: a closed vocabulary of short labels. Putting
error text in a node would break the tree, since a TOML parse error is multi-line with caret art.
`NodeKind` states whether a node is a profile or a skill rather than leaving consumers to infer it
from having no children — an empty profile is also a childless leaf.

Because `build_tree` duplicates the traversal, `the_tree_and_the_flat_listing_agree_on_every_healthy_shape`
pins the two walks against each other across chain, diamond, fan-out, empty-profile and
repeated-skill graphs. Any drift between them shows up there.

**Validating before writing.** `flatten_with_extends` walks the graph with the root's `extends`
replaced by a hypothetical list, and tolerates a root that does not exist yet. `profile extend`
uses it to reject a cycle or an over-deep chain *before* `set_profile_extends`; writing first
would persist a selection that is then rejected, leaving the profile broken on disk.

**Depth is checked before the `expanded` short-circuit** in `Walk::visit`. The other order makes
the verdict depend on which branch `extends` happens to list first, because a profile already
reached by a short route would skip the check.

**Cycle detection.** `Walk::visit` is recursive on purpose. `path` holds the chain being expanded
and catches any edge back into it; `expanded` is only memoization so a diamond is walked once.
Correctness depends on recursion: `visit` does not return until a subtree is fully expanded, so a
route that re-enters a profile meanwhile still finds it on `path`. An iterative walk with one
shared visited set lacks that property and can skip the branch where a cycle is visible. Recursion
is bounded — and therefore safe — only because `MAX_EXTEND_DEPTH` caps the depth, which is what
makes that limit load-bearing rather than cosmetic.

### `reconcile_with_setup` pipeline

1. Load active profile (or override)
2. `resolver::resolve(profile, store, disabled)`
3. If `--dry-run`: `compute_link_changes` → print `(dry-run) +/-` on stderr; planned exclude paths the same way; return
4. `ensure_store_subdirs`
5. `db::refresh_store_index`
6. Resolve agent target; create target dir
7. Sync the managed block in `info/exclude` when `placement.ignore_links` is on (default) and the
   target is in a git worktree — [SPEC.md](SPEC.md#local-exclude-for-store-owned-links-placementignore_links).
   This happens before link mutations, so failure cannot leave a new link unignored. Never writes
   a `.gitignore` in the tree
8. `remove_dangling_store_symlinks` → `clean_target` → `prune_empty_skill_dirs` → `apply_placements` (skips foreign occupants; does not fail reconcile)

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

Read-only. Runs store → index → disk skills → meta → profiles → config → links (if active profile resolves). Uses `select_setup_lenient` so invalid agents surface as `config.unknown_agent`. Link checks reuse store-ownership rules from `sync/links.rs`. `link.tracked` shells out to `git ls-files` only when ignore is on and a `.git` ancestor exists; missing `git` skips the check. Does not call `reconcile` or mutate disk. Does not write `info/exclude`.

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
| Integration | `tests/*.rs` (`common/` helpers) | `assert_cmd`; isolate with temp `HOME` + `SKM_STORE` |

```bash
cargo test
cargo clippy -- -D warnings
cargo fmt --check
```

Helper pattern in `tests/common/mod.rs`: `with_env(home, store)` sets `HOME`, `XDG_CONFIG_HOME`, `SKM_STORE`, `current_dir`.

**`ignore_links` / local exclude.** Required cases live in [SPEC.md](SPEC.md#local-exclude-for-store-owned-links-placementignore_links) (table). Do not ship the feature without them. The one that is easy to skip and expensive to miss: **two git projects, one store** — each `git init` in its own tempdir (never in `HOME`), `use-profile` in both, then assert each `info/exclude` lists only that worktree's paths and neither `.gitignore` changed. A single-project test cannot catch writes that leak into the other repo, `HOME`, or a global exclude.

---

## Extending the codebase

### New agent adapter

1. Implement `AgentAdapter` in `src/adapters/mod.rs`
2. Register in `get_adapter()` and `INIT_AGENTS` (the `init` / `setup-agents` picker)
3. Unit test `target_dir` for project + user levels
4. Integration test: `init --agent <id>` + `use-profile` → symlink under expected path
5. Document in [SPEC-AGENTS.md](SPEC-AGENTS.md) and README agent table

### New profile-graph consumer

Decide whether the operation edits files (direct list) or resolves placements (flattened list),
then use `load_profile` or `extends::load_flattened_profile` accordingly. Anything that resolves
must tolerate a broken graph: `doctor` reports `profile.extend_broken` instead of aborting, and
`clear_active_profile_if_empty` declines to act when flattening fails.

### New doctor check

1. Add function in `doctor/checks.rs` returning `Vec<Issue>` with stable `code`
2. Wire into `doctor::run_checks`
3. Integration test in `tests/doctor.rs` for human + `--json` output
4. Document code in [SPEC.md](SPEC.md) doctor table

### New interactive picker

Reuse `tui::MultiSelect` rather than `dialoguer::MultiSelect` so the keys and hint bar stay
consistent. Add widget behavior as pure `State` methods with unit tests; gate the command on
`io::stdin().is_terminal()` and return `SkmError::NotATty` off-TTY.

### New mutating command

Prefer routing placement changes through `reconcile()` or `refresh_store_index` rather than ad-hoc symlink or index updates.

### Local exclude (`ignore_links`)

Keep gitignore/exclude logic in `sync/`, called from `reconcile` (and from `setup-agents` via `refresh_local_exclude` when the agent set changes without a sync). The managed block is one list per worktree, so `sync_local_exclude` takes **every** target directory at once — writing it per directory would have each agent's paths erase the previous agent's. Resolve the exclude file with `git rev-parse --git-path info/exclude` from the skills dir (or project root), not a hard-coded `.git/info/exclude`. When adding tests, copy the SPEC table — especially **two projects, one store**.

Two invariants worth keeping when touching `sync/exclude.rs`:

- **Anchor every pattern** with a leading `/`. An unanchored pattern with no separator matches at every depth. Today's adapters all nest two segments deep so the interior separator is already there, but that is an adapter property, not a guarantee of this module.
- **Degrade, never block.** The exclude is a convenience and the links are the command, so anything this module cannot do — a block it cannot parse, a path gitignore cannot express — is a `progress::warn` and a `return Ok(())`, not an error. `ignore_links = false` already supports wiring with no exclude at all, so refusing to wire would be inconsistent with the feature's own opt-out. Warnings that name a file must print its absolute path: in a linked worktree the exclude is under `.git/worktrees/<name>/`. Empty patterns mean "remove the block" only for that opt-out. A command whose targets all sit outside the worktree (typical `--user`) must leave an existing block alone.

---

## Roadmap

| Version | Status | Scope |
|---------|--------|-------|
| 0.1.0 | Shipped | Core CLI, 3 agents |
| 0.2.0 | Shipped | `doctor`, `--json`, Tier 1 agents, scan adopt |
| 0.2.1 | Shipped | Remove `codex` adapter; `switch-agent` same-target fix; `generic` client docs |
| 0.2.2 | Shipped | Drop `skm add`; engineer-oriented `--help` |
| 0.3.0 | Shipped | Multi-agent setups, profile `extends`, local git exclude, `skm destroy`, full-screen picker |
| 0.3.1 | Shipped | Prune stays out of skill trees; missing home is an error; one disabled-list read per index pass |
| 0.3.2 | Shipped | Project commands require `./.skm.toml` unless `--user`; grouped root help |
| 0.4.0 | Planned | Windows binary, GitHub import |
| Later | — | Tier 2 agents, `freeze`, variants, skill groups |

---

## Settled decisions

- Unified `skm init` (no `init store` subcommands)
- One setup file may list several target agents (`placement.agents`)
- Symlink-only placements (no copy-mode ledger yet)
- Store-owned links are ignored by default via a managed block in **`.git/info/exclude`** (local, not committed). Never edit project `.gitignore` or `<skills-dir>/.gitignore`. Never `*`. Never `git rm --cached`
- Nested skill discovery at any depth; skip walking inside valid skill dirs when scanning for orphans
- `use-profile` replaces `skm use`; writes active profile only after reconcile succeeds
- Index is disposable — always rebuildable from store + meta on disk
