---
id: wigscript
group: codewig
summary: WIGSCRIPT layers, parse/expand/execute path, invalid patterns, device/param rules for CODEWIG.
triggers: wigscript, fluent, colon notes, music parse, mute, scene, clip, device param, mini-notation, codewig-core
verify: four layers not mixed; quotes on note patterns; + for params; TCP via core Client; no bare Tidal
adapted: true
---

# WIGSCRIPT (local CODEWIG skill)

**Summary:** Music language for Bitwig live performance. Shared by `codewig-live` and
`codewig-cli` via **`codewig-core`**. Load when editing parse/expand/execute or language UX.

Local skill (`07-codewig/`) — not in agal tool packs. Canonical examples: root `README.md`.

## Pipeline

```
line string
  → music::parse::parse_music_line
  → expand (chord / arp / mini)
  → execute (MusicSession + Client TCP)
  → extension :9470 → Bitwig
```

| Module | Job |
|--------|-----|
| `core/src/music/parse/` | fluent · line · mini |
| `core/src/music/expand.rs` | chord / arp / pattern expand |
| `core/src/music/execute.rs` | session + bridge commands |
| `core/src/music/device.rs` | insertable / drum helpers |
| `core/src/music/param_catalog.rs` | `devices/*.yaml` |
| `core/src/protocol.rs` | length-prefixed JSON TCP |

## Four layers

| Layer | When | Shape |
|-------|------|--------|
| **Fluent** | Create structure | `new track(name).device(…).n("…").clip(start)` |
| **Colon** | Write into existing cell | `track: n "…"` · `track@scene: n "…"` |
| **Param** | Snapshot device/perform params | `track: +device.param:val` |
| **Performance** | Transport / launch / mute | `play` · `s(verse).start` · `mute(kick) 4` |

Long ≡ short: `track`/`t`, `clip`/`c`, `scene`/`s`, `device`/`d`, `notes`/`n`.

## Notes rules

- Bitwig octave: `c` = C3 = MIDI 60
- Space-separated events = 1 beat each
- `~` rest · `[c d e f]` 16ths in one beat
- Actions: `n` · `chord` · `arp` / `arp:up|down|updown|rand`
- Expressions: `.vel` `.pan` `.pres` `.tim` `.gain` `.chnz` (see README)

## Invalid (reject / do not invent)

- Bare `c e g` or Tidal full lines
- `d "bd hh"` hit-maps / Drum Machine kit syntax
- `drums:909:` style kits
- Param keys on note lines without documented `+param:` / `+device.param:` forms
- Assuming UI shells out to CLI

## Devices

| Concern | Rule |
|---------|------|
| **Insert** | Any resolvable Bitwig/library name (not a closed nine-name list) |
| **YAML help** | `devices/*.yaml` → UI Devices tab + display ranges |
| **Params without YAML** | wire **0..1** + Bitwig param name as typed |
| **Out of scope** | Sampler / Drum Machine kit map; VST3/LV2 help catalog |

## Agent check

1. Which layer is this line? (one job)
2. Does it need quotes around mini-notation?
3. Params → `+device.param:val` on a target line (`+device:` for lifecycle)?
4. Change parse? Keep UI + CLI on same `parse_music_line` path.
5. Bridge only via `Client` — no second protocol.

## Related

| Need | Where |
|------|--------|
| Full examples | root `README.md` |
| Crate map | `agal/notes/codewig-core.md` |
| Slint UI | skill `ui/slint` |
| Workspace atoms | `agal/notes/_workspace.md` |
