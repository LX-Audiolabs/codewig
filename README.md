# Codewig

[![License](https://img.shields.io/badge/license-GPL--3.0--or--later-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](rust-toolchain.toml)
[![Version](https://img.shields.io/badge/version-0.2.2-informational.svg)](Cargo.toml)
[![agal](https://img.shields.io/badge/powered%20by-agal-00ADD8.svg)](https://github.com/LX-Audiolabs/agal)
[![AI](https://img.shields.io/badge/dev-AI--assisted-6E40C9.svg)](https://github.com/LX-Audiolabs/agal)

**Live coding for Bitwig Studio** — program your DAW live with WIGSCRIPT, a
music language built for Bitwig and live performance.

```
┌─────────────────────────────────────────────┐
│  codewig-live (Slint UI)  ·  codewig-cli    │
│  ┌─────────────────────────────────────────┐│
│  │  WIGSCRIPT                              ││
│  │  new track(bass).device(Polymer)        ││
│  │  bass: n "c e g"                        ││
│  │  kick&v9kick: decay(50)                 ││
│  └─────────────────────────────────────────┘│
│         ↓  TCP + JSON  (localhost :9470)     │
│  Codewig.bwextension  (Bridge)               │
│         ↓  Controller API                    │
│  Bitwig Studio                               │
└─────────────────────────────────────────────┘
```

## What is Codewig?

A **Slint UI** (`codewig-live`) and a **CLI** (`codewig-cli`) with a **purpose-built
coding language for music** — made for **live performance in Bitwig**.
Control tracks, devices, clips, and parameters from the graphical interface or
directly via **CMD / PowerShell / Terminal**.

Both clients talk to the same extension: **`Codewig.bwextension`** on TCP **`:9470`**.

AI agents can use the same interface — focus stays **human + machine live on stage**.

### Inspiration

Codewig is **heavily inspired by**:

- **[DrivenByMoss](https://github.com/git-moss/DrivenByMoss)** (Jürgen Moßgraber) — Bitwig Controller API extensions, OSC, and deep DAW control from outside the box
- **[Strudel](https://strudel.cc)** / [strudel.cc](https://strudel.cc) — live-coding music in the browser, mini-notation, and the idea that patterns are first-class

WIGSCRIPT is its own language for Bitwig live performance; those projects shaped the direction.

---

## WIGSCRIPT — two phases

Codewig separates **authoring** (building the project and clip content) from **performance** (live control while the track runs). Within authoring, **beat** and **notes** are two different DSLs for different jobs; both compile to the same internal note representation before hitting Bitwig.

| Phase | Role | Example |
|-------|------|---------|
| **Authoring** | Build structure + clip content | `new track(lead).device(Polymer).n("c e d g").clip(start)` |
| **Performance** | Live triggers | `play` · `mute(kick)` · `s(verse).start` · `c(lead.0).start` |

Long form ≡ short form (same parse): `track`/`t`, `clip`/`c`, `scene`/`s`, `device`/`d`, `notes`/`n`.  
Example: `track(bass).device(Polymer)` ≡ `t(bass).d(Polymer)`; `clip(bass.0).start` ≡ `c(bass.0).start`.

```wigscript
# Authoring: Fluent — create track, insert device, write content (slot 0)
new track(bass).device(Polymer).n("c e g").clip(start)
new track(kick).device(v9 kick).beat(4_).clip(start)
new track(lead).device(Polymer).add(Delay+)

# Authoring: Colon — write notes into an existing track / scene cell
bass: n "c e g"
lead@verse: n "e c g"
lead: arp:up "Cm7"
lead: chord "C Am F G"

# Performance: Param snapshot — track × device (& not @)
kick&v9kick: decay(50) punch(40)
# Without YAML: wire 0..1 + Bitwig param name as typed
# someClap&foo: cutoff(0.7)

# Performance: Live
new scene(verse)
s(verse).t(lead).c(new)
s(verse).start
c(bass.0).start
mute(kick)
unmute(kick)
mute(kick) 4          # timed mute (bars)
tempo 128
play
stop
```

### Notes (colon + fluent `.n`)

- Bitwig octaves: `c` = **C3** = MIDI **60**
- Space-separated events = **1 beat** each (steps 0, 4, 8, …)
- `~` = rest · `[c d e f]` = 16ths in one beat
- Actions: **`n`**, **`chord`**, **`arp`** / `arp:up|down|updown|rand`
- **Invalid:** bare `c e g` · `d "bd hh"` · `drums:909: …` · params glued onto note lines

### Fluent details

- `.n` / `.beat` write **slot 0** only. Multi-clip → colon `track@scene` / `track@slot`
- `.beat(4_)` = 4-on-the-floor for mono drum modules (not Strudel hit markers)
- `.beat:16(1,5,9,13)` = explicit 16th positions; positions are **always 1-based** (musician view) and normalized to 0-based for Bitwig
- `.c` / `.clip` = clip cell (new/start/stop), **not** notes
- `.device(...)` / `.add(...)` = insert Bitwig device (see below)

### Devices — insert vs display vs params

Not a closed hard-coded allowlist of nine names.

| | Rule |
|---|------|
| **Insert** | Any resolvable Bitwig/library name (alias, `.bwdevice`, UUID). Not an allowlist. |
| **UI Devices tab** | **Help only** — devices with `devices/*.yaml` (sensible coding targets). You can still drive unlisted devices. |
| **Params** (`track&device:`) | With YAML: display ranges + aliases. Without: raw wire **0..1** + param name (CLI-style). |
| **Not for help YAML** | Presets, browsers, ultra-complex UIs (e.g. Delay-4) — use Bitwig UI or raw CLI. |
| **Out of scope** | Sampler / Drum Machine insert. No `d "bd hh"`. No VST3/LV2 help catalog. |

Add a help entry → drop `devices/<id>.yaml` (see `devices/README.md`) → appears in Devices tab.

---

## Control

| Path | What |
|------|------|
| **Slint UI** (`codewig-live`) | Live input + sidebar reference; same WIGSCRIPT as CLI |
| **CLI** (`codewig-cli`) | `codewig-cli eval "…"`, batch files, legacy flat tokens |
| **Extension** (`Codewig.bwextension`) | Bridge for UI + CLI (Controllers → Codewig Bridge) |

UI and CLI both go through **`codewig-core`** → TCP `:9470`. The UI does **not** shell out to `codewig-cli`.

---

## Status

| Phase / Component | Status |
|-------------------|--------|
| **Authoring** | |
| Fluent structure | ✅ create track, device insert, `.n` / `.beat`, mute, clip |
| Colon notes | ✅ `n` / `chord` / `arp` with quoted mini-notation |
| Mini-notation | ✅ spaces, rests `~`, groups `[…]`, euclid, suffixes |
| Device insert | ✅ open Bitwig resolve (UUID / library file); no closed name list |
| Devices UI list | ✅ only entries from `devices/*.yaml` |
| **Performance** | |
| Param (`track&device:`) | ✅ catalog snapshot (`devices/*.yaml`; v9 Kick shipped) |
| Live triggers | ✅ play/stop, tempo, mute (incl. timed), scene/clip launch |
| **Open / not supported** | Expander edge cases 🚧 · Polymer params ⏸ · Sampler / Drum Machine insert, `d "bd hh"`, kit `:909`, bare Tidal, VST3/LV2 params |

---

## Quickstart

Bitwig running with **Codewig Bridge** loaded, then:

```powershell
# Authoring: build structure + clip content
codewig-cli eval "new track(bass).device(Polymer).n(\"c e g\").clip(start)"
codewig-cli eval "new track(kick).device(v9 kick).beat(4_).clip(start)"

# Authoring: notes into existing cells
codewig-cli eval "bass: n \"c e g\""
codewig-cli eval "lead@verse: n \"e c g\""

# Performance: param snapshot
codewig-cli eval "kick&v9kick: decay(50) pitch(40)"

# Performance: live
codewig-cli eval "new scene(verse)"
codewig-cli eval "s(verse).start"
codewig-cli eval "mute(kick)"
codewig-cli eval "tempo 128"
codewig-cli eval "play"

# Same lines work in codewig-live UI input
# Batch: codewig-cli batch session.wig
```

Legacy flat CLI tokens still work (`codewig-cli play`, `track mute kick`, …) as a fallback — prefer WIGSCRIPT `eval`.

---

## Build

Requires a stable Rust toolchain (see `rust-toolchain.toml`).

```bash
# Rust workspace (CLI + core + Slint UI)
cargo build --workspace

# Release builds
cargo build --workspace --release

# Run checks
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

The Java extension under `extension/` is built separately with Gradle:

```bash
cd extension
./gradlew build
```

The resulting `extension/build/libs/*.jar` can be installed as a Bitwig Controller Extension (Settings → Controllers → Install Extension).

## Installation

1. Build or download the binaries (`codewig-cli`, `codewig-live`).
2. Build and install the Bitwig extension from `extension/`.
3. Start Bitwig Studio, open the Codewig Bridge controller extension, and ensure it is listening on `127.0.0.1:9470`.
4. Run `codewig-live` for the UI or use `codewig-cli eval "…"` from a terminal.

## Contributing

Open issues and pull requests are welcome. When changing language behavior, update the WIGSCRIPT examples in this README and add or adjust tests in the relevant `core/src/music/*` module.

## License

Codewig is licensed under the [GNU General Public License v3.0 or later](LICENSE).
