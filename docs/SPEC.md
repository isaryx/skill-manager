# Spec: `skm`

**Version:** 0.2.1 · Architecture: [DESIGN.md](DESIGN.md) · Agents: [SPEC-AGENTS.md](SPEC-AGENTS.md)

CLI for managing AI agent skills: one library, named profiles, symlink-based installs.

---

## Commands

Global flags:

| Flag | Notes |
|------|--------|
| `--verbose` / `-v` | Debug logs on stderr |
| `--store <path>` | Override store (`SKM_STORE`) |
| `--json` | **Only** `status`, `ls`, `skill ls`, `doctor` — otherwise exit 2 |
| `--dry-run` | **Only** `sync`, `use-profile`, `skill rm` — preview; no writes |
| `--color auto\|always\|never` | Human output styling (`auto` respects `NO_COLOR`) |

`--dry-run` and `--json` cannot be combined.

Many commands accept `--user` / `-u` for `~/.skm.toml`.

```bash
skm init [--agent AGENT] [--force] [--accept-existing-skills]
skm import <dir> --copy|--move [--as NAME]    # alias: skm add

skm profile setup|ls|show|rm <name>
skm skill setup|ls|rm <id> [--force]
skm ls [-s|--skill | -p|--profile]

skm use-profile <profile>
skm switch-agent [--agent AGENT]
skm sync
skm status
skm scan
skm doctor [--json]
```

Agents: `generic` (Codex, Cursor, Gemini CLI, Copilot CLI), `claude-code`, `cursor`, `gemini-cli`, `copilot-cli`. Existing configs with `placement.agent = "codex"` still work.

---

## Configuration

**App** (`~/.config/skm/config.toml`): `[store].path`

**Setup** (`./.skm.toml` or `~/.skm.toml`):

```toml
version = 1
[placement]
agent = "claude-code"
[profile]
active = "work"
```

**Profile** (`$STORE/.skm/profiles/work.toml`):

```toml
[[skill]]
id = "engineering/tdd"
```

Store path: `--store` → `SKM_STORE` → app config → `~/.skill-store`.

| Command | Setup file |
|---------|------------|
| Most commands | `./.skm.toml` if present, else `~/.skm.toml` |
| `skm status` | Requires `./.skm.toml` (unless `--user`) |

---

## Command behavior

### `skm init`

Creates/validates store, writes app config if needed, writes `./.skm.toml` with `placement.agent`. Refuses existing setup unless `--force` (preserves `[profile].active`).

If the agent skills directory already has entries (project-bundled or hand-installed skills), `init` prompts on a TTY to continue. Off-TTY, pass `--accept-existing-skills` to proceed. `skm` only manages its own symlinks and will not remove foreign entries; names already taken are skipped during sync.

### `skm import`

Requires initialized store. `--copy` and `--move` are required and mutually exclusive.

- Single skill: `SKILL.md` at root, no nested trees
- Skill tree: one or more `SKILL.md` at any depth under one bundle name
- Writes `.skm/meta/<name>.toml`, rebuilds index
- Rejects overwrite and reserved names (`.skm`, leading `.`)

### Profiles & library skills

- `profile setup` — interactive multi-select from **enabled** library skills
- `skill setup` — hide skills via `.skm/disabled.toml` (still in profiles; skipped when wiring)
- `skill rm` — delete from store and all profiles; TTY confirm or `--force`
- Disabled skills show `(disabled)` on `profile show`; sync unwires them

### `skm use-profile` / `skm sync`

Both call `reconcile()`: validate → fix (rebuild index, adopt missing meta) → clean stale `skm` symlinks → apply links.

`use-profile` writes `[profile].active` only after reconcile succeeds. `sync` does not change active profile.

`--dry-run` resolves the profile and prints planned `+`/`-` link changes on stderr without writing symlinks or updating the active profile (`use-profile` only).

Flat names in agent dir: unique leaf → leaf; collisions → `__` between path segments. Unresolvable collision → exit 2.

**Foreign skills:** a top-level entry in the agent skills dir that is not a store-owned symlink is left untouched. If a profile placement targets an occupied name, that placement is **skipped** (stderr: `skipped <name> (conflicted)`); reconcile continues and exits **0**.

### `skm switch-agent`

Updates `placement.agent` in the setup file. When the old and new agents resolve to **different** target directories, optionally syncs skills to the new agent (TTY prompt; auto on off-TTY when a profile is active) and removes store-owned symlinks from the previous agent's directory.

When both agents share the same target directory (e.g. legacy `codex` → `generic`), only the agent name in config changes — existing symlinks are left in place and sync is skipped.

### `skm skill rm`

`--dry-run` prints planned skill IDs, store path, and affected profiles without deleting.

`--force` required off-TTY (unchanged).

### `skm scan`

Rebuilds `index.db`. Adopts skills copied into the store without meta (create-if-missing only; `source_type = "store"`, `transfer = "adopted"`). Never overwrites import meta.

### `skm doctor`

Read-only health report. Setup selection same as `sync`.

| Code | Sev | Condition |
|------|-----|-----------|
| `store.missing` / `store.invalid` | error | Store layout |
| `config.unknown_agent` | error | Unknown `placement.agent` |
| `profile.missing_ref` | error | Profile references missing skill |
| `index.stale` | warn | Index count ≠ disk |
| `skill.missing_skill_md` | warn | Dir looks like skill, no `SKILL.md` |
| `meta.orphan` / `link.broken` / `link.stale` | warn | Orphan meta or bad symlinks |
| `profile.empty` | warn | Zero skills in profile |
| `meta.missing` | info | No meta — run `skm scan` |
| `profile.disabled_ref` | info | Profile includes disabled skill |
| `link.extra` | info | Store-owned symlink not in active profile |
| `link.conflict` | info | Profile placement blocked by a non-skm entry at that name |
| `config.no_active_profile` | info | No active profile (skips link checks) |

Exit **1** on any `warn` or `error`; **0** on success or `info` only.

### `skm ls` / `skm status`

Human output on stdout; progress on stderr. `status` shows agent, profile, **Linked** skills with paths, and **Conflicts** (profile skills blocked by a non-skm entry at that name).

`ls` lists `profile/…` and `skill/…` under headings (or filtered with `-s`/`-p`).

---

## JSON output (`--json`)

Data on stdout only; no ANSI on stdout.

**`status`:** `{ agent, skills_path, profile, skills: [{ name, source }], conflicts: [{ name, store_id, reason: "conflicted" }] }`

**`ls` / `skill ls`:** `{ skills: [...] }` and/or `{ profiles: [...] }` depending on filters.

**`doctor`:** `{ ok, store, agent, profile, issues: [{ code, severity, message, ... }] }` — `ok` matches exit code.

---

## I/O & exit codes

| Stream | Content |
|--------|---------|
| stdout | Data (`status`, `ls`, `import` tree IDs, `--json`) |
| stderr | Progress, errors, logs (`--verbose` / `RUST_LOG`) |

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Failure (I/O, store, placement, doctor warn/error) |
| 2 | Usage or resolve conflict |

Respect `NO_COLOR`.

---

## Deferred

- **0.3.0:** Windows release binary; `skm import github:…`, `skm update`
- **Later:** Tier 2 agents, skill groups, `skm freeze`, variants (`skm fork`), `skm init --user`, copy-mode placements
