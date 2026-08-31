# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- **`skm profile setup` and `skm skill setup`** — replaced the inline `dialoguer` multi-select
  with a full-screen picker on the alternate screen: `/` search filter, `[x]` checkboxes, a
  bottom hint bar, arrow **and** vim (`k`/`j`, `g`/`G`) navigation, `a` to toggle every matching
  row, and paging. The filter is a case-insensitive AND over whitespace-separated terms and never
  disturbs the selection. A status line reports the selection, the list size (`27 items`, or
  `12 of 27 match` while filtering) and `↑n` / `↓n` for rows off screen. `q` / `Esc` / `Ctrl-C`
  cancel without writing, and restore the terminal.
- **`profile setup`** now titles the list with the profile name

### Added

- `src/tui/` — reusable full-screen widgets. `tui::MultiSelect` backs both setup commands and is
  the intended base for future pickers (see [docs/DESIGN.md](docs/DESIGN.md)). Text it draws is
  stripped of control characters, so a skill name cannot emit terminal escapes; the IDs written
  back to the profile are unchanged

## [0.2.2] - 2026-08-31

Engineer-oriented CLI help, and drop the `skm add` alias.

### Removed

- **`skm add`** — use `skm import`

### Changed

- **`skm -h` / `--help`** — store, profiles, and symlink model; examples; automation (`SKM_STORE`, `--json` stdout vs stderr, `--dry-run`, exit codes); docs URL
- **`skm status --help`** — read-only; Linked vs Conflicts (non-skm name clash); requires `./.skm.toml` unless `--user`; missing or broken links are `skm doctor`
- **`--agent` values** — skills directory per agent, including Copilot project vs `--user`
- **README** — store wording, scripting/CI, Copilot user path
- Completions regenerated from clap

[0.2.2]: https://github.com/isaryx/skill-manager/releases/tag/v0.2.2

## [0.2.1] - 2026-08-28

Agent adapter cleanup and `switch-agent` fix when agents share a skills directory.

### Changed

- **`codex` agent removed** — use `generic` instead (same `.agents/skills` path). Existing configs with `placement.agent = "codex"` still resolve.
- **`generic` agent** — help and docs now list supported clients: Codex, Cursor, Gemini CLI, Copilot CLI ([docs/SPEC-AGENTS.md](docs/SPEC-AGENTS.md))

### Fixed

- **`skm switch-agent`** — when old and new agents resolve to the same target directory, only updates `placement.agent` (no sync, no cleanup of existing symlinks). Fixes `codex` → `generic` and any future agents that share a path.

[0.2.1]: https://github.com/isaryx/skill-manager/releases/tag/v0.2.1

## [0.2.0] - 2026-08-28

Health checks, scriptable JSON output, Tier 1 agent adapters, and CLI polish.

### Added

- **`skm doctor`** — read-only health report for the store, profiles, and skill links (`--json` supported)
- **Global `--json`** — machine-readable output for `status`, `ls`, `skill ls`, and `doctor`
- **Agent adapters** — `codex`, `gemini-cli`, `copilot-cli` (see [docs/SPEC-AGENTS.md](docs/SPEC-AGENTS.md))
- **`--color auto|always|never`** — global flag; `auto` respects `NO_COLOR`, `CLICOLOR`, `CLICOLOR_FORCE`
- **`--dry-run`** — preview link changes for `sync` and `use-profile`; preview removal for `skill rm`
- **Shell completions** — `completions/` (bash, zsh, fish); regenerate with `cargo run --example generate-completions`
- **Foreign skill handling** — reconcile skips profile placements blocked by non-skm entries; `status` and `doctor` report conflicts; `init --accept-existing-skills` for non-empty agent dirs off-TTY

### Changed

- `skm init` and `skm switch-agent` accept the three new agent ids
- **`skm init`** — prompts when the agent skills directory is not empty; refuses off-TTY unless `--accept-existing-skills`
- **`skm sync` / `use-profile`** — skip conflicted placement names instead of failing; other skills still link
- **`skm status`** — Linked and Conflicts sections; JSON includes `conflicts` with `reason: "conflicted"`
- **`skm doctor`** — `link.conflict` info when a profile skill is blocked by a foreign entry
- **`skm scan` / `skm sync`** — adopt on-disk skills that have no provenance meta (writes `.skm/meta/<bundle>.toml` or `<id>.toml` with `source_type = "store"`, `transfer = "adopted"`; never overwrites existing meta from `skm import`)
- **`--json`** on unsupported commands exits 2 with a clear error (no silent ignore)
- **`--dry-run`** cannot be combined with `--json`
- `skm --help` long description lists all shipped agents

### Dependencies

- `serde_json` — JSON output for `--json` commands

[0.2.0]: https://github.com/isaryx/skill-manager/releases/tag/v0.2.0

## [0.1.0] - 2026-08-28

First public release of `skm` — one skill library, named profiles, and symlink-based installs for AI agent folders.

### Added

- **`skm init`** — set up a skill store and project config in one step
- **`skm import`** — import a skill or nested skill tree with `--copy` or `--move`; rename with `--as` (`skm add` is an alias)
- **`skm skill ls` / `setup` / `rm`** — list, enable/disable, or permanently remove library skills (`rm` also updates profiles; `--force` required when not on a TTY)
- **`skm ls`** — list skills and profiles (`skill/…`, `profile/…`); `-s` / `--skill` or `-p` / `--profile` to filter
- **`skm profile setup` / `ls` / `show` / `rm`** — create and manage profiles
- **`skm use-profile`** — activate a profile and link its skills into the agent folder
- **`skm sync`** — refresh skill links without changing the active profile
- **`skm status`** — show target agent, active profile, and linked skills
- **`skm scan`** — rescan the skill store when files change on disk
- **`skm switch-agent`** — switch between `generic`, `claude-code`, and `cursor`
- **Store override** — `--store` or `SKM_STORE`
- **User-level config** — `~/.skm.toml` via `--user` / `-u` on supported commands
- **Interactive prompts** — store location, agent, library skills, and profile skills on a TTY
- **Colored output** — highlighted `status`; sync shows `+`/`-` like `git diff`
- **Safe sync** — manually installed skills in the agent folder are left untouched
- **MIT license** — see [LICENSE](LICENSE)

[0.1.0]: https://github.com/isaryx/skill-manager/releases/tag/v0.1.0
