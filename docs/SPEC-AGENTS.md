# Agent adapters

Each `placement.agent` in `.skm.toml` maps to a skills directory. All adapters use flat symlink names, absolute links into the store, and one agent per config file.

## Shipped (0.2.1)

| Agent | Project | User |
|-------|---------|------|
| `generic` | `.agents/skills` | `~/.agents/skills` |
| `claude-code` | `.claude/skills` | `~/.claude/skills` |
| `cursor` | `.cursor/skills` | `~/.cursor/skills` |
| `gemini-cli` | `.gemini/skills` | `~/.gemini/skills` |
| `copilot-cli` | `.github/skills` | `~/.copilot/skills` |

`generic` is the [Agent Skills](https://agentskills.io) layout (`.agents/skills` / `~/.agents/skills`):

| Client | Relationship |
|--------|----------------|
| **Codex** | Native path ([docs](https://developers.openai.com/codex/skills)) |
| **Cursor** | Interoperable alias; native path is `.cursor/skills` ([docs](https://cursor.com/docs/skills)) |
| **Gemini CLI** | Interoperable alias; native path is `.gemini/skills` ([docs](https://geminicli.com/docs/cli/skills/)) |
| **Copilot CLI** | Interoperable alias; native project path is `.github/skills` ([docs](https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-skills)) |

Use `generic` when you want one shared directory for these tools. Use `cursor`, `gemini-cli`, or `copilot-cli` adapters when you only want that agent's native path. Legacy configs with `placement.agent = "codex"` are accepted as an alias for `generic`.

## Planned tiers

**Tier 2** (0.3.0+): `windsurf`, `cline`, `opencode`, `goose`, `roo-code`, `openclaw` — see source repo docs before shipping.

**Tier 3** (on demand): `kilocode`, `aider`, `amazon-q`, `augment`, `tabnine`, `sourcegraph-cody`, `antigravity`, `pi`.

## Interoperability

Codex, Cursor, Gemini CLI, and Copilot CLI all document `.agents/skills` (see table above). Claude Code uses its native dir only (`claude-code` adapter). Many other [Agent Skills clients](https://agentskills.io/clients) may also read `.agents/skills`; `generic` targets that shared layout.

## References

- [Agent Skills](https://agentskills.io)
- [Cursor skills](https://cursor.com/docs/skills)
- [Copilot CLI skills](https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-skills)
- [Gemini CLI skills](https://geminicli.com/docs/cli/skills/)
