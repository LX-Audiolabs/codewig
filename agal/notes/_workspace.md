# Workspace memory

**Summary:** Durable cross-crate notes for CODEWIG agents.  
**Never overwritten** by `agal .`. Keep short. Prefer `[ATOM]` one-liners.

## Atoms

```text
[ATOM] type=decision | detail=Product is live-coding Bitwig via WIGSCRIPT, not an audio plugin host/format stack
[ATOM] type=constraint | detail=UI (codewig-live) and CLI both use codewig-core::Client TCP :9470; UI must not spawn CLI
[ATOM] type=constraint | detail=WIGSCRIPT four layers — fluent=structure, colon=notes, param=track&device snapshots, performance=play/mute/scene/clip; do not mix jobs
[ATOM] type=constraint | detail=No bare Tidal lines (c e g); mini-notation only inside quotes; no d "bd hh"; no Drum Machine/Sampler insert as kit map
[ATOM] type=decision | detail=Param form is track&device: key(val) with & not @; YAML under devices/ optional (display ranges); without YAML wire 0..1 + Bitwig name
[ATOM] type=decision | detail=Device insert is open Bitwig resolve (name/alias/UUID); devices/*.yaml is help catalog for UI tab only
[ATOM] type=constraint | detail=Java extension is thin bridge — Rust owns language; keep allowlist/device catalog in sync only when changing insert surface
[ATOM] type=decision | detail=Legacy CLI flat tokens still work via cli; WIGSCRIPT eval is primary path for agents and UI
[ATOM] type=lesson | detail=agal must not treat clap (CLI crate) as CLAP format — codewig-cli uses clap for args only
```

## Open

- [ ] Expander edge cases (pattern → MIDI)
- [ ] Polymer params YAML (deferred empty)
- [ ] Timed scene/clip follow on mute schedule (TODO, not now)

## Decisions

See atoms above. Status table: root `README.md`.
