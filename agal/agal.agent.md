# agal agent summary

**Summary:** Compact structural map of this audio-plugin workspace.  
Lists plugins, crates, frameworks, migrations, edges, and findings.  
Use as first context before opening the full JSON graph.

project: **CODEWIG**  
generated: `2026-08-08T08:38:19Z`  
version: 0.6.2  
health: **ok**  
nodes: 3 · edges: 2 · findings: 1 (error=0 warn=0 info=1)

## frameworks detected
clap, slint

## plugins

## crates

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

_7 skill file(s) under `skills/` — see AGAL.md._

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

