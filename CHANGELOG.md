# Changelog

All notable changes to Codewig.

## [Unreleased]

### Changed
- Public-release readiness: aligned crate metadata, fixed Windows resource copyright to GPL-3.0-or-later, and removed local absolute paths from generated agal metadata.
- README expanded with build, install, contributing, and license sections.

## [0.2.2] — 2026-08-08

### Changed
- **Repo move** — repository relocated from `lxndr/CLIwig` to `LX-Audiolabs/codewig`.
- Workspace metadata updated: GPL-3.0-or-later license, repository URL, and crate-level readme references.
- Controller extension build migrated to current Bitwig Controller API toolchain.

### Added
- WIGSCRIPT performance-layer controls: rename/delete for tracks, scenes, and clips; device on/off/move/delete.
- `s(verse)` shorthand alias for `scene(verse)`.
- Linux AppImage build workflow.

### Fixed
- Expander mini-notation edge cases (subdivide, random choice, repeat/group).
- Clip note writes now use viewport-relative coordinates and correct beat durations.

## [0.2.1] — unreleased

### Added
- **WIGSCRIPT** — fluent, colon, param, and performance layers for Bitwig live coding.
- **`codewig-live`** — Slint UI for live performance.
- **`codewig-cli`** — command-line interface (`eval`, TCP bridge).
- **`codewig-core`** — shared music language parser, AST, and Bitwig device execution.
- **`Codewig.bwextension`** — Bitwig Controller API bridge (TCP `:9470`).
- **Device parameter catalog** — YAML-based help entries for Bitwig devices.
