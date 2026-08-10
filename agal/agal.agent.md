# agal agent summary

**Summary:** Compact structural map of this audio workspace.  
Lists members, plugins, crates, frameworks, edges, and findings.  
Use as first context before opening the full JSON graph.

project: **CODEWIG**  
generated: `2026-08-10T16:45:02Z`  
version: 0.7.0  
health: **ok**  
nodes: 3 · edges: 2 · findings: 1 (error=0 warn=0 info=1)

## frameworks detected
slint

## rules
- **bridge**: TCP JSON on localhost:9470 only; UI and CLI share codewig-core Client; UI never shells out to CLI
- **devices**: Insert any resolvable Bitwig name; devices/*.yaml = help + param ranges only
- **java_out**: extension/ is thin Bitwig Controller API bridge — leave alone unless task is Java
- **kind**: tool — Bitwig live-coding clients + WIGSCRIPT; not a CLAP/VST plugin framework
- **language**: WIGSCRIPT primary — four layers (fluent / colon / param / performance); no bare Tidal; no d "bd hh"
- **roadmap**: README.md status table + agal/notes/_workspace.md atoms

## edges
### depends_on
- `cli` → `core`
- `ui` → `core`

## findings (error=0 warn=0 · info=1 in json/html)
health **ok** — if **blocked**, fix errors before feature work.
_no error/warn_

## notes (focus)
`agal/notes/<name>.md` — auto header + human body

members: `codewig-cli`, `codewig-core`, `codewig-live`

## skills

Full pack list + index: **`AGAL.md`**. Load on demand only.

_10 skill file(s) under `skills/` — see AGAL.md._

## read order
Disclosure: **L3** `AGAL.md` → **L2** this file (+ delta) → **L1** one note → **L0** slice/json.  
Open the next layer only if the current one is not enough.

1. **L3** **`AGAL.md`** (orientation — skills + budget / loadouts / disclosure).  
2. **L2** **this file** (structural map + health).  
3. If health is **blocked**, fix error findings first (path + fix fields).  
4. **L2** **`agal.delta.md`** if present.  
5. **L1** **`notes/<focus>.md`** (**one** note; scan `[ATOM]` first).  
6. **loadout** — skills on demand from `AGAL.md` (never dump all; **≤1** skill file).  
7. **L0** escalate: `agal --plugin NAME .` slice, or `agal.json`.  
8. HTML is for humans (overview); agents prefer md/json.  

Skills are **not** auto-copied on generate — `agal skills sync` (default: **core** only).  
Existing `*.slice.json` files are refreshed on every generate; new slices need `--plugin NAME`.  
Human CLI cheatsheet: **`Cheatsheet.md`** in this folder.

