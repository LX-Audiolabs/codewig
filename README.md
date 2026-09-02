# Codewig

[![License](https://img.shields.io/badge/license-GPL--3.0--or--later-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](rust-toolchain.toml)
[![Version](https://img.shields.io/badge/version-0.2.2-informational.svg)](Cargo.toml)
[![agal](https://img.shields.io/badge/powered%20by-agal-00ADD8.svg)](https://github.com/LX-Audiolabs/agal)
[![AI](https://img.shields.io/badge/dev-AI--assisted-6E40C9.svg)](https://github.com/LX-Audiolabs/agal)

**Live coding for Bitwig Studio** — program your DAW live with WIGSCRIPT, a
music language built for Bitwig and live performance.

> ⚠️ **WIGSCRIPT is under active development.** Grammar and commands are stable
> enough for live use, but details may still change between releases.

```
┌─────────────────────────────────────────────┐
│  codewig-live (Slint UI)  ·  codewig-cli    │
│  ┌─────────────────────────────────────────┐│
│  │  WIGSCRIPT                              ││
│  │  new track(bass).device(Polymer)        ││
│  │  bass: n "c e g" +cutoff:0.3           ││
│  │  t(bass).device(Polymer).page(list)     ││
│  │  t(bass).perform(cutoff=0.3)            ││
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

Codewig separates **authoring** (building the project and clip content) from
**performance** (live control while the track runs). Within authoring, **beat**
and **notes** are two different DSLs for different jobs; both compile to the same
internal note representation before hitting Bitwig.

| Phase | Role | Example |
|-------|------|---------|
| **Authoring** | Build structure + clip content | `new track(lead).device(Polymer).n("c e d g").clip(start)` |
| **Performance** | Live triggers | `play` · `mute(kick)` · `s(verse).start` · `c(lead.0).start` |

Long form ≡ short form (same parse): `track`/`t`, `clip`/`c`, `scene`/`s`,
`device`/`d`, `notes`/`n`.
Example: `track(bass).device(Polymer)` ≡ `t(bass).d(Polymer)`; `clip(bass.0).start` ≡ `c(bass.0).start`.

Full grammar reference: [`docs/WigScript-Authoring.md`](docs/WigScript-Authoring.md)
and [`docs/WigScript-Performance.md`](docs/WigScript-Performance.md).
**Code wins** when notes and docs disagree (`core/src/music/`).

### Authoring — fluent chains

One line = **one chain**. Starts with `new track(name)` (create) or
`track(name)` / `t(name)` (existing track). Steps are appended with `.` and run
**in order**.

| Step | Short | What it does |
|------|-------|--------------|
| `.device(<name>)` | `.d(...)` | Insert device (synth / FX / drum module) |
| `.add(<name>)` | — | Append another device to the chain |
| `.beat(<shorthand>)` | — | Percussion rhythm (mono drum modules), writes **slot 0** |
| `.beat:16(1,5,9,13)` | — | Explicit 16th positions (1-based) |
| `.n("…")` | `.notes("…")` | Note pattern, writes **slot 0** |
| `.mute()` | — | Mute the track |
| `.rename(<name>)` | — | Rename the track |
| `.delete()` | — | Delete the track |
| `.clip(start)` / `.clip(stop)` | — | Launch/stop the clip on slot 0 |
| `.c(0).start` / `.c(0,1).start` | — | Launch/stop/rename/delete specific clip slots |

```wigscript
new track(bass).device(Polymer).n("c e g").clip(start)
new track(kick).device(v9 kick).beat(4_).clip(start)
new track(lead).device(Polymer).add(Delay+)
t(bass).mute()
t(bass).rename(low)
```

> **Important:** `.n` and `.beat` write **slot 0 only**. For multiple clips per
> track use colon notation `track@scene: n "…"`.

Chain shorthand (no `new track`): `!name [kind:kit] device1 device2 …`

```wigscript
!bass Polymer Filter Delay-2        # instrument track, devices in order
!drums:909 layer                    # drum-kit shell
```

### Authoring — beat DSL (rhythm)

For **monophonic** Bitwig drum modules (not Drum Machine). Trigger note is MIDI
36 (C1) or the last drum device in the chain.

| Shorthand | Pattern (1-based) | 0-based | Duration |
|-----------|-------------------|---------|----------|
| `4_`  | 1, 5, 9, 13   | 0, 4, 8, 12  | 1 beat |
| `2_4` | 1, 9          | 0, 8         | 2 beats |
| `off` | 3, 7, 11, 15  | 2, 6, 10, 14 | 1 beat |
| `bk2` | 1, 9          | 0, 8         | 2 beats |

```wigscript
new track(kick).device(v9 kick).beat(4_).clip(start)
new track(hat).device(v8 Hat).beat(off)
new track(perc).device(v9 Snare).beat:16(1,5,11,14)
```

- Explicit positions are **always 1-based** (musician view: "hit 1, 5, 9, 13");
  internally normalized to 0-based for Bitwig.
- Invalid: `0` or values > grid. "Step" = smallest grid unit (16th at `beat:16`).

### Authoring — note DSL (mini-notation)

#### Pitches

| Written | Meaning |
|---------|---------|
| `c` | C3 = MIDI 60 (Bitwig octave) |
| `c3`, `c4`, `eb4` | explicit Bitwig octave |
| `c#` / `cis`, `eb` / `es` | accidentals (English + German) |
| `60`, `36`, `127` | without key = raw MIDI; **with key** = scale degree (`0 2 4` = root/third/fifth) |

Set the key first: `k C minor` (root + scale; scales: major, minor, dorian,
phrygian, lydian, mixolydian, locrian, pentatonic, blues, chromatic).

#### Rhythm / structure

| Syntax | Meaning |
|--------|---------|
| space | next event = **1 beat** (steps 0, 4, 8, …) |
| `~` | rest |
| `[c d e f]` | one beat subdivided into 16ths |
| `,` | superposition (parallel sequences) |
| `<c e g>` | alternation (cycles) |
| `{c e, g a}` | polymetric (own rates, wraps) |
| `{c e g}%3` | subdivide (spread over 3 slots) |
| `[c \| d]` | random choice per pass |
| `c(3,8)` | euclidean rhythm (3 hits in 8 grid steps) |

#### Suffixes (on one event)

| Suffix | Meaning |
|--------|---------|
| `*N` | repeat N× (same beat) |
| `/N` | slow down (duration ×N) |
| `!N` | replicate (N copies) |
| `_` | elongate ×2 |
| `@N` | elongate ×N |
| `?` / `?0.3` | random drop (default 50%) |
| `:N` | shift octave (±N) |
| `(beats,steps[,offset])` | euclid suffix |

#### Note modifiers (after the pattern)

| Modifier | Meaning | Range |
|----------|---------|-------|
| `.vel(80)` | velocity | 0..127 |
| `.pres(50)` | pressure | 0..100 % |
| `.tim(30)` | timbre | −100..100 % |
| `.pan(-20)` | pan | −100..100 % |
| `.gain(100)` | gain | 0..100 % |
| `.chnz(75)` | chance (per note) | 0..100 % |

`~` inside the parens skips that note (value unchanged).

#### Actions (colon form)

```wigscript
bass: n "c e g"              # exact notes
lead: chord "C Am F G"       # chord tokens
lead: arp:up "Cm7"           # arp: up|down|updown|rand
```

Colon addressing: `track:`, `track@scene:`, `track@0:` (scene name must exist
first via `new scene(name)`; index 0 is primary). No `@` → slot 0.

**Invalid:** bare `c e g` · `d "bd hh"` · `drums:909: …` · params glued onto note lines.

### Performance

```wigscript
# Transport
play
stop
tempo 128

# Mute / unmute — name or comma-separated indices
mute(kick)
mute(1,3,5)
unmute(kick)
mute(kick) 4          # timed: auto-invert after N bars
mute(kick) @bar       # apply at next bar boundary
mute(kick) 4 @bar     # both

# Scenes
new scene(verse)
s(verse).start        # by name
s(1).start            # by index
scene(0).stop
s(verse).rename(drop)
s(verse).delete()     # incl. clips

# Clips — launch / stop / rename / delete, multi-ref
c(bass.0).start
c(bass.0).stop
c(bass.0).rename(intro)
c(bass.0).delete()
c(bass.0, kick.1).start
clip(bass.0).start

# Scene × track clip (Bitwig launcher cell)
s(verse).t(lead).c(new)          # empty clip at track × scene
s(verse).t(lead).c(new, intro)   # … with clip name
s(verse).t(lead).c(start)
s(1).t(bass).c(stop)
```

### Performance — param snapshots (page model)

Codewig controls Bitwig through **Remote Control / Perform pages** (8 slots per page),
not the full direct-parameter dump. This matches how hardware controllers work:
map the parameters you want on a page, then address them by name or slot.

List the current page slots first, then set them:

```wigscript
# Track Perform page (track-level 8 slots)
t(bass).perform(list)
t(bass).perform(cutoff=0.3, resonance=0.7)

# Device Remote Control page (cursor device 8 slots)
t(bass).device(Polymer).page(list)
t(bass).device(Polymer).page(cutoff=0.3)
```

Inline on a note line (writes notes + sets page slots in one line):

```wigscript
bass: n "c e g" +cutoff:0.3              # track Perform page
bass: n "c e g" +Polymer.cutoff:0.3     # Polymer device page
```

Pattern-less — same page set without touching notes/clip. First token needs
`+`; later tokens in the same run may drop it and inherit the device:

```wigscript
bass: +cutoff:0.3                       # track Perform page
bass: +polymer.cutoff:0.3               # Polymer device page
bass: +polymer.cutoff:0.3 res:0.5       # res inherits "polymer"
```

| Syntax | Target |
|--------|--------|
| `+name:value` | track Perform page |
| `+device.name:value` | that device's Remote Control page |
| `.perform(...)` | track Perform page (list or name=value) |
| `.page(...)` | cursor device's Remote Control page (list or name=value) |

Values are wire-normalized **0..1**; Bitwig maps them to the parameter's real range.
Use `.list` first to see exact slot names.

Device lifecycle ops use the same `+device:` prefix, followed by a bare op keyword:

```wigscript
kick: +Polymer: on
kick: +Polymer: off
kick: +Polymer: delete
kick: +Polymer: move 0      # chain position (0-based)
```

### Misc

```wigscript
> track mute kick         # passthrough to legacy CLI tokens
mode cmd                  # mode switch (cosmetic in core)
k C minor                 # set key/scale for scale-degree notes
```

---

## Devices — aliases + page model

Codewig keeps a single alias catalog: `devices/aliases.yml`.
It maps short names/aliases to Bitwig's canonical device names and declares whether
a device is a Bitwig stock device or a CLAP plugin.

| | Rule |
|---|------|
| **Insert** | Any resolvable Bitwig/library name (alias, `.bwdevice`, UUID). Not closed. |
| **UI Devices tab** | **Cheat sheet** — aliases from `aliases.yml`. Right-click inserts `.device(<name>)`. |
| **Params** | Use Bitwig's Remote Control / Perform pages: `.page(list)`, `.perform(list)`, `+name:value`. |
| **No per-device param YAMLs** | Old `devices/*.yaml` with `params:` sections are no longer loaded. |
| **Out of scope** | Sampler / Drum Machine insert. No `d "bd hh"`. No auto VST3/LV2 param catalog. |

Add an alias → edit `devices/aliases.yml` (see `devices/README.md`) → hit ↻ in the UI.

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

WIGSCRIPT is **functional but still in development** — expect grammar tweaks.

| Phase / Component | Status |
|-------------------|--------|
| **Authoring** | |
| Fluent structure | ✅ create/rename/delete track, `.device` / `.add`, `.n` / `.beat`, `.mute()`, `.clip(start)` / `.c(0).start` |
| Chain shorthand | ✅ `!name device1 device2 …` (optional `name:kit`) |
| Beat DSL | ✅ `4_` · `2_4` · `off` · `bk2` · explicit `beat:16(…)` (1-based) |
| Colon notes | ✅ `n` / `chord` / `arp:…` with quoted mini-notation |
| Mini-notation | ✅ rests `~`, groups `[…]`, alternation `<…>`, polymetric `{…}`, random `\|`, subdivide `%N`, euclid, suffixes |
| Key / scale | ✅ `k C minor`, scale-degree numbers `n "0 2 4"` |
| Note modifiers | ✅ `.vel` / `.pres` / `.tim` / `.pan` / `.gain` / `.chnz` |
| Device insert | ✅ open Bitwig resolve (UUID / library file); no closed name list |
| Devices UI list | ✅ alias cheat sheet from `devices/aliases.yml` |
| **Performance** | |
| Track Perform page | ✅ `.perform(list)` / `.perform(name=value)` / `+name:value` (inline or pattern-less) |
| Device page | ✅ `.page(list)` / `.page(name=value)` / `+device.name:value` (inline or pattern-less) |
| Device ops | ✅ `+device: on` / `off` / `delete` / `move N` |
| Live triggers | ✅ play/stop, tempo, mute (incl. timed + `@bar`), scene/clip launch |
| Scenes / clips | ✅ create/rename/delete; scene × track clip cells |
| **Open / not supported** | Sequences/ramps/automation 🚧 · Expander edge cases 🚧 · Per-device param YAMLs removed · Sampler / Drum Machine insert, `d "bd hh"`, kit `:909`, bare Tidal, VST3/LV2 param catalog |

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
codewig-cli eval "k C minor"
codewig-cli eval "bass: n \"0 2 4\" .vel(100 80 60)"

# Performance: page-model param snapshots (use list first to see slot names)
codewig-cli eval "t(bass).device(Polymer).page(list)"
codewig-cli eval "t(bass).device(Polymer).page(cutoff=0.3)"
codewig-cli eval "bass: n "c e g" +cutoff:0.3"

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
