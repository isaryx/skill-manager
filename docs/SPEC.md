# Spec: `skm`

**Version:** 0.3.1 · Architecture: [DESIGN.md](DESIGN.md) · Agents: [SPEC-AGENTS.md](SPEC-AGENTS.md)

CLI for managing AI agent skills: one library, named profiles, symlink-based installs.

---

## Commands

Global flags:

| Flag | Notes |
|------|--------|
| `--verbose` / `-v` | Debug logs on stderr |
| `--store <path>` | Override store (`SKM_STORE`) |
| `--json` | **Only** `status`, `ls`, `skill ls`, `doctor` — otherwise exit 2 |
| `--dry-run` | **Only** `sync`, `use-profile`, `skill rm`, `destroy` — preview; no writes |
| `--color auto\|always\|never` | Human output styling (`auto` respects `NO_COLOR`) |

`--dry-run` and `--json` cannot be combined.

Many commands accept `--user` / `-u` for `~/.skm.toml`.

```bash
skm init [--agent AGENT]... [--force] [--accept-existing-skills]
skm import <dir> --copy|--move [--as NAME]

skm profile setup|extend|ls|show|rm <name>
skm skill setup|ls|rm <id> [--force]
skm ls [-s|--skill | -p|--profile]

skm use-profile [profile]
skm setup-agents [--agent AGENT]...        # alias: switch-agent
skm destroy [--force]
skm sync
skm status
skm scan
skm doctor [--json]
```

Agents: `generic` (Codex, Cursor, Gemini CLI, Copilot CLI), `claude-code`, `cursor`, `gemini-cli`, `copilot-cli`. Existing configs with `placement.agent = "codex"` still work.

`--agent` is repeatable and comma-separated (`--agent claude-code,cursor`); repeats and ids that resolve to the same directory are collapsed. Omitted on a TTY, the agents are picked from a checkbox list.

---

## Configuration

**App** (`~/.config/skm/config.toml`): `[store].path`

**Setup** (`./.skm.toml` or `~/.skm.toml`):

```toml
version = 1
[placement]
agents = ["claude-code", "cursor"]   # one entry per target agent
# ignore_links = true   # default when omitted; set false to opt out
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

Creates/validates store, writes app config if needed, writes `./.skm.toml` with `placement.agents`. Refuses an existing `./.skm.toml` **before** prompting for agents or touching the store, unless `--force`. The error names `skm setup-agents` (change agents) and `skm use-profile` (activate a profile). `--force` overwrites the file and preserves `[profile].active`.

If any target agent's skills directory already has entries (project-bundled or hand-installed skills), `init` prompts once on a TTY, naming those directories, to continue. Off-TTY, pass `--accept-existing-skills` to proceed. `skm` only manages its own symlinks and will not remove foreign entries; names already taken are skipped during sync.

### `skm destroy`

Requires `./.skm.toml` in the current directory (does not fall back to `~/.skm.toml`). Unwires store-owned skill links in **every known project agent directory** (not only `placement.agents`), so an empty or stale agent list cannot leave links behind. Then removes the managed git exclude block (user lines in `info/exclude` stay) and deletes `./.skm.toml`. Foreign / project-bundled skills, the skill store, and app config are left alone.

TTY: confirm, default no. Off-TTY: `--force`. `--dry-run` prints the plan and writes nothing. Missing `[profile].active`, or an active name with no matching profile in the store, prints `warning: profile not found` (same wording as the error). Unknown agent names in the file are warned about; their directories are not cleaned because skm cannot resolve them.

### `skm import`

Requires initialized store. `--copy` and `--move` are required and mutually exclusive.

- Single skill: `SKILL.md` at root, no nested trees
- Skill tree: one or more `SKILL.md` at any depth under one bundle name
- Writes `.skm/meta/<name>.toml`, rebuilds index
- Rejects overwrite and reserved names (`.skm`, leading `.`)

### Profiles & library skills

- `profile setup` — pick a profile's skills from the **enabled** library
- `profile extend` — pick which **profiles** this profile inherits skills from (creates the profile if missing, like `setup`)
- `use-profile [profile]` — activate and sync a named profile; omit the name on a TTY to choose
  from the available profiles. The active profile is marked and selected by default.
- `skill setup` — hide skills via `.skm/disabled.toml` (still in profiles; skipped when wiring)
- `skill rm` — delete from store and all profiles; TTY confirm or `--force`
- Disabled skills show `(disabled)` on `profile show`; sync unwires them

`profile setup` also offers any **disabled** skill the profile already references, marked
`(disabled)`, so editing a profile never silently drops a hidden skill.

#### Profile inheritance (`extends`)

`skm profile extend <profile>` opens the same picker over profile names and writes
`extends = ["base", …]` into the profile file. It creates the profile if missing, like `setup`.

`extends` is a **live reference, not a copy.** The skill list is flattened every time the profile
is resolved, so editing a base profile immediately changes everything extending it.

Flattening rules:

- Own skills first, then inherited depth-first in `extends` order
- Deduplicated by skill ID; the first occurrence wins, so a directly declared skill is attributed
  to the profile itself even when a base also lists it
- A diamond (two profiles extending a common base) contributes that base's skills once
- A profile with no skills of its own is valid; `use-profile` and `doctor` judge emptiness on the
  **flattened** set

Rejected, at write time and again at resolve time since profile files are hand-editable:

| Condition | Error | Exit |
|---|---|---|
| Profile extends itself | `profile \`a\` cannot extend itself` | 2 |
| Same profile listed twice | `duplicate extended profile: a` | 2 |
| Cycle | `extend cycle between profiles: a → b → a` | 2 |
| Chain deeper than **8** hops | `extend chain is deeper than 8: …` | 2 |
| Extends a profile that does not exist | `profile \`a\` extends missing profile \`b\`` | 1 |

The depth limit is a comprehension guard, not what makes resolution terminate — cycle detection
does that. Realistic hierarchies (`base → org → team → personal`) are about three hops.

**Conflicting skills.** Flattening resolves ID-level duplicates but not placement-name
collisions, which are handled by the existing flat-naming rules:

| Case | Behavior |
|---|---|
| Same skill ID declared directly and inherited | One placement; attributed to the profile itself |
| Same ID inherited from two bases (diamond) | One placement |
| Different IDs sharing a leaf (`a/tdd` + `b/tdd`) | Both disambiguate to `a__tdd` / `b__tdd` |
| Two IDs that disambiguate to the same name | Exit 2, `resolve conflict for <name>` |
| Inherited skill that is disabled | Reported `(from base, disabled)`; not wired |
| Placement name taken by a non-skm entry | That placement is skipped; reported as `link.conflict` |

Two consequences specific to `extends`:

- **Placement names depend on profile composition.** `eng` alone places `engineering/tdd` as
  `tdd`; a profile extending both `eng` and an `ops` profile places `engineering__tdd` and
  `ops__tdd`. Extending a second profile can therefore rename skills already in the agent
  directory.
- **Two profiles that each resolve can be unresolvable together.** The conflict names the
  colliding placement but not the profiles that contributed it; `skm profile show <name>` gives
  the origin of each skill.

There is **no way to exclude a skill inherited from a base** — `extends` is union-only, with no
negation or override.

The picker does not offer profiles that already reach this one, since extending them would close
a cycle. `skm profile rm` refuses while another profile extends the target, naming the extenders,
mirroring the existing refusal to remove the active profile.

`skm profile show` prints the flattened list and marks inherited entries: `git (from base)`,
combining with the disabled marker as `git (from base, disabled)`. The `extends` line itself goes
to stderr so stdout stays one skill per line.

#### `profile show --tree`

`--tree` prints the extend graph instead of the flat list. The flat view marks *which* profile a
skill came from; the tree also shows the **path** — whether `work` extends `shared` directly or
inherits it through `base`, which the flat view cannot express.

```
work
├── pdf
├── base
│   ├── docx
│   └── shared
│       ├── git
│       └── writing/adr (disabled)
└── infra
    ├── tf
    └── shared (*)

5 skills resolved, 1 disabled and not wired
```

The graph is a DAG, not a tree, so:

- `(*)` marks a node already accounted for above — a profile subtree rendered elsewhere, or a
  skill an earlier profile already contributed. Those are not counted twice.
- The count equals the line count of the flat listing. It is **not** the number of symlinks
  created: disabled skills resolve but are never wired, which is why they are counted separately.

Unlike the flat listing, `--tree` **renders a broken graph** before failing, marking each problem
in place — `(cycle)`, `(not found)`, `(too deep)`, `(unreadable)` — and continuing through the
siblings. Markers are short fixed labels; the full error (a TOML parse failure, say) is printed
after the tree rather than inside a node, so the tree keeps its shape. This is
the view to reach for when `use-profile` refuses a profile. It then exits with the same code the
flat listing would for that graph (2 for a cycle or over-deep chain, 1 for a missing profile).

`--tree` replaces the one-skill-per-line stdout with the tree and its count, so scripts should not
pass it. The `(extends …)` stderr note is suppressed, since the tree already shows it.

#### Interactive picker

`profile setup`, `profile extend` and `skill setup` take over the terminal with a searchable checkbox list, and
restore the previous screen on exit. Both require a TTY (`SKM_STORE`/`--store` and a
hand-written profile TOML cover automation). Checked means *in the profile* for `profile setup`,
*enabled* for `skill setup`.

Two modes, shown in the bottom hint bar:

| Mode | Keys |
|------|------|
| **List** | `↑`/`↓` or `k`/`j` move (wraps) · `PgUp`/`PgDn` page · `Home`/`g`, `End`/`G` jump · `Space` toggle · `a` toggle all matching rows · `/` search · `Enter` confirm · `Esc` clear the filter, or quit when there is none · `q` quit |
| **Search** | type to filter · `Backspace` / `Ctrl-U` edit · `↑`/`↓` move · `Tab` toggle · `Enter` or `Esc` leave search, keeping the filter |

The filter is a case-insensitive AND over whitespace-separated terms, matched against the skill
ID and its `(disabled)` marker. It never changes the selection, so you can search, toggle, search
again, and confirm once. Quitting (`q`, `Esc`, `Ctrl-C`) writes nothing and exits 1.

The status line above the hint bar reads `<n> selected`, then the list size — `27 items`
unfiltered, or `12 of 27 match` while filtering — then `↑<n>` / `↓<n>` for the rows off screen
above and below. Each arrow is dropped when there is nothing that way, so no arrows means the
whole list is visible.

### `skm use-profile` / `skm sync`

Both call `reconcile()`: validate → fix (rebuild index, adopt missing meta) → sync the managed
local exclude (when `ignore_links` is on) → clean stale `skm` symlinks → apply links. The exclude
is written before symlinks change, so a write failure cannot leave newly linked skills unignored.

`use-profile` writes `[profile].active` only after reconcile succeeds. `sync` does not change active profile.

`--dry-run` resolves the profile and prints planned `+`/`-` link changes on stderr without writing symlinks, exclude, or updating the active profile (`use-profile` only). Planned exclude-list changes print the same way (`+`/`-` paths under the managed exclude).

Flat names in agent dir: unique leaf → leaf; collisions → `__` between path segments. Unresolvable collision → exit 2.

**Foreign skills:** a top-level entry in the agent skills dir that is not a store-owned symlink is left untouched. If a profile placement targets an occupied name, that placement is **skipped** (stderr: `skipped <name> (conflicted)`); reconcile continues and exits **0**. That name is **not** added to the managed exclude list, so a project-bundled skill stays committable.

#### Local exclude for store-owned links (`placement.ignore_links`)

Git records a symlink as the link itself. skm placements are absolute paths into the store, so `git add` on a wired skill commits a machine-local target that is dangling for everyone else.

`placement.ignore_links` (bool, **default `true`** when omitted) keeps those paths out of `git add` **without editing any committed file**. Set `ignore_links = false` to opt out. `skm init` does not write the key.

When on, reconcile maintains a **managed block** in the repo's **local** exclude file (not in the tree, never committed):

```
# BEGIN skm-managed
/.claude/skills/docx
/.claude/skills/git
# END skm-managed
```

Path: `git rev-parse --git-path info/exclude` (usually `.git/info/exclude`; correct for linked worktrees). Patterns are relative to the worktree root and **anchored** with a leading `/` — an unanchored pattern with no separator in it matches at every depth, so a bare `docx` would also ignore an unrelated `vendor/docx`.

skm does **not** write `<skills-dir>/.gitignore` or the project `.gitignore`. Those stay entirely the user's.

| Rule | Behavior |
|------|----------|
| What is listed | Worktree-relative paths of store-owned placements this reconcile **wires** (not skipped conflicts, not disabled) |
| What is not listed | Foreign occupants; never `*` or the whole skills directory |
| Same file, other content | User lines outside the block are left alone; only the block is rewritten |
| `ignore_links = false` | Remove the managed block; stop updating. Do not delete `info/exclude` if other lines remain |
| Where | Only if the skills directory is inside a git worktree. Typical `~/.claude/skills` is skipped |
| `git` missing | Skip exclude management; do not fail reconcile |
| Already tracked | Exclude does not unstage. Never run `git rm` / `git add` |
| `--dry-run` | Print the planned block; write nothing |
| Special characters in a path | Escape so the pattern matches the literal placement |
| Path a pattern cannot express | A line break has no gitignore escape: skip that path with a `warning:` naming it, and list the rest |
| Malformed managed block | Not exactly one `BEGIN` … `END` pair (something else edited the file): leave the file byte-for-byte alone, `warning:` naming its **absolute path**, and **continue reconciling**. The links are the command; the exclude is a convenience, and `ignore_links = false` already supports wiring without one |
| Other clones / teammates | Each machine gets this on its own `sync` / `use-profile`. Nothing to commit |

**Tests (required).** Integration tests in `tests/exclude.rs` must cover this when it is implemented — including **two independent git projects** that share one store. Exclude is per-repo; a bug that writes into `HOME` or a global gitignore would only show up with more than one project.

| Test | Assert |
|------|--------|
| Git project, default on | After `use-profile` / `sync`, that repo's `info/exclude` has the managed block with **worktree-relative** wired paths. No `.gitignore` created or edited at the project root or in the skills dir |
| **Two git projects, one store** | Separate `git init` dirs, same `HOME` + `SKM_STORE`, `use-profile` in each. A's exclude lists only A's skills paths; B's only B's. A's file is unchanged by a later sync in B. Neither project's `.gitignore` is touched |
| Not a git repo | Project without `.git`: no `info/exclude` created, no `.gitignore` added |
| Opt-out is per project | `ignore_links = false` in A removes A's managed block only; B's exclude is unchanged |
| `--dry-run` | Planned exclude `+`/`-` on stderr; `info/exclude` not written |
| Progress | When the managed block is written or removed, stderr: `updating local git exclude`. No line if unchanged |
| User lines preserved | Pre-existing lines in `info/exclude` outside the block survive a rewrite |
| `--user` / no git at home | User-level skills dir not in a worktree: **no exclude write**. That includes a later `--user` sync from a git project that already has a managed block — do not remove it. Only `ignore_links = false` removes the block |
| `link.tracked` | Store-owned symlink is `git add`ed: `doctor` reports `link.tracked` (human + `--json`) |
| `setup-agents` | Exclude block in **that** project lists the paths of every current agent; a dropped agent's paths are gone. Other projects untouched |
| Malformed block | Break the block by hand, then `sync`: exits **0**, links still wired, `warning:` names the exclude's absolute path, and the file is byte-for-byte unchanged |
| Anchoring | Emitted patterns start with `/`, and a same-named path elsewhere in the repo (`vendor/skills/<id>`) is **not** ignored (assert via `git check-ignore`) |

Init `git` **in the project tempdir**, not in `HOME`, so user-level tests stay non-git. Skip the git-backed cases if `git` is not on `PATH` (same as runtime).

### `skm setup-agents`

Replaces `placement.agents` in the setup file. Without `--agent`, the agents are picked from a checkbox list pre-checked with the current set (`switch-agent` remains as an alias).

When the new set adds a target directory, skills are optionally synced into it (TTY prompt; auto on off-TTY when a profile is active). Store-owned symlinks are removed from every directory that leaves the set. Dropping agents alone needs no sync — the directories that remain are already wired — and with `ignore_links` on, the managed exclude block is rebuilt from the links still on disk, so a dropped agent's paths drop out of the list.

A selection that names the same agents in a different order is treated as unchanged. Replacing a legacy alias with the id it maps to (`codex` → `generic`) rewrites the config but touches no symlinks, since both name the same directory.

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
| `config.unknown_agent` | error | Unknown agent in `placement.agents` (one issue per agent, `agent` field names it) |
| `config.no_agents` | error | `placement.agents` is empty |
| `profile.missing_ref` | error | Profile references missing skill |
| `profile.extend_broken` | error | Cycle, over-deep chain, or missing profile in `extends` |
| `index.stale` | warn | Index count ≠ disk |
| `skill.missing_skill_md` | warn | Dir looks like skill, no `SKILL.md` |
| `meta.orphan` / `link.broken` / `link.stale` | warn | Orphan meta or bad symlinks |
| `profile.empty` | warn | Zero skills after flattening `extends` |
| `meta.missing` | info | No meta — run `skm scan` |
| `profile.disabled_ref` | info | Profile includes disabled skill |
| `link.extra` | info | Store-owned symlink not in active profile |
| `link.conflict` | info | Profile placement blocked by a non-skm entry at that name |
| `link.tracked` | warn | `ignore_links` on, skills dir in a git worktree, and a store-owned symlink is tracked (`git ls-files`). Message names the path and hints `git rm --cached <path>` — skm never untracks |
| `config.no_active_profile` | info | No active profile (skips link checks) |

Exit **1** on any `warn` or `error`; **0** on success or `info` only.

### `skm ls` / `skm status`

Human output on stdout; progress on stderr. `status` shows every target agent with its skills directory, the active profile, **Linked** skills with paths, and **Conflicts** (profile skills blocked by a non-skm entry at that name). With more than one agent, Linked and Conflicts are grouped under a per-agent heading, since a name can be conflicted in one agent's directory and linked in another's.

`ls` lists `profile/…` and `skill/…` under headings (or filtered with `-s`/`-p`).

---

## JSON output (`--json`)

Data on stdout only; no ANSI on stdout.

**`status`:** `{ agents: [{ agent, skills_path, skills: [{ name, source }], conflicts: [{ name, store_id, reason: "conflicted" }] }], profile }`

**`ls` / `skill ls`:** `{ skills: [...] }` and/or `{ profiles: [...] }` depending on filters.

**`doctor`:** `{ ok, store, agents: [...], profile, issues: [{ code, severity, message, agent?, ... }] }` — `ok` matches exit code. Issues belonging to one agent's skills directory carry its `agent`.

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

- **0.4.0:** Windows release binary; `skm import github:…`, `skm update`
- **Later:** Tier 2 agents, skill groups, `skm freeze`, variants (`skm fork`), `skm init --user`, copy-mode placements
