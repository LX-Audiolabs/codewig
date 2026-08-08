<!-- AGAL:AUTO-START -->
# codewig-core

> Auto-generated from workspace scan. Do not edit between AUTO markers.

| | |
|---|---|
| kind | `member` |
| path | `core` |
| description | Codewig core library — shared between codewig-cli and codewig-live |
| generated | `2026-08-08T08:38:19Z` |

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

_Why this crate/plugin exists. Edit freely._

## Open

- [ ] 

## Decisions

_Architecture choices worth remembering._

## Atoms (human)

_Graph atoms live **above** in AUTO. Add durable decisions/lessons here:_

```text
[ATOM] type=decision|lesson|constraint | detail=…
```
