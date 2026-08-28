# Agent adapters

Each `placement.agent` in `.skm.toml` maps to a skills directory. All adapters use flat symlink names, absolute links into the store, and one agent per config file.

## Shipped (0.2.0)

| Agent | Project | User |
|-------|---------|------|
| `generic` | `.agents/skills` | `~/.agents/skills` |
| `claude-code` | `.claude/skills` | `~/.claude/skills` |
| `cursor` | `.cursor/skills` | `~/.cursor/skills` |
| `codex` | `.agents/skills` | `~/.agents/skills` |
| `gemini-cli` | `.gemini/skills` | `~/.gemini/skills` |
| `copilot-cli` | `.github/skills` | `~/.copilot/skills` |

`generic` and `codex` share paths ([Agent Skills](https://agentskills.io) layout). `copilot-cli` uses `.github/skills` at project level (not `.agents/`).

## Planned tiers

**Tier 2** (0.3.0+): `windsurf`, `cline`, `opencode`, `goose`, `roo-code`, `openclaw` — see source repo docs before shipping.

**Tier 3** (on demand): `kilocode`, `aider`, `amazon-q`, `augment`, `tabnine`, `sourcegraph-cody`, `antigravity`, `pi`.

## Interoperability

Several agents read `.agents/skills` as an alias (Codex, Gemini CLI, Copilot CLI). Claude and Cursor use their native dirs only. Copilot still needs the `copilot-cli` adapter for correct project-level `.github/skills` symlinks.

## References

- [Agent Skills](https://agentskills.io)
- [Copilot CLI skills](https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-skills)
- [Gemini CLI skills](https://geminicli.com/docs/cli/skills/)
