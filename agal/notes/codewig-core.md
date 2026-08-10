<!-- AGAL:AUTO-START -->
# codewig-core

> Auto-generated from workspace scan. Do not edit between AUTO markers.

| | |
|---|---|
| kind | `member` |
| path | `core` |
| description | Codewig core library — shared between codewig-cli and codewig-live |
| generated | `2026-08-10T16:45:02Z` |

## Graph atoms (auto)

_Regenerated each `agal .`. Scan these first. Human atoms: below HUMAN marker._

```text
[ATOM] type=fact | detail=kind=member id=core
[ATOM] type=fact | detail=roles=entry+manifest+source
[ATOM] type=fact | detail=used_by=cli via depends_on
[ATOM] type=fact | detail=used_by=ui via depends_on
```

## dependents (inbound)
- `cli` --depends_on--> `core`
- `ui` --depends_on--> `core`

## structure
- roles: entry, manifest, source

## agent focus
**L1:** scan **Graph atoms** above first, then human body below HUMAN.  
After `agal.agent.md` (L2). Escalate L0: `core` in json / `agal --plugin codewig-core .`

<!-- AGAL:AUTO-END -->

<!-- AGAL:HUMAN — edit below this line; preserved on regenerate -->

## Intent

Shared library for CLI + UI: **WIGSCRIPT** parse/expand/execute and TCP **Client**
to the Bitwig extension (`protocol`, default `:9470`).

## Modules

| Path | Role |
|------|------|
| `music/parse` | fluent · line · mini |
| `music/expand` | chord / arp / pattern |
| `music/execute` | `MusicSession` → bridge cmds |
| `music/device` · `param_catalog` | insert helpers · YAML params |
| `protocol` | length-prefixed JSON TCP |

## Open

- [ ] Expander edge cases
- [ ] Polymer YAML params (deferred)

## Decisions

- One language path for UI + CLI (no UI→CLI shell).
- Language skill: `agal/skills/07-codewig/wigscript.md`.

## Atoms (human)

```text
[ATOM] type=constraint | detail=All WIGSCRIPT execution goes through music::parse → expand → execute + Client
[ATOM] type=decision | detail=Param catalog from devices/*.yaml is optional help; insert is open Bitwig resolve
```
