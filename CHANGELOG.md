# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-04-18

### Added

- Hierarchical tag tree with unambiguous leaf-name resolution
  (`takt start fix-bug` resolves to `work/project-x/fix-bug` when unique).
- `takt start <tag>` — begin tracking a task, auto-stopping any active one.
- `takt stop` — end the current task.
- `takt status` — show the running task and elapsed time.
- `takt tag add <path>` and `takt tag list` — manage the tag tree.
- `takt report this <day|week|month>` — time per tag for the current period.
- `takt report last <n> <day|week|month>` — time per tag for the last N units.
- `takt report` with no arguments defaults to `this week`.
- Reports spanning month boundaries are stitched across monthly log files;
  malformed files warn on stderr, missing files are silently skipped.
- Human-readable log format (`~/.local/share/takt/log/YYYY-MM.takt`) with one
  file per month.
- Indentation-based tag file (`~/.local/share/takt/tags`) with space-only,
  even-width indentation.
- Structured error messages with 1-indexed line numbers for malformed tag and
  log files, including depth-jump detection that previously caused an infinite
  loop.

[Unreleased]: https://github.com/kronberger-droid/takt/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/kronberger-droid/takt/releases/tag/v0.1.0
