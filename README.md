# skill-manager (`skm`)

**skm** is a command-line tool for managing AI agent skills across projects and editors.

Keep skills in one local library, organize them into named profiles, and link them into agent folders (Claude Code, Cursor, generic for Codex/Cursor/Gemini CLI/Copilot CLI, and more). Run `skm sync` any time to refresh those links.

## How it works

| Piece | What it does |
|-------|----------------|
| **Skill library** | Canonical copies of your skills (one folder per skill under the store) |
| **Profile** | A named set of skills to activate together (`work`, `personal`, …) |
| **Sync** | Creates symlinks from the active profile into the agent’s skills directory |

**Library vs profile:** `skm skill setup` controls which skills appear in your library (disable without deleting). `skm profile setup` picks which library skills belong to a profile.

A skill is a directory with a `SKILL.md` file at its root (or nested under a skill tree).

## Install

**Platforms:** macOS and Linux (symlink-based installs). Windows is not supported yet (planned for 0.3.0 — see [docs/SPEC.md](docs/SPEC.md)).

**Homebrew** ([isaryx/collection](https://github.com/isaryx/homebrew-collection)):

```bash
brew install isaryx/collection/skm
```

**From a release** — download `skm` for your platform from [GitHub Releases](https://github.com/isaryx/skill-manager/releases) (`macos-arm64`, `macos-x86_64`, `linux-arm64`, or `linux-x86_64`), extract the binary, and put it on your `PATH`.

**From source** (requires [Rust](https://rustup.rs)):

```bash
git clone https://github.com/isaryx/skill-manager.git
cd skill-manager
cargo install --path .
```

## Quick start

```bash
skm init --agent claude-code
skm import ./my-skill --copy
skm profile setup work
skm use-profile work
skm status
```

`skm import` is also available as `skm add`.

## Project and hand-installed skills

Repositories often ship skills under `.claude/skills/`, `.cursor/skills/`, and similar paths. **skm does not delete or overwrite skills it did not place.** It only manages symlinks whose targets live inside your skill store.

| Situation | What skm does |
|-----------|----------------|
| Skill in the agent folder, **not** in your active profile | Left alone |
| Profile wants a name already taken by a project or hand-installed skill | Skips that placement; other skills still link |
| You run `skm init` on a project that already has agent skills | Prompts on a TTY; use `--accept-existing-skills` in scripts |

Check results with `skm status` (Linked and **Conflicts** sections) or `skm doctor` (`link.conflict` is informational — exit 0).

## Common workflows

**Import from outside the store**

```bash
skm import ./path/to/skill --copy          # or --move
skm import ./skill-tree --copy --as local  # nested skills under one bundle name
```

**Copy skills into the store yourself**

If you place skill folders directly under the store (for example `cp -r ./local ~/.skill-store/local`), run `skm scan` to refresh the index and register them. Existing import metadata is never overwritten.

**Check health**

```bash
skm doctor           # human-readable report; exit 1 on warnings/errors
skm doctor --json    # for scripts (includes link.conflict when a profile skill is blocked)
```

## Commands

| Command | Description |
|---------|-------------|
| `skm init` | Set up the skill store and write `./.skm.toml` (`--accept-existing-skills` when the agent folder already has skills) |
| `skm import <dir> --copy\|--move` | Import a skill or nested skill tree into the library |
| `skm ls` | List skills and profiles (`-s`/`--skill` or `-p`/`--profile` to filter) |
| `skm skill ls` / `setup` / `rm` | List, enable/disable, or remove skills in the library |
| `skm profile setup/ls/show/rm` | Create and manage profiles |
| `skm use-profile <profile>` | Activate a profile and sync links to the agent folder |
| `skm switch-agent` | Change the target agent in your config |
| `skm sync` | Refresh skill links and index without changing the active profile |
| `skm status` | Show target agent, active profile, linked skills, and placement conflicts |
| `skm doctor` | Health report for store, profiles, and links |
| `skm scan` | Refresh the skill index and adopt skills added to the store without metadata |

Global flags: `--verbose` / `-v`, `--store <path>` (env: `SKM_STORE`), `--json` (on `status`, `ls`, `skill ls`, `doctor`), `--dry-run` (on `sync`, `use-profile`, `skill rm`), `--color auto|always|never`.

Many commands accept `--user` / `-u` to use `~/.skm.toml` instead of `./.skm.toml`.

**Preview changes**

```bash
skm sync --dry-run
skm use-profile work --dry-run
skm skill rm docx --dry-run
```

## Shell completions

Regenerate after CLI changes:

```bash
cargo run --example generate-completions
```

Install (bash example):

```bash
source completions/skm.bash
```

Files: `completions/skm.bash`, `completions/_skm` (zsh), `completions/skm.fish`.

## Configuration

| File | Purpose |
|------|---------|
| `~/.config/skm/config.toml` | App config: skill store path (`[store].path`) |
| `./.skm.toml` or `~/.skm.toml` | Project or user config: target agent, active profile |
| `$STORE/.skm/disabled.toml` | Library skills you have hidden (optional) |

Store path resolution (first match wins): `--store` → `SKM_STORE` → app config → `~/.skill-store`.

### Supported agents

| Agent | Skills directory |
|-------|------------------|
| `generic` | `.agents/skills` (Codex, Cursor, Gemini CLI, Copilot CLI) |
| `claude-code` | `.claude/skills` |
| `cursor` | `.cursor/skills` |
| `gemini-cli` | `.gemini/skills` |
| `copilot-cli` | `.github/skills` |

Each config file targets **one** agent via `placement.agent`.

## Documentation

- [CHANGELOG.md](CHANGELOG.md) — release history
- [docs/SPEC.md](docs/SPEC.md) — command reference
- [docs/DESIGN.md](docs/DESIGN.md) — architecture
- [docs/SPEC-AGENTS.md](docs/SPEC-AGENTS.md) — agent paths

## Development

The `.agents/skills/` directory in this repo holds agent skills used while building skm (optional; not required to use the tool).

```bash
cargo build
cargo test
cargo clippy -- -D warnings
cargo fmt --check
```

## License

MIT — see [LICENSE](LICENSE).
