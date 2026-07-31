# AGENTS.md — codewig-live / CLIwig

## Source of truth (read first)

**Code wins.** Prefer these over long design essays:

| Priority | Where |
|----------|--------|
| 1 | `core/src/music/{parse,ast,execute,device,expand}.rs` |
| 1 | `extension/.../DeviceCatalog.java` (insert allowlist) |
| 1 | `ui/src/commands.rs`, `cli/src/main.rs` (`eval`) |
| 2 | Root `README.md` |
| 3 | Local `research/CURRENT.md` if present (**folder is gitignored**) |

### Do not reintroduce these mistakes

1. **Bare Tidal lines are not the REPL** — invalid: `c e g`, `kick ~ ~ snare`.  
   Valid: `bass: n "c e g"`, mini-notation **only inside quotes**.
2. **WIGSCRIPT is primary** (UI form). CLI: `cliwig eval "same line"`. Not a second grammar.
3. **UI and CLI both use `cliwig-core`** → TCP `:9470`. UI does **not** shell out to `cliwig`.
4. **Insert allowlist = 9 devices** (Polymer, Polysynth, Organ, Instrument Layer, Filter, Reverb, Delay+, Chorus+, Saturator).  
   **No** Sampler / Drum Machine via `device.add`. Drum pads = manual + MIDI aliases only.
5. **`research/` is gitignored** — may be stale; always verify against code. Prefer `research/CURRENT.md` when updating notes.
6. Live focus after setup: **clips / scenes / mute**, not full device browser.

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
