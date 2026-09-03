# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.3](https://github.com/isaryx/skill-manager/releases/tag/v0.3.3) - 2026-09-03

CLI refactor: multi-active profiles and split agent/profile commands for interactive vs scripting.

### Changed

- **`skm use-agents`** replaces **`skm setup-agents`** / **`skm switch-agent`**; use **`skm add-agent`** / **`skm remove-agent`** for scripting.
- **`skm use-profiles`** is interactive-only; use **`skm add-profile`** / **`skm remove-profile`** for scripting.
- **`skm use-profile`** removed (no alias).
- **Active profiles** are stored as a list in `.skm.toml` (`active = ["work"]`); legacy `active = "work"` is still read. Skills from all active profiles are merged when wiring links.
- **`skm status --json`** reports `profiles` (array) instead of `profile`.
- **`add-profile`**, **`remove-profile`**, **`add-agent`**, and **`remove-agent`** are no-ops with a message when the target is already in (or not in) the set.
- **`remove-agent`** on the last target agent fails before making changes.
## [0.3.2](https://github.com/isaryx/skill-manager/releases/tag/v0.3.2) - 2026-09-03

Project commands require `./.skm.toml` by default, and root help groups commands by project vs store.

### Changed

- `skm sync`, `skm use-profile`, and `skm setup-agents` require `./.skm.toml` in the current directory (same as `skm status`); use `--user` / `-u` for `~/.skm.toml`.
- Root `--help` groups commands under `PROJECT COMMANDS` and `STORE COMMANDS`.



## [0.3.1](https://github.com/isaryx/skill-manager/releases/tag/v0.3.1) - 2026-09-01

Prune safety, missing-home error, and a cheaper disabled-list read on index rebuild.

### Changed

- Index rebuild and meta adoption read the disabled-skill list once per pass instead of once per skill.



### Fixed

- Unwiring no longer deletes empty directories inside a project or hand-installed skill (`SKILL.md`).
- A missing home directory is an error instead of treating `/` as `$HOME`.



## [0.3.0](https://github.com/isaryx/skill-manager/releases/tag/v0.3.0) - 2026-09-01

Multiple target agents per setup, profile inheritance, local git excludes, and `skm destroy`.

### Changed

- **Multiple target agents per config (breaking)** — `[placement]` now takes a list:
`agents = ["claude-code", "cursor"]`. Every listed agent gets its own symlinks, so one profile
can serve several tools at once. Setups written with the old single `agent = "…"` are still
read, and rewritten as a list the next time skm writes the file. `--agent` on `skm init` is
repeatable and comma-separated (`--agent claude-code,cursor`); repeats and ids that resolve to
the same directory (`codex` and `generic`) are collapsed, so no directory is placed into twice.
- `skm switch-agent` **is now** `skm setup-agents` (the old name remains as an alias). Without
`--agent` it opens the same full-screen checkbox list as `skm init`, pre-checked with the
current agents. Agents added to the set get skills synced into their directory (TTY prompt;
auto off-TTY when a profile is active); agents removed from it have their store-owned links
unwired, and the managed git exclude block is rebuilt from the links still on disk. Dropping
agents alone no longer triggers a sync, and reordering the same set is treated as unchanged.
- `skm status` heads its output with `Target agents:` and one line per agent. With more than
one agent, **Linked** and **Conflicts** are grouped under a per-agent heading, since a name can
be conflicted in one agent's directory and linked in another's.
- `--json` **(breaking)** — `status` now reports
`{ agents: [{ agent, skills_path, skills, conflicts }], profile }` instead of a single flat
agent, and `doctor` reports `agents: [...]` in place of `agent`. Doctor issues raised against
one agent's skills directory carry that agent in a new `agent` field.
- `skm doctor` reports `config.unknown_agent` once per unknown agent, and the new
`config.no_agents` error when the list is empty.
- `skm profile setup` **and** `skm skill setup` — replaced the inline `dialoguer` multi-select
with a full-screen picker on the alternate screen: `/` search filter, `[x]` checkboxes, a
bottom hint bar, arrow **and** vim (`k`/`j`, `g`/`G`) navigation, `a` to toggle every matching
row, and paging. The filter is a case-insensitive AND over whitespace-separated terms and never
disturbs the selection. A status line reports the selection, the list size (`27 items`, or
`12 of 27 match` while filtering) and `↑n` / `↓n` for rows off screen. `q` / `Esc` / `Ctrl-C`
cancel without writing, and restore the terminal.
- `profile setup` now titles the list with the profile name
- `skm init` refuses an existing `./.skm.toml` before the agent picker or store work. The
error points at `skm setup-agents` and `skm use-profile`. `--force` still overwrites.
- `skm use-profile` accepts an optional profile name. Without one, a TTY opens a simple
single-select list of available profiles, marking and defaulting to the active profile.



### Added

- `skm destroy` — tear down this project's `./.skm.toml`: confirm on a TTY (`--force`
off-TTY), unwire store-owned skill links in every known project agent directory (not only
agents listed in the file), remove the managed git exclude block, then delete the setup file.
The skill store, profiles, and foreign skills are not touched. `--dry-run` previews without
writing. A missing `[profile].active` (or a name with no profile in the store) prints
`warning: profile not found`.
- **Local Git excludes for linked skills** — project syncs maintain a clone-local managed block in
`.git/info/exclude`, preventing store-owned symlinks from being committed without changing the
project `.gitignore`. Enabled by default; set `[placement].ignore_links = false` to opt out.
When the exclude actually changes, `sync` / `use-profile` / `setup-agents` log
`updating local git exclude`. Patterns are anchored to the worktree root, so they never match a
same-named path elsewhere in the repo. If the managed block has been edited into something skm
cannot parse, it is left alone with a warning and reconcile continues — the links still get
wired. `skm doctor` reports already tracked links as `link.tracked`.
- `skm profile extend <profile>` — pick which profiles a profile inherits skills from, in the
same full-screen picker. Creates the profile if it does not exist, like `setup`. `extends` is a
live reference: the skill list is flattened at sync time, so editing a base profile updates
everything extending it. Own skills come first, then
inherited depth-first, deduplicated by ID. A profile whose skills all come from `extends` is
valid. Cycles, chains deeper than 8, self-extension and duplicate entries are rejected both when
written and when resolved; the picker never offers a profile that would close a cycle, and a
selection that would break the graph is rejected before anything is written.
`skm profile rm` now refuses while another profile extends the target, and `skm doctor` reports
a broken graph as `profile.extend_broken`
- `skm profile show` — marks inherited skills as `git (from base)`, combining with the
disabled marker as `git (from base, disabled)`
- `skm profile show <profile> --tree` — prints the extend graph instead of the flat list, so
the *path* a skill arrived by is visible, not just its origin. `(*)` marks a profile subtree or
skill already accounted for above, and the resolved count is reported alongside how many of
those are disabled and therefore not wired. Unlike the flat listing it renders a broken graph,
marking each `(cycle)`, `(not found)` or `(too deep)` in place before exiting with the same code
the flat listing would
- `src/tui/` — reusable full-screen widgets. `tui::MultiSelect` backs both setup commands and is
the intended base for future pickers (see [docs/DESIGN.md](docs/DESIGN.md)). Text it draws is
stripped of control characters, so a skill name cannot emit terminal escapes; the IDs written
back to the profile are unchanged



### Fixed

- A `--user` sync from a git project no longer deletes that project's managed exclude block.
Empty patterns still remove the block when `ignore_links = false`; they no longer mean "this
command had no in-worktree targets".
- `skm setup-agents` writes the new agent list before unwiring dropped directories, so a failed
config write cannot resurrect those links on the next sync. A failed sync still leaves the
previous list in place.



## [0.2.2](https://github.com/isaryx/skill-manager/releases/tag/v0.2.2) - 2026-08-31

Engineer-oriented CLI help, and drop the `skm add` alias.

### Removed

- `skm add` — use `skm import`



### Changed

- `skm -h` **/** `--help` — store, profiles, and symlink model; examples; automation (`SKM_STORE`, `--json` stdout vs stderr, `--dry-run`, exit codes); docs URL
- `skm status --help` — read-only; Linked vs Conflicts (non-skm name clash); requires `./.skm.toml` unless `--user`; missing or broken links are `skm doctor`
- `--agent` **values** — skills directory per agent, including Copilot project vs `--user`
- **README** — store wording, scripting/CI, Copilot user path
- Completions regenerated from clap



## [0.2.1](https://github.com/isaryx/skill-manager/releases/tag/v0.2.1) - 2026-08-28

Agent adapter cleanup and `switch-agent` fix when agents share a skills directory.

### Changed

- `codex` **agent removed** — use `generic` instead (same `.agents/skills` path). Existing configs with `placement.agent = "codex"` still resolve.
- `generic` **agent** — help and docs now list supported clients: Codex, Cursor, Gemini CLI, Copilot CLI ([docs/SPEC-AGENTS.md](docs/SPEC-AGENTS.md))



### Fixed

- `skm switch-agent` — when old and new agents resolve to the same target directory, only updates `placement.agent` (no sync, no cleanup of existing symlinks). Fixes `codex` → `generic` and any future agents that share a path.



## [0.2.0](https://github.com/isaryx/skill-manager/releases/tag/v0.2.0) - 2026-08-28

Health checks, scriptable JSON output, Tier 1 agent adapters, and CLI polish.

### Added

- `skm doctor` — read-only health report for the store, profiles, and skill links (`--json` supported)
- **Global** `--json` — machine-readable output for `status`, `ls`, `skill ls`, and `doctor`
- **Agent adapters** — `codex`, `gemini-cli`, `copilot-cli` (see [docs/SPEC-AGENTS.md](docs/SPEC-AGENTS.md))
- `--color auto|always|never` — global flag; `auto` respects `NO_COLOR`, `CLICOLOR`, `CLICOLOR_FORCE`
- `--dry-run` — preview link changes for `sync` and `use-profile`; preview removal for `skill rm`
- **Shell completions** — `completions/` (bash, zsh, fish); regenerate with `cargo run --example generate-completions`
- **Foreign skill handling** — reconcile skips profile placements blocked by non-skm entries; `status` and `doctor` report conflicts; `init --accept-existing-skills` for non-empty agent dirs off-TTY



### Changed

- `skm init` and `skm switch-agent` accept the three new agent ids
- `skm init` — prompts when the agent skills directory is not empty; refuses off-TTY unless `--accept-existing-skills`
- `skm sync` **/** `use-profile` — skip conflicted placement names instead of failing; other skills still link
- `skm status` — Linked and Conflicts sections; JSON includes `conflicts` with `reason: "conflicted"`
- `skm doctor` — `link.conflict` info when a profile skill is blocked by a foreign entry
- `skm scan` **/** `skm sync` — adopt on-disk skills that have no provenance meta (writes `.skm/meta/<bundle>.toml` or `<id>.toml` with `source_type = "store"`, `transfer = "adopted"`; never overwrites existing meta from `skm import`)
- `--json` on unsupported commands exits 2 with a clear error (no silent ignore)
- `--dry-run` cannot be combined with `--json`
- `skm --help` long description lists all shipped agents



### Dependencies

- `serde_json` — JSON output for `--json` commands



## [0.1.0](https://github.com/isaryx/skill-manager/releases/tag/v0.1.0) - 2026-08-28

First public release of `skm` — one skill library, named profiles, and symlink-based installs for AI agent folders.

### Added

- `skm init` — set up a skill store and project config in one step
- `skm import` — import a skill or nested skill tree with `--copy` or `--move`; rename with `--as` (`skm add` is an alias)
- `skm skill ls` **/** `setup` **/** `rm` — list, enable/disable, or permanently remove library skills (`rm` also updates profiles; `--force` required when not on a TTY)
- `skm ls` — list skills and profiles (`skill/…`, `profile/…`); `-s` / `--skill` or `-p` / `--profile` to filter
- `skm profile setup` **/** `ls` **/** `show` **/** `rm` — create and manage profiles
- `skm use-profile` — activate a profile and link its skills into the agent folder
- `skm sync` — refresh skill links without changing the active profile
- `skm status` — show target agent, active profile, and linked skills
- `skm scan` — rescan the skill store when files change on disk
- `skm switch-agent` — switch between `generic`, `claude-code`, and `cursor`
- **Store override** — `--store` or `SKM_STORE`
- **User-level config** — `~/.skm.toml` via `--user` / `-u` on supported commands
- **Interactive prompts** — store location, agent, library skills, and profile skills on a TTY
- **Colored output** — highlighted `status`; sync shows `+`/`-` like `git diff`
- **Safe sync** — manually installed skills in the agent folder are left untouched
- **MIT license** — see [LICENSE](LICENSE)

