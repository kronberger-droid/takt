# takt

A time tracker that encourages focusing on one thing at a time.

I work on a lot of different things at once — work, study, personal projects. takt tracks what I'm doing and for how long, with hierarchical tags and a human-readable storage format.

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

## v0.2 — Server mode & shared use

Make takt hostable so multiple users can track on a shared server.

Two cargo features:
- `local` (default) — current behavior, flat files, single user
- `server` — web API, sled database, multi-user

### Scope

| Area | Description |
|---|---|
| Storage trait | Abstract storage behind a trait so flat-file and sled backends plug in |
| sled backend | Implement storage trait with sled for concurrent access |
| Web API (axum) | REST endpoints: `POST /start`, `POST /stop`, `GET /status`, `GET /report`, `GET /tags`, `POST /tags` |
| Auth | Simple token-based auth, one token per user |
| Multi-user | Separate data per user, keyed by user ID in sled |
| CLI server mode | `takt --server http://host:port` — CLI sends HTTP instead of local file access |
| Local fallback | Queue actions locally when server is unreachable, sync on reconnect |
| `takt serve` | New subcommand to start the server |

### Out of scope for v0.2

- Real auth (OAuth, sessions)
- TUI
- Sync conflict resolution (last-write-wins is fine)
