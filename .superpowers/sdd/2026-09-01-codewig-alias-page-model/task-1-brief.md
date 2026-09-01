# Task 1: Introduce `DeviceAlias` catalog from `aliases.yml`

**Project context:** Codewig is a Rust CLI/UI + Java Bitwig-extension project. The goal is to replace per-device YAML param catalogs with a single `devices/aliases.yml` that only maps aliases to canonical Bitwig device names. This task is the first in that refactor and is isolated to the Rust core crate.

**Files:**
- Create: `core/src/music/alias_catalog.rs`
- Modify: `core/src/music/mod.rs` (register module)
- Test: inline `#[cfg(test)]` in `core/src/music/alias_catalog.rs`

**Interfaces (produced by this task):**
- `pub enum DeviceHostKind { Bitwig, Clap }`
- `pub struct DeviceAlias { pub id: String, pub bitwig_name: String, pub kind: DeviceHostKind, pub aliases: Vec<String> }`
- `pub struct AliasCatalog { devices: Vec<DeviceAlias> }`
- `impl AliasCatalog { pub fn from_yaml(text: &str) -> Result<Self, String>; pub fn resolve(&self, name: &str) -> Option<&DeviceAlias>; pub fn devices(&self) -> &[DeviceAlias]; }`

**Steps (must all be completed and committed):**

1. Write a failing test `resolves_polymer_alias` in the new module that parses a YAML snippet and resolves `poly` to `Polymer`.
2. Run `cargo test -p codewig-core alias_catalog::tests::resolves_polymer_alias -- --nocapture` and verify it fails.
3. Implement `alias_catalog.rs` exactly as specified in the plan (use `super::device::norm` for normalization; use `serde::Deserialize` and `std::collections::HashMap`).
4. Register the module in `core/src/music/mod.rs` with `pub mod alias_catalog;`.
5. Run `cargo test -p codewig-core alias_catalog::tests::resolves_polymer_alias -- --nocapture` and verify it passes.
6. Run `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` to ensure no regressions.
7. Commit with message: `feat(core): add DeviceAlias catalog parser for aliases.yml`.

**Global constraints for this task:**
- No breaking changes to existing public APIs.
- All changes must compile and pass tests.
- Do not touch `param_catalog.rs`, `paths.rs`, CLI, UI, extension, or docs in this task.

**Report contract:** Write a one-paragraph summary plus the commit hash to the report file. Include the exact test command output.
