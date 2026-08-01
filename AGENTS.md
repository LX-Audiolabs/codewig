# AGENTS.md — codewig-live / CLIwig

## Source of truth (read first)

**Code wins.** Prefer these over long design essays:

| Priority | Where |
|----------|--------|
| 1 | `core/src/music/{parse,ast,execute,device,expand,param_catalog}.rs` |
| 1 | `devices/*.yaml` (param catalog — bitwig\|clap only) |
| 1 | `extension/.../DeviceCatalog.java` (insert allowlist) |
| 1 | `ui/src/commands.rs`, `cli/src/main.rs` (`eval`) |
| 2 | Root `README.md` |
| 3 | Local `research/CURRENT.md` if present (**folder is gitignored**) |

### WIGSCRIPT — three layers (do not blur)

| Layer | Role | Examples |
|-------|------|----------|
| **Fluent** | Build structure | `new track(lead).device(Polymer).n("c e d g").clip(start).mute()` |
| **Colon** | Content in existing cells | `lead@verse: n "e c g"` |
| **Param** | Device params on a track | `kick&v9kick: decay(50) pitch(40)` (`@` = scene/slot, `&` = track×device) |
| **Performance** | Live triggers | `play` · `mute(kick)` · `s(verse).start` · `c(lead.0).start` |

- Fluent **`.n` / `.beat`** → **slot 0** only (first clip). Multi-clip → `track@scene` / `track@slot`.
- Notes: Bitwig octaves (`c` = **C3** = MIDI **60**); space events = **1 beat** each (steps 0,4,8…); `~` = rest; `[c d e f]` = 16ths in one beat.
- `.c` / `.clip` = clip cell (new/start/stop), **not** notes (notes = `.n` / `n`).
- Global `play`/`stop` = transport, not fluent step.

### Do not reintroduce these mistakes

1. **Bare Tidal lines are not the REPL** — invalid: `c e g`. Valid: `bass: n "c e g"` (quotes).
2. **WIGSCRIPT is primary**. CLI: `cliwig eval "same line"`.
3. **UI + CLI → `cliwig-core` TCP `:9470`**. UI does not shell out to `cliwig`.
4. **Insert allowlist** — Polymer, Polysynth, Organ + stock drums (`v9 kick`…). No Sampler / Drum Machine. No `d "bd hh"`.
5. **`research/` gitignored** — code wins.
6. Live focus: **clips / scenes / mute**.
7. **Params = snapshot only** (`param.set`). Display ranges from `devices/*.yaml` → wire `0..1`. No YAML / empty `params` → no param support (insert may still work).
8. **Param catalog scope** = Bitwig devices + CLAP (system paths later). **No** VST3/LV2. No Bitwig plugin-path queries.
9. **Param address** = `track&device:` — not track alone, not `@` (reserved scene/clip).
10. **Timed mute OK**: `mute(x) N` / `@bar`.
11. **`>` passthrough** only UI/CLI entry, not `execute_line`.
12. **Launcher** — scene=row, track=column, clip=cell: `new scene(verse)` · `s(verse).t(lead).c(new)` · `lead@verse: n "…"`.
13. **Polymer** — insert + notes OK; params deferred (`devices/polymer.yaml` empty until fixed subset).

## Caveman
Talk terse. Drop articles/filler/pleasantries. Fragments OK. Technical terms exact.
`/caveman lite|full|ultra|wenyan`. Stop: "stop caveman". Normal prose for security warnings, irreversible actions, confusion. Code/commits/PRs normal.

## Ponytail — Lazy Senior Dev
Before code, climb ladder: 1. YAGNI? 2. Already in codebase? 3. stdlib? 4. platform? 5. installed dep? 6. one-liner? 7. write minimum.
Bug fix = root cause, not symptom. Trace every caller.
No abstractions, no new deps, no boilerplate. Delete > add. Boring > clever. Fewest files. Question complexity. Mark simplifications `ponytail:`.
Not lazy: input validation, error handling preventing data loss, security, accessibility, explicit requests. Non-trivial logic → ONE assert/test.

## github commits & push
Commits always as user.name="lxndrbe" & user.email="[redacted-email]"  
Github AUTH always as github.user "lxndrbe"
