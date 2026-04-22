# CLAUDE.md

Project-specific context for Claude. This file should be kept current so a
fresh Claude session on any machine can pick up where the last left off.

## What is takt

A time-tracking CLI with hierarchical tags and human-readable storage.
Shipped as v0.2.0 on crates.io and GitHub Releases. Repo:
<https://github.com/kronberger-droid/takt>.

## Architecture overview

- **`src/main.rs`** — CLI entry point. Builds a `Box<dyn Store>` and dispatches
  every command through the trait. Sync throughout; async is contained to the
  `serve` subcommand (spins up a tokio runtime just for that).
- **`src/model.rs`** — the shared `Entry` type every backend emits and consumes.
- **`src/store/mod.rs`** — defines `trait Store` (the CLI verbs: start, stop,
  active, entries_between, tag_add, tag_list, tag_resolve), plus a shared
  `test_harness` module and `store_tests!` macro that every backend runs.
- **`src/store/flat.rs`** — `FlatStore`, the v0.1 file-backed implementation.
  Still the only backend the CLI currently uses.
- **`src/store/sqlite.rs`** — `SqliteStore` via `rusqlite`, scoped to a single
  `user_id` passed at construction. Not yet wired into the CLI; will be used by
  the server once v0.3 Phase 2 is further along.
- **`src/server/mod.rs`** — axum-based HTTP server for `takt serve`. Currently
  scaffolded with an empty router; no handlers yet.
- **`src/log.rs`, `src/tags.rs`** — file-format details, `pub(crate)` to keep
  them private implementation details of `FlatStore`.
- **`src/report.rs`** — report generation, operates on `&[Entry]`. Backend-agnostic.
- **`src/error.rs`** — central `TaktError` enum with `#[from]` for io,
  rusqlite, and chrono errors.

### Dependencies worth knowing

- `clap 4` (derive) — CLI
- `chrono` — datetimes (we use `NaiveDateTime` as UTC-for-timestamp-conversion)
- `rusqlite 0.32` (`bundled` feature — ships SQLite statically)
- `axum 0.8` + `tokio 1` — server (runtime only, not async CLI)
- `thiserror 2` — error derive
- `tempfile` (dev) — isolated dirs for FlatStore tests

## Release status

- **v0.1.0** — unshipped (deleted tag; CI issue). Don't re-publish this version.
- **v0.2.0** — shipped (crates.io, GitHub Releases, 4 prebuilt binaries). First
  public release. Includes the `Store` trait + `FlatStore` (v0.3 Phase 1 was
  merged pre-release).
- **v0.3.0** (in progress) — server mode. Three phases:
  - Phase 1: SqliteStore as a second `Store` impl — **✅ done** (merged on main
    but not yet released). All 48 tests pass, both backends exercise the same
    shared-harness test suite.
  - Phase 2: `takt serve` (axum), bearer-token auth, JSON endpoints — **in
    progress**. Scaffolding done, empty router responds 404 on bind port.
  - Phase 3: `takt user-add` CLI command for server admin.
- **v0.4.0** (planned) — `takt --server <url>` mode. `RemoteStore` (a third
  `Store` impl) that talks HTTP to the server. CLI stays sync; RemoteStore
  uses a sync HTTP client (likely `ureq`) or a blocking `reqwest` to hide
  async internally.

## Where we are in v0.3 Phase 2

Phase 2 is ordered in layers that each compile and run independently. Current
state and next step:

1. **Empty async scaffolding — ✅ done.** `takt serve --port 8080` binds,
   responds 404, shuts down cleanly on Ctrl+C or SIGTERM. The `server::run`
   function is async; `main.rs` calls it via `tokio::runtime::Runtime::new()?
   .block_on(...)` so only this subcommand touches async.
2. **One endpoint (no auth) → `GET /status` — ⏳ in progress.** Boilerplate
   is wired: `server::run` accepts a DB path, creates `Arc<Mutex<SqliteStore>>`
   with a default user, routes `GET /status` to `status_handler`. `serde` +
   `serde_json` added to deps, `Entry` derives `Serialize`, chrono has its
   `serde` feature enabled. **What's left:** define `StatusResponse` fields
   and implement `status_handler` body (both marked `TODO` in
   `src/server/mod.rs`).
3. **Bearer-token middleware.** Add a middleware layer that reads
   `Authorization: Bearer <token>`, looks up the user via
   `SELECT id FROM users WHERE token = ?`, and attaches `user_id` to request
   extensions. Handlers then read `user_id` from extensions and scope their
   `SqliteStore::new(..., user_id)` accordingly.
4. **Remaining endpoints** — `POST /start`, `POST /stop`, `GET /report`,
   `GET /tags`, `POST /tags`. Mechanical once the first one works.
5. **`takt user-add <name>`** — a CLI subcommand that generates a random
   token, inserts into `users`, prints the token. Required before anyone can
   use the server.

## How to pick up where we left off

1. `git pull` on main.
2. `cargo test` — expect 48/48 green.
3. Open `src/server/mod.rs` — two `TODO` markers to fill in:
   - `StatusResponse` struct fields (line ~20).
   - `status_handler` body (line ~47).
4. Once filled in, `cargo run -- serve --port 18080` and
   `curl http://127.0.0.1:18080/status` should return JSON.

### Design calls already locked in (don't re-litigate)

- **Single binary** contains both client and server. The `serve` subcommand
  is the server; without it, the binary is a client that uses `FlatStore`.
- **SQLite, not sled.** Chosen for maturity, observability (`sqlite3` CLI),
  and SQL's fit for time-range queries. `rusqlite 0.32` with `bundled`.
- **SqliteStore takes `user_id` at construction.** Every query has
  `WHERE user_id = ?`. This scaling mechanism is already in the schema.
- **`Store` trait stays sync forever.** RemoteStore (v0.4) will internalize
  the async via a sync HTTP client.
- **Shared state in the server: `Arc<Mutex<SqliteStore>>`** (planned). Single
  SQLite connection guarded by a Mutex. For v0.3's personal-server scale
  (≤10 users) this is fine; no connection pool needed.
- **Auth: bearer tokens only.** Each user has one token in the `users` table.
  No OAuth, no sessions, no refresh. `takt user-add` generates the token.
- **SQL is hand-written via rusqlite.** If pain grows, v0.4+ can migrate to
  `sqlx` (compile-time query checking) without touching `main.rs`.

## Conventions

### Commits

- Use **imperative mood** and a scope prefix: `feat(tags): …`, `fix: …`,
  `refactor: …`, `test: …`, `chore: …`, `docs: …`, `ci: …`.
- Body explains *why*, not *what*. The diff shows the what.
- Co-author line at the end: `Co-Authored-By: Claude Opus 4.7 (1M context)
  <noreply@anthropic.com>`.
- Each commit must compile and pass tests. If a refactor is big, split it so
  each step is bisectable.

### Rust style

- No comments on *what* the code does (names should tell that story). Only
  comment *why* when the why is non-obvious.
- Prefer `?;` + `Ok(())` over `.map(|_| ())` unless inside a builder-style
  chain.
- SQL: backslash-continued single line for readability; no leading whitespace
  inside string literals (it leaks into error messages).
- Row-decoding for SQL lives in `row_to_entry` (or similar) — if you're
  writing the same `row.get(0)` three times, extract.

### Testing

- Every `Store` backend runs the `test_harness` via the `store_tests!` macro.
  If you add a new shared assertion, **every backend gets the test for free**;
  re-run `cargo test` to confirm both pass.
- New behavior asserts belong in `src/store/mod.rs::test_harness` (shared) if
  they're about the trait contract, or in the backend's own test module if
  they're backend-specific.
- Unit tests (inside modules) test primitives; shared-harness tests assert
  trait-level behavior.

## Gotchas

- **cargo sometimes reuses a stale binary** when Edit-tool-like tools modify
  files without bumping mtime. If behavior seems wrong but code looks right,
  `cargo clean -p takt && cargo build`.
- **`chrono::NaiveDateTime` displays with nanoseconds when present.** We
  truncate to whole seconds via `log::now_seconds()` so the text format can
  round-trip through `parse`. Don't store sub-second precision anywhere.
- **`NaiveDateTime` + `and_utc().timestamp()`** is how we get unix seconds for
  SQLite. It treats the naive value as UTC for the conversion; since we never
  record timezones, round-tripping works. See `store::sqlite::to_ts` / `from_ts`.
- **Nix flake `version` field is hard-coded** — has to be bumped alongside
  `Cargo.toml` on each release. Or (future polish) switch to
  `builtins.fromTOML (builtins.readFile ./Cargo.toml)).package.version`.
- **Don't re-tag v0.1.0.** It was deleted as a "CI casualty never shipped";
  v0.2.0 is the first public version. Adding a v0.1.0 release now would
  confuse version history.

## Release procedure (for future v0.3+)

1. Finalize changes, 48+ tests green, `cargo clippy` clean.
2. Bump `Cargo.toml` version + `flake.nix` version to match.
3. Update `CHANGELOG.md` with a new `## [X.Y.Z] - YYYY-MM-DD` section.
4. `cargo publish --dry-run` to catch metadata issues.
5. Commit: `chore: bump version to X.Y.Z`.
6. Push to main.
7. `git tag vX.Y.Z && git push --follow-tags`.
8. Watch <https://github.com/kronberger-droid/takt/actions>. The workflow
   runs `test → 4 build jobs → release → publish-crates` sequentially.
9. If any job fails: re-run only the failing job from the Actions UI. The
   sequence was chosen so `publish-crates` doesn't burn the crates.io
   version slot if the GitHub Release failed.
