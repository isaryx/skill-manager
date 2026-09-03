# skill-manager (`skm`)

**skm** keeps canonical skill directories in one local store, groups skill IDs into named profiles, and creates symlinks in agent-specific directories (Claude Code, Cursor, or the generic Agent Skills layout). Run `skm sync` to reconcile those links.

## How it works

| Piece | What it does |
|-------|----------------|
| **Store** | Canonical copies of your skills (one folder per skill) |
| **Profile** | A named set of skills to activate together (`work`, `personal`, …) |
| **Sync** | Creates symlinks from the active profile into every target agent’s skills directory |

**Store vs profile:** `skm skill setup` controls which store skills are enabled (disable without deleting). `skm profile setup` picks which enabled skills belong to a profile.

Both open a full-screen picker: `/` to search, `space` to toggle, `enter` to confirm, `q` to quit. Arrow keys and `k`/`j` both move. Keys are listed in the bar at the bottom of the screen.

**Profiles can extend other profiles.** `skm profile extend work` picks the profiles `work` inherits from; its skill list is the union, flattened when you sync. Editing a base profile updates everything extending it. `skm profile show` marks where each skill came from:

```
docx
git (from base)
```

`skm profile show work --tree` shows the whole graph, including the path each skill arrived by:

```
work
├── pdf
├── base
│   ├── docx
│   └── shared
│       └── git
└── infra
    ├── tf
    └── shared (*)

4 skills resolved
```

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
skm init --agent claude-code           # or --agent claude-code,cursor for several
skm import ./my-skill --copy
skm profile setup work
skm use-profile work
skm status
```

## Project and hand-installed skills

Repositories often ship skills under `.claude/skills/`, `.cursor/skills/`, and similar paths. **skm does not delete or overwrite skills it did not place.** It only manages symlinks whose targets live inside your skill store.

In Git projects, skm keeps its own links out of `git add` through a managed block in the clone-local
`.git/info/exclude`; it never edits a project `.gitignore`. This is on by default. Set
`ignore_links = false` under `[placement]` in `.skm.toml` to opt out.

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
| `skm init` | Set up the skill store and write `./.skm.toml`. Refuses if that file already exists (`setup-agents` / `use-profile`); `--force` overwrites. `--accept-existing-skills` when the agent folder already has skills |
| `skm import <dir> --copy\|--move` | Import a skill or nested skill tree into the store |
| `skm ls` | List skills and profiles (`-s`/`--skill` or `-p`/`--profile` to filter) |
| `skm skill ls` / `setup` / `rm` | List, enable/disable, or remove skills in the store |
| `skm profile setup/ls/show/rm` | Create and manage profiles |
| `skm profile extend <profile>` | Pick which profiles this one inherits skills from (creates the profile if missing) |
| `skm use-profile [profile]` | Activate a profile and sync links to every target agent folder; omit the name to choose interactively (`./.skm.toml` unless `--user`) |
| `skm setup-agents` | Choose which agents your config places skills into (checkbox list; `switch-agent` still works; `./.skm.toml` unless `--user`) |
| `skm destroy` | Remove `./.skm.toml`, store-owned links in every known project agent dir, and the managed git exclude (store kept; `--force` off-TTY) |
| `skm sync` | Refresh skill links and index without changing the active profile (`./.skm.toml` unless `--user`) |
| `skm status` | Show target agents, active profile, linked skills, and name conflicts (`./.skm.toml` unless `--user`) |
| `skm doctor` | Health report for store, profiles, and links |
| `skm scan` | Refresh the skill index and adopt skills added to the store without metadata |

Global flags: `--verbose` / `-v`, `--store <path>` (env: `SKM_STORE`), `--json` (on `status`, `ls`, `skill ls`, `doctor`), `--dry-run` (on `sync`, `use-profile`, `skill rm`, `destroy`), `--color auto|always|never`.

Many commands accept `--user` / `-u` to use `~/.skm.toml` instead of `./.skm.toml`.

### Scripting and CI

- Set `SKM_STORE` or pass `--store <path>` to select the store without a prompt.
- Pass `--agent` to `skm init` and `skm setup-agents`; repeat it or comma-separate for several agents (`--agent claude-code,cursor`). If a target directory already contains skills, also pass `--accept-existing-skills`.
- Use `--json` with `status`, `ls`, `skill ls`, and `doctor`. Structured data stays on stdout; progress and errors go to stderr.
- Use `--dry-run` before `sync`, `use-profile`, or `skill rm`. Non-interactive `skill rm` also requires `--force`.
- Exit codes are `0` for success, `1` for runtime or health-check failure, and `2` for invalid usage or resolution conflicts.

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
| `./.skm.toml` or `~/.skm.toml` | Project or user config: target agents, active profile |
| `$STORE/.skm/disabled.toml` | Store skills you have hidden (optional) |

Store path resolution (first match wins): `--store` → `SKM_STORE` → app config → `~/.skill-store`.

### Supported agents

| Agent | Skills directory |
|-------|------------------|
| `generic` | `.agents/skills` (Codex, Cursor, Gemini CLI, Copilot CLI) |
| `claude-code` | `.claude/skills` |
| `cursor` | `.cursor/skills` |
| `gemini-cli` | `.gemini/skills` |
| `copilot-cli` | `.github/skills` (project); `~/.copilot/skills` (`--user`) |

A config file targets one or more agents via `placement.agents`:

```toml
[placement]
agents = ["claude-code", "cursor"]
```

Every target agent gets its own symlinks, so the same profile can serve several tools at once. Run `skm setup-agents` to change the set — agents you uncheck have their store-owned links removed. Setups written before multi-agent support (`agent = "claude-code"`) are still read, and rewritten as a list the next time skm writes the file. Project vs user paths are listed in [docs/SPEC-AGENTS.md](docs/SPEC-AGENTS.md).

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
