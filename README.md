# takt

A time tracker that encourages focusing on one thing at a time.

I work on a lot of different things at once — work, study, personal projects. takt tracks what I'm doing and for how long, with hierarchical tags and a human-readable storage format.

## Install

From [crates.io](https://crates.io/crates/takt):

```
cargo install takt
```

Via [Nix](https://nixos.org) (uses the repo's flake):

```
nix run github:kronberger-droid/takt
```

Prebuilt binaries for Linux (x86_64, aarch64), macOS (x86_64, aarch64), and Windows (x86_64) are attached to each [GitHub Release](https://github.com/kronberger-droid/takt/releases/latest).

## Data format

### Tags (`~/.local/share/takt/tags`)

Indentation-based hierarchy:

```
work
  project-x
    implement-api
    fix-bug
  project-y
study
  math
    linear-algebra
  rust
```

Tags can be referenced by their leaf name if unambiguous (e.g. `fix-bug` resolves to `work/project-x/fix-bug`).

### Log (`~/.local/share/takt/log/YYYY-MM.takt`)

One file per month, human-readable:

```
2026-04-06 09:15:04 -- 2026-04-06 11:30:10 | work/project-x/fix-bug
2026-04-06 11:35:02 -- *                    | study/math/linear-algebra
```

`*` means the task is still running.

## v0.1 — Local CLI

Core tracking with hierarchical tags and human-readable storage.

### Commands

| Command | Description |
|---|---|
| `takt start <tag>` | Start tracking (auto-stops active task) |
| `takt stop` | Stop the current task |
| `takt status` | Show what's running and for how long |
| `takt tag add <path>` | Add a tag (e.g. `work/project-x/new-task`) |
| `takt tag list` | Show the tag tree |
| `takt report this <day\|week\|month>` | Time per tag for the current period |
| `takt report last <n> <day\|week\|month>` | Time per tag for the last N units |

Tags resolve by leaf name when unambiguous, so `takt start fix-bug` works if `fix-bug` appears once in the tree. Ambiguous names show all matching paths; unknown names error out.

Report spans that cross monthly log files are stitched automatically — `takt report last 3 month` reads all four month files and warns on stderr for any that are missing.

### Progress

- [x] CLI skeleton (clap)
- [x] Tag tree — parse, write, resolve, add
- [x] Log format — parse, write, load, save
- [x] Tracking — start, stop
- [x] Wire into CLI — tag add, tag list, start, stop
- [x] Status command
- [x] Reporting — aggregate by tag, time range
- [x] Error handling for malformed tag files

## v0.2 — Storage abstraction (pure refactor)

No user-visible changes. Introduces a `Store` trait with the verbs the CLI needs (`start`, `stop`, `active`, `entries_between`, `tag_add`, `tag_list`, `tag_resolve`) and migrates the CLI to go through it. The only implementation is `FlatStore`, wrapping the current file-backed logic.

Sets the stage for v0.3 to add database-backed and remote implementations without changing the CLI layer.

## v0.3 — Server mode & shared use

Make takt hostable so multiple users can track on a shared server. A single binary contains both client and server code; mode is chosen at runtime, not at compile time (so a desktop can double as its own server).

### Scope

| Area | Description |
|---|---|
| SQLite backend | `SqliteStore` implementation of the `Store` trait via `rusqlite` |
| Web API (axum) | REST endpoints: `POST /start`, `POST /stop`, `GET /status`, `GET /report`, `GET /tags`, `POST /tags` |
| Auth | Simple token-based auth, one token per user |
| Multi-user | Per-user data in SQLite, keyed by user id |
| `takt serve` | New subcommand to start the server |

## v0.4 — Remote client

| Area | Description |
|---|---|
| CLI server mode | `takt --server http://host:port` — CLI uses `RemoteStore` (HTTP) instead of local files |
| Config | Persistent `~/.config/takt/config.toml` for server URL + token |
| Local fallback | Queue actions locally when server is unreachable, sync on reconnect |

### Out of scope for v0.2

- Real auth (OAuth, sessions)
- TUI
- Sync conflict resolution (last-write-wins is fine)
