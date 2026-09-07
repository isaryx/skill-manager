# Spec: Remote skill sources (GitHub)

**Status:** Draft · **Target:** 0.5.0 · **Architecture:** [DESIGN.md](DESIGN.md) · **CLI:** [SPEC.md](SPEC.md)

Register GitHub repositories as skill sources. Clone into the store, discover skills under standard paths, record provenance in meta, and refresh from upstream on `update` / `sync`. Remote skills appear in the library immediately but are not wired to agent directories until added to a profile.

Aligned with [myskills](https://github.com/jverhoeks/myskills) for URL shorthand, discovery paths, and opt-in activation — adapted to `skm`'s store + profile model.

---

## Assumptions

1. **Git is on `PATH`** for clone/pull; missing `git` is a clear error on remote commands.
2. **HTTPS clone URLs** by default (`owner/repo` → `https://github.com/owner/repo.git`). SSH URLs pass through unchanged.
3. **Library entries are symlinks** into the checkout (not copies). Content updates flow through without re-wiring agent links.
4. **No tokens in store files** — auth is via SSH keys, `GITHUB_TOKEN`, or `gh` (same as myskills).
5. **Remote skills are read-only** through the library symlink. Local edits happen via `skm import` or a future fork flow.
6. **`skm scan` does not `git pull`** — pull is `update` / `sync` only.

---

## Objective

| Goal | Success |
|------|---------|
| Add a GitHub repo | `skm repo add jverhoeks/myskills` clones, discovers skills, writes meta |
| Library visibility | New skills appear in `skm ls` / `skm search` |
| No auto-wiring | Agent dirs unchanged until profile + `sync` |
| Stay current | `skm update` / `skm sync` pull upstream; existing links pick up content changes |
| Resilience | One repo's pull failure warns and skips; other repos and sync continue |

---

## On-disk layout

Remote state lives **inside the store** (not a separate XDG cache).

```
~/.skill-store/
├── .skm/
│   ├── remotes/myskills/              # git checkout (.git here)
│   │   ├── .git/
│   │   ├── skills/
│   │   │   └── deploy/SKILL.md
│   │   └── README.md
│   ├── repos/myskills.toml            # registry (url, commit, skills_root)
│   ├── meta/myskills.toml             # bundle provenance (source_type=remote)
│   ├── profiles/…
│   ├── disabled.toml
│   └── index.db
├── myskills/                          # library namespace (repo-qualified prefix)
│   └── deploy/  →  ../.skm/remotes/myskills/skills/deploy
└── docx/                              # local import (real files, no .git)
    └── SKILL.md
```

| Path | Role |
|------|------|
| `.skm/remotes/<name>/` | Git working copy; `git pull` target |
| `.skm/repos/<name>.toml` | Registry: canonical URL, recorded commit, skills root |
| `.skm/meta/<name>.toml` | Bundle provenance inherited by `name/*` skills |
| `<name>/<skill>/` | Library entry — symlink into checkout; skill id `name/skill` |

**Discovery walks the library surface** (`$STORE/` top-level and below). `.skm/` is skipped (unchanged). Scan does not walk `.skm/remotes/` directly.

---

## Local vs remote

From the library's perspective both look the same: a directory with `SKILL.md`. Origin is **metadata**, not `.git` in the skill folder.

| | Local (`skm import`) | Remote (`skm repo add`) |
|--|----------------------|-------------------------|
| Library path | `$STORE/<id>/` (real files) | `$STORE/<repo>/<skill>/` (symlink) |
| `.git` in skill dir | No | No |
| `.git` anywhere | No | Yes — `.skm/remotes/<repo>/.git` only |
| `source_type` | `local` / `local-bundle` | `remote` |
| `transfer` | `copy` / `move` | `clone` |
| `remote_url` | absent | set |
| `commit` | absent | set at add/update |
| `repo_name` | optional (`--repo` import) | always set |
| Update path | re-import manually | `skm update` / `skm sync` |

`skm import --repo` today sets `repo_name` but keeps `source_type = local` until replaced by a real remote registration.

---

## Skill discovery (standard paths)

Search the checkout root in order; use the **first** directory that contains at least one immediate child with `SKILL.md`:

| Order | Path |
|-------|------|
| 1 | `skills/` |
| 2 | `.agents/skills/` |
| 3 | `.cursor/skills/` |
| 4 | `.github/skills/` |
| 5 | `.claude/skills/` |
| 6 | `.copilot/skills/` |

**Root skill:** if `SKILL.md` exists at the checkout root (and no multi-skill dir matched first), the repo is a single-skill source. Library id is `<repo-name>` (the repo registry name), not a nested path.

Nested skills under the discovered root use existing `discover_all_skill_dirs` semantics. Library id: `<repo-name>/<path-within-root>` (e.g. `myskills/deploy`, `myskills/nested/foo`).

Port discovery rules from [myskills `internal/repo`](https://github.com/jverhoeks/myskills/blob/main/internal/repo/repo.go).

---

## Registry and meta schemas

### `.skm/repos/<name>.toml`

```toml
version = 1
name = "myskills"
url = "https://github.com/jverhoeks/myskills.git"
checkout = "remotes/myskills"          # relative to .skm/
commit = "abc123…"                      # HEAD after last successful pull
skills_root = "skills"                  # "" when root SKILL.md
cloned_at = "2026-09-07T10:00:00Z"
updated_at = "2026-09-07T10:00:00Z"
```

### `.skm/meta/<name>.toml` (bundle)

```toml
source_type = "remote"
repo_name = "myskills"
remote_url = "https://github.com/jverhoeks/myskills.git"
path = "/abs/path/.skill-store/.skm/remotes/myskills"
commit = "abc123…"
hash = "…"                              # hash of checkout tree at registration
imported_at = "2026-09-07T10:00:00Z"
transfer = "clone"
```

Per-skill meta files are not required when bundle meta covers `name/*` (same as local bundles today).

---

## Commands

Global flags unchanged (`--store`, `--verbose`, etc.). Remote commands are **store commands** (no `./.skm.toml` required).

### `skm repo add <ref> [--name NAME]`

Register and clone a GitHub repository.

**Arguments**

- `<ref>` — `owner/repo` or full git URL (HTTPS/SSH/git@).

**Flags**

| Flag | Notes |
|------|-------|
| `--name <name>` | Registry + library prefix (default: last URL segment, e.g. `myskills`) |
| `--strict` | Fail if any discovered skill has invalid `SKILL.md` frontmatter (default: warn) |

**Behavior**

1. Resolve URL: `owner/repo` → `https://github.com/owner/repo.git`.
2. Derive `name` (validate with `validate_store_entry_name`).
3. **Reject duplicates:**
   - `repo "myskills" already registered`
   - `URL already registered as "other-name"` (normalize URL: strip `.git`, lowercase host)
4. `git clone <url> $STORE/.skm/remotes/<name>/`
5. Discover skills (standard paths + root `SKILL.md`).
6. Write `.skm/repos/<name>.toml` and `.skm/meta/<name>.toml`.
7. Create library symlinks: `$STORE/<name>/<skill>/` → checkout path (relative symlink preferred).
8. `refresh_store_index` (rebuild index).
9. **Do not** modify profiles, `disabled.toml`, or agent symlinks.

**Stdout:** one skill id per line (same as `skm import` tree).

**Exit:** 0 success; 1 I/O or git failure; 2 usage/duplicate/invalid name.

### `skm repo ls [--json]`

List registered remotes. Human: `name`, `url`, `commit` (short), skill count. JSON: `{ "repos": [{ "name", "url", "commit", "skills_root", "skill_count", "updated_at" }] }`.

### `skm repo rm <name> [--force]`

Remove a registered remote: delete registry, checkout, library symlinks under `<name>/`, bundle meta. Refuse if any profile references a skill under that prefix unless `--force` (lists affected profiles). TTY confirm default no. **Deferred to 0.5.1** if needed to ship add/update/sync first.

### `skm update [name]`

Pull-only: refresh remote checkouts **without** changing agent symlinks.

**Behavior**

1. For each registered repo (or one `name`):
   - `git pull --ff-only` in `.skm/remotes/<name>/`
   - On failure: `warning: pull failed for <name>: …` on stderr; **continue** other repos
   - On success: refresh library symlinks (add/remove/repoint), update `commit` / `updated_at` in registry + meta, update bundle `hash`
2. `refresh_store_index`

**Does not** call `reconcile()` (no agent dir changes).

**Flags:** `--dry-run` — print planned pulls and symlink changes; no git or writes.

### `skm sync` (changed)

Existing project command; add a **pull phase** before reconcile:

```
sync:
  1. pull_remotes()          # same per-repo logic as update (warn, continue)
  2. refresh_library_links() # per successfully pulled repo
  3. reconcile()             # unchanged: index, exclude, agent symlinks
```

**Flag:** `--no-pull` — skip steps 1–2 (offline / pinned workflow).

`add-profile`, `remove-profile`, `use-profiles` inherit pull-via-sync (they call reconcile).

### URL resolution

```rust
// owner/repo → https://github.com/owner/repo.git
// https://github.com/o/r.git → unchanged
// git@github.com:o/r.git → unchanged
```

`NameFromURL`: `owner/repo` → `repo`; URL → basename without `.git`.

---

## Library symlink refresh

Run after successful pull (in `update` and `sync`). For each repo:

| Checkout state | Action |
|----------------|--------|
| New skill dir with `SKILL.md` | Create `$STORE/<repo>/<id>/` symlink |
| Skill removed upstream | Remove library symlink; leave profile refs (doctor warns) |
| Skill path unchanged | Keep symlink (content already live) |
| `skills_root` changed (rare) | Re-run discovery; treat as add/remove |

Symlinks must resolve inside the store checkout. `is_store_owned_symlink` already requires target under canonical store root — verify checkout paths satisfy this.

---

## Scan and index

| Command | Pull | Library symlinks | Agent symlinks | Index |
|---------|------|------------------|----------------|-------|
| `skm scan` | No | No | No | Rebuild from disk + meta |
| `skm update` | Yes | Yes | No | Rebuild |
| `skm sync` | Yes (unless `--no-pull`) | Yes | Yes | Rebuild (via reconcile) |

`rebuild_from_store` reads `source_type` from meta — remote skills index as `remote`. Hash is computed through library symlinks (content hash of skill tree).

---

## Activation (unchanged)

Remote skills follow the same rules as local:

1. Visible in `skm ls` when not in `disabled.toml` (remote skills are **not** auto-disabled).
2. Wired to agents only when in an **active profile** and `sync` / `use-profiles` runs.
3. `skm skill setup` can disable library-wide.

---

## Doctor (new codes)

| Code | Sev | Condition |
|------|-----|-----------|
| `remote.pull_failed` | warn | Last pull for repo failed (store last-known-good commit) |
| `remote.checkout_missing` | error | Registry exists but `.skm/remotes/<name>/` missing |
| `remote.library_broken` | warn | Library symlink does not resolve |
| `remote.no_skills` | info | Registered repo, zero skills discovered |
| `profile.missing_ref` | error | (existing) profile refs skill removed upstream |

---

## Error handling

| Situation | Behavior |
|-----------|----------|
| Duplicate name / URL | Exit 2, clear message |
| `git clone` fails | Exit 1, no partial registry (or rollback clone dir) |
| `git pull --ff-only` fails | Warn, skip repo, continue |
| Clone OK, zero skills | Exit 0 with stderr warning; registry kept for retry after upstream fix |
| Library symlink target occupied by non-skm dir | Skip that skill, warn (same as reconcile foreign rule) |

---

## Done criteria

### `skm repo add`

- [ ] `owner/repo` shorthand resolves and clones correctly
- [ ] Full HTTPS and SSH URLs work
- [ ] Skills found for each standard directory layout (table above) and root `SKILL.md`
- [ ] Duplicate repo name or URL fails with clear error (exit 2)
- [ ] Skills appear in `skm ls` and `skm search` after add
- [ ] No agent symlinks created by `repo add`
- [ ] Meta: `source_type = remote`, `remote_url`, `repo_name`, `commit` set

### `skm update` / `skm sync` pull

- [ ] `sync` pulls remotes before reconcile (unless `--no-pull`)
- [ ] One failed pull does not stop other repos or agent reconcile
- [ ] `skm update` refreshes checkouts and library symlinks without agent changes
- [ ] Content change in existing skill visible after pull without re-sync
- [ ] New upstream skill appears in `skm ls` after update/sync
- [ ] `commit` in registry/meta updated after successful pull

### Integration

- [ ] `skm profile setup` lists remote skills; `add-profile` + `sync` wires agent symlinks
- [ ] `skm doctor` reports broken/missing remote state
- [ ] `skm status --json` shows remote skills with `source_type` / store id

---

## Out of scope (0.5.0)

- `skm repo rm` (optional 0.5.1)
- `skm import github:…` sugar (use `skm repo add`)
- Branch/tag pin, `skm repo pin`
- skills.sh browse UI
- Non-GitHub hosts (GitLab URL works if `git clone` works; no special UI)
- Windows release binary (separate 0.5.0 item)
- Copy-mode remote import (symlinks only)

---

## Implementation plan

Vertical slices; each slice lands tests + docs. Do not advance until tests pass.

### Phase 1 — Foundation (no CLI)

**Goal:** Types, URL parse, discovery, paths.

| Task | Module | Notes |
|------|--------|-------|
| 1.1 | `store/remote/url.rs` | `resolve_url`, `name_from_url`, `normalize_url` |
| 1.2 | `store/remote/discover.rs` | `find_skills_root`, `list_repo_skills`, root-skill case |
| 1.3 | `store/remote/paths.rs` | `remotes_dir`, `repo_registry_file`, `checkout_path` |
| 1.4 | `config.rs` | `RepoRegistration` struct + (de)serialize |
| 1.5 | Unit tests | URL + discovery fixtures per layout |

### Phase 2 — `skm repo add`

**Goal:** Register, clone, symlink, meta, index.

| Task | Module | Notes |
|------|--------|-------|
| 2.1 | `store/remote/git.rs` | `clone_repo`, `pull_ff_only` (shell `git`) |
| 2.2 | `store/remote/registry.rs` | read/write/list, duplicate detection |
| 2.3 | `store/remote/link.rs` | `install_library_symlinks`, `refresh_library_symlinks` |
| 2.4 | `store/remote/register.rs` | orchestrate add flow |
| 2.5 | `cli/repo.rs` + `cli/mod.rs` | `repo add`, `repo ls` |
| 2.6 | `error.rs` | `RepoAlreadyRegistered`, `GitCommandFailed`, … |
| 2.7 | Integration tests | `tests/remote_add.rs` — temp store, local git fixture repo |

### Phase 3 — `skm update`

**Goal:** Pull-only path.

| Task | Module | Notes |
|------|--------|-------|
| 3.1 | `store/remote/update.rs` | `pull_remotes`, per-repo warn/continue |
| 3.2 | `cli/update.rs` | `skm update [name]`, `--dry-run` |
| 3.3 | Integration tests | `tests/remote_update.rs` — amend remote, pull, assert hash |

### Phase 4 — `sync` integration

**Goal:** Pull before reconcile.

| Task | Module | Notes |
|------|--------|-------|
| 4.1 | `sync/mod.rs` | Call `pull_remotes` + `refresh_library_symlinks` before reconcile |
| 4.2 | `cli/sync.rs` | `--no-pull` flag |
| 4.3 | Integration tests | `tests/remote_sync.rs` — wired skill content updates without re-profile |

### Phase 5 — Doctor, docs, polish

| Task | Notes |
|------|-------|
| 5.1 | `doctor/checks.rs` — remote codes |
| 5.2 | `tests/doctor.rs` — remote fixtures |
| 5.3 | Update [SPEC.md](SPEC.md), [DESIGN.md](DESIGN.md), [CHANGELOG.md](../CHANGELOG.md) |
| 5.4 | Shell completions for `repo`, `update` |
| 5.5 | README quick-start example |

---

## Test strategy

| Level | Location | Coverage |
|-------|----------|----------|
| Unit | `store/remote/*` | URL parse, discovery order, root skill |
| Integration | `tests/remote_*.rs` | Local bare git repos as fixtures (`git init` + commit skills) |
| Integration | `tests/qualified_ids.rs` pattern | Repo-qualified ids coexist with local |
| Doctor | `tests/doctor.rs` | `remote.*` codes |

**Fixture pattern:** create bare/remote git repo in `TempDir`, `skm repo add` via file URL or local path (support `file://` or absolute path for tests). Production uses GitHub HTTPS.

**Skip if no git:** same as exclude tests — skip when `git` not on `PATH`.

---

## Module map (target)

```
src/store/remote/
  mod.rs          re-exports
  url.rs          URL shorthand
  discover.rs     standard path discovery
  paths.rs        store paths for remotes
  git.rs          clone, pull
  registry.rs     .skm/repos/*.toml
  link.rs         library symlinks
  register.rs     repo add orchestration
  update.rs       pull + refresh

src/cli/
  repo.rs         repo add, repo ls
  update.rs       update command
```

`reconcile()` stays the only agent symlink mutator. Remote pull is a pre-phase in `sync` and standalone in `update`.

---

## Example workflows

```bash
# Register upstream (library only)
skm repo add jverhoeks/myskills
skm ls -s
# myskills/dependency-bloat-reduction
# …

# Activate subset
skm profile setup work          # pick remote skills
skm add-profile work            # wires agent symlinks + pulls first

# Refresh content only
skm update

# Refresh content + re-wire (e.g. after upstream adds skills you added to profile)
skm sync
```

---

## References

- [myskills](https://github.com/jverhoeks/myskills) — URL shorthand, discovery, opt-in enable
- [SPEC.md](SPEC.md) — profiles, reconcile, scan, meta
- [DESIGN.md](DESIGN.md) — store layout, module layers
