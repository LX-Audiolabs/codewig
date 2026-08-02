# Codewig

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

---

## WIGSCRIPT — four layers

Do not mix them. Each layer has one job.

| Layer | Role | Example |
|-------|------|---------|
| **Fluent** | Build structure | `new track(lead).device(Polymer).n("c e d g").clip(start)` |
| **Colon** | Notes into existing cells | `lead@verse: n "e c g"` · `bass: n "c e g"` |
| **Param** | Device params (snapshot) | `kick&v9kick: decay(50) pitch(40)` |
| **Performance** | Live triggers | `play` · `mute(kick)` · `s(verse).start` · `c(lead.0).start` |

```wigscript
# Fluent — create track, insert curated device, write notes (slot 0)
new track(bass).device(Polymer).n("c e g").clip(start)
new track(kick).device(v9 kick).beat(4_).clip(start)
new track(lead).device(Polymer).add(Delay+)

# Colon — write notes into an existing track / scene cell (quotes required)
bass: n "c e g"
lead@verse: n "e c g"
lead: arp:up "Cm7"
lead: chord "C Am F G"

# Param — track × device only (& not @). Display ranges from devices/*.yaml → wire 0..1
kick&v9kick: decay(50) pitch(40)

# Performance
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
- `.c` / `.clip` = clip cell (new/start/stop), **not** notes
- `.device(...)` / `.add(...)` = insert allowlist only

### Insert allowlist

**Synths / shell / FX (UUID):** Polymer, Polysynth, Organ, Instrument Layer, Filter, Reverb, Delay+, Chorus+, Saturator

**Stock mono drums (file insert):** `v9 kick`, `v9 snare`, `v0 hat`, … (all v0/v1/v8/v9 modules)

**Not insertable:** Sampler, Drum Machine. No kit syntax (`:909`), no hit-map `d "bd hh"`.

### Params

- Address = **`track&device:`** (`@` is scene/slot only)
- Snapshot only (`param.set`) — no automation ramps
- Catalog = `devices/*.yaml` (Bitwig + CLAP). No YAML / empty params → insert may work, **params do not**
- Polymer: insert + notes OK; **params deferred** (empty catalog until a fixed subset is documented)

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

| Component | Status |
|-----------|--------|
| Fluent | ✅ create track, device insert, `.n` / `.beat`, mute, clip |
| Colon notes | ✅ `n` / `chord` / `arp` with quoted mini-notation |
| Param (`track&device:`) | ✅ catalog snapshot (`devices/*.yaml`; v9 Kick shipped) |
| Performance | ✅ play/stop, tempo, mute (incl. timed), scene/clip launch |
| Mini-notation | ✅ spaces, rests `~`, groups `[…]` |
| Device allowlist | ✅ curated insert list (Java UUID ↔ Rust); drums via file |
| Expander | 🚧 pattern → MIDI (partial edge cases) |
| Polymer params | ⏸ deferred (empty YAML) |
| **Not supported** | `d "bd hh"`, kit suffixes (`:909`), bare Tidal lines, VST3/LV2 params |

---

## Quickstart

Bitwig running with **Codewig Bridge** loaded, then:

```powershell
# Build structure
codewig-cli eval "new track(bass).device(Polymer).n(\"c e g\").clip(start)"
codewig-cli eval "new track(kick).device(v9 kick).beat(4_).clip(start)"

# Notes into existing cells
codewig-cli eval "bass: n \"c e g\""
codewig-cli eval "lead@verse: n \"e c g\""

# Params (device must be on the track; YAML required)
codewig-cli eval "kick&v9kick: decay(50) pitch(40)"

# Live
codewig-cli eval "new scene(verse)"
codewig-cli eval "s(verse).start"
codewig-cli eval "mute(kick)"
codewig-cli eval "tempo 128"
codewig-cli eval "play"

# Same lines work in codewig-live UI input
# Batch: codewig-cli batch session.wig
```

Legacy flat CLI tokens still work (`codewig-cli play`, `track mute kick`, …) as a fallback — prefer WIGSCRIPT `eval`.
