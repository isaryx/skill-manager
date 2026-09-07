# skill-manager (`skm`)

**skm** manages a local library of [Agent Skills](https://agentskills.io) and wires them into Claude Code, Cursor, and other tools via symlinks. You keep one canonical store, group skills into named profiles, and run `skm sync` to reconcile links.

Import skills from disk, **register git repositories** (GitHub `owner/repo`, GitLab, SSH), **browse [skills.sh](https://skills.sh)** to add popular repos, and **pull upstream** with `skm update` or `skm sync`.

A skill is a directory with a `SKILL.md` file at its root (or nested under a skill tree).

## Quick start

```bash
skm init --agent claude-code           # or --agent claude-code,cursor for several
skm import ./my-skill --copy
skm profile setup work
skm add-profile work
skm status
```

When setup succeeds, `skm status` shows target agents, active profiles, and linked skills:

```
Target agents:
  claude-code (.claude/skills)
Active profiles: work

Linked
  my-skill -> ~/.skill-store/my-skill
```

Most project commands read `./.skm.toml` in the current directory. Pass `--user` / `-u` to use `~/.skm.toml` instead.

## How it works

| Piece | What it does |
|-------|----------------|
| **Store** | Canonical skill library — local copies (`import`) or symlinks into git checkouts (`repo add`) |
| **Profile** | A named set of skills to activate together (`work`, `personal`, …) |
| **Sync** | Pulls remotes (optional), then symlinks active profile skills into every target agent directory |

**Store vs profile:** `skm skill setup` controls which store skills are enabled (disable without deleting). `skm profile setup` picks which enabled skills belong to a profile. Both open a full-screen picker; keys are shown in the bar at the bottom of the screen.

**Remote repos:** `skm repo add` clones into the store and symlinks skills into the library (`myskills/deploy`). They are not wired to agents until you add them to a profile and sync. Use `skm repo ls` to see registered remotes; `skm doctor` warns when a remote checkout has moved on (`remote.stale`).

## Install

macOS and Linux only (symlink-based). Windows is not supported yet — see [docs/SPEC.md](docs/SPEC.md).

**Homebrew** ([isaryx/collection](https://github.com/isaryx/homebrew-collection)):

```bash
brew install isaryx/collection/skm
```

**Install script** (macOS and Linux, `arm64` / `x86_64`; default install dir: `~/.local/bin`):

```bash
curl -fsSL https://raw.githubusercontent.com/isaryx/skill-manager/master/scripts/install.sh | bash
```

Pin a release with `SKM_VERSION=v0.5.1` or `--version v0.5.1`. Use `--install-dir` for a custom path and `--dry-run` to preview. Unsupported OS or architecture exits with a clear error.

Other install paths: download a binary from [GitHub Releases](https://github.com/isaryx/skill-manager/releases), build from source with `cargo install --path .`, or run `scripts/check-update.sh` to compare against the latest release. Details in [docs/SPEC.md](docs/SPEC.md).

## Common workflows

**Import from outside the store**

```bash
skm import ./path/to/skill --copy          # or --move
skm import github:owner/repo             # register a GitHub remote (no --copy/--move)
skm import ./skill-tree --copy --as local  # nested skills under one bundle name
```

**Copy skills into the store yourself**

If you place skill folders directly under the store (for example `cp -r ./local ~/.skill-store/local`), run `skm scan` to refresh the index and register them. Existing import metadata is never overwritten.

**Register or browse git skill repos**

```bash
skm repo add owner/repo                        # GitHub shorthand
skm repo add https://gitlab.com/g/skills.git --name skills
skm import github:owner/repo                   # same as repo add
skm repo browse                                # skills.sh leaderboard (TTY)
skm repo ls                                    # list registered remotes

skm profile setup work                         # pick remote skills (e.g. myskills/deploy)
skm add-profile work
skm update                                     # pull upstream without re-wiring agents
skm sync                                       # pull + refresh agent symlinks
skm sync --no-pull                             # offline: skip pull, reconcile only

skm repo pin myskills v1.0.0                   # optional: pin checkout
skm repo pin myskills --clear                  # track default branch again
skm repo rm myskills --force                   # remove when no longer needed
```

Requires `git` for `repo add`, `update`, and the pull phase of `sync`. See [docs/SPEC.md](docs/SPEC.md#git-remote-sources).

**Compose profiles**

Profiles can extend other profiles. `skm profile extend work` picks which profiles `work` inherits from; its skill list is the union, flattened when you sync:

```bash
skm profile extend work
skm profile show work --tree
```

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

**Check health**

```bash
skm doctor           # human-readable report; exit 1 on warnings/errors
skm doctor --json    # for scripts (link.conflict, remote.stale, …)
```

## Project and hand-installed skills

**skm does not delete or overwrite skills it did not place.** It only manages symlinks whose targets live inside your skill store. In Git projects, skm keeps its own links out of `git add` through a managed block in `.git/info/exclude` (never `.gitignore`). Set `ignore_links = false` under `[placement]` in `.skm.toml` to opt out.

| Situation | What skm does |
|-----------|----------------|
| Skill in the agent folder, **not** in your active profiles | Left alone |
| Profile wants a name already taken by a project or hand-installed skill | Skips that placement; other skills still link |
| You run `skm init` on a project that already has agent skills | Prompts on a TTY; use `--accept-existing-skills` in scripts |

Check results with `skm status` (Linked and **Conflicts** sections) or `skm doctor` (`link.conflict` is informational — exit 0).

## Commands

Grouped overview — every flag and exit code is in [docs/SPEC.md](docs/SPEC.md).

| Group | Commands |
|-------|----------|
| Setup | `init`, `destroy` |
| Store | `import`, `repo add/ls/rm/pin/browse`, `update`, `scan`, `search`, `ls`, `skill ls/setup/rm/validate` |
| Profiles | `profile setup/ls/show/rm/extend`, `use-profiles`, `add-profile`, `remove-profile` |
| Agents | `use-agents`, `add-agent`, `remove-agent` |
| Sync & status | `sync`, `status`, `doctor` |

Global flags: `--verbose` / `-v`, `--store <path>` (env: `SKM_STORE`), `--json`, `--dry-run`, `--color auto|always|never`. See SPEC for which commands accept each flag.

### Scripting and CI

- Set `SKM_STORE` or pass `--store <path>` to select the store without a prompt.
- Pass `--agent` to `skm init`; repeat it or comma-separate for several agents (`--agent claude-code,cursor`). Use `skm add-agent` / `skm remove-agent` to change the set after init. If a target directory already contains skills, also pass `--accept-existing-skills`.
- Use `--json` with `status`, `ls`, `search`, `skill ls`, `skill validate`, `doctor`, and `repo ls`. Structured data stays on stdout; progress and errors go to stderr.
- Use `--dry-run` before `sync`, `update`, `add-profile`, `remove-profile`, or `skill rm`. Non-interactive `skill rm` also requires `--force`.
- Exit codes: `0` success, `1` runtime or health-check failure, `2` invalid usage or resolution conflicts.

```bash
skm update --dry-run
skm sync --dry-run
skm add-profile work --dry-run
skm skill rm docx --dry-run
```

## Configuration

| File | Purpose |
|------|---------|
| `~/.config/skm/config.toml` | App config: skill store path (`[store].path`) |
| `./.skm.toml` or `~/.skm.toml` | Project or user config: target agents, active profiles |
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

```toml
[placement]
agents = ["claude-code", "cursor"]
```

Every target agent gets its own symlinks, so the same profile can serve several tools at once. Project vs user paths are in [docs/SPEC.md](docs/SPEC.md#agent-adapters).

## Documentation

| Doc | Use when you need |
|-----|-------------------|
| This README | Install, quick start, everyday workflows |
| [docs/SPEC.md](docs/SPEC.md) | Commands, agents, remotes, flags, exit codes |
| [docs/DESIGN.md](docs/DESIGN.md) | Architecture and design decisions |
| [CHANGELOG.md](CHANGELOG.md) | Release history |

## Development

The `.agents/skills/` directory in this repo holds agent skills used while building skm (optional; not required to use the tool).

```bash
cargo build
cargo test
cargo clippy -- -D warnings
cargo fmt --check
cargo run --example generate-completions   # writes completions/skm.{bash,fish} and completions/_skm
```

## License

MIT — see [LICENSE](LICENSE).
