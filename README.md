# Codewig

**Live coding for Bitwig Studio** — program your DAW live with WIGSCRIPT, a
music language built specifically for Bitwig and live performance.

```
┌─────────────────────────────────────────────┐
│  codewig-live (Slint UI)  ·  codewig-cli    │
│  ┌─────────────────────────────────────────┐│
│  │  WIGSCRIPT — music language for Bitwig  ││
│  │  bass: n "c e g" +cutoff:0.3           ││
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

Both clients talk to the same extension: **`Codewig.bwextension`**.

AI agents can also drive Bitwig through the same interface — but the focus is
**human + machine live on stage**.

---

## WIGSCRIPT — the music language

```wigscript
# Music mode: trackname: action "pattern"
bass: n "c e g" +cutoff:0.3        # notes on track "bass", params inline
drums:909: d "bd hh sd"            # drums with 909 kit

# Chain: track + devices in one line
!bass Polymer Filter Delay+

# Fluent: track + insertable device + pattern
new track(bass).device(Polymer).add(Delay+).n("0 2 4 0")
new track(drums).device(layer)   # pads (v9Kick…) manually in the layer

# Live: clips / scenes / mute (main workflow)
s(1).start                         # scene 1
c(bass.0).start                    # clip slot 0 on track "bass"
mute(kick)                         # mute track
unmute(kick)
```

- **Music mode:** `trackname: n "pattern"` / `d "bd hh"` (mini-notation only inside quotes)
- **Fluent:** `new track(name).device(dev).add(fx)…` — **curated** devices only
- **Insert allowlist (9):** Polymer, Polysynth, Organ, Instrument Layer, Filter, Reverb, Delay+, Chorus+, Saturator  
  (no Sampler, no Drum Machine, no v\* pads via `device.add`)
- **Live focus:** clip/scene launch + mute/unmute; remaining devices are added by the user in Bitwig
- **Drum patterns:** MIDI aliases (`bd`, `kick.v9`…) write notes — pads must already exist

---

## Control

| Path | What |
|------|------|
| **Slint UI** (`codewig-live`) | Graphical interface, sidebar with WIGSCRIPT reference, live input |
| **CLI** (`codewig-cli`) | PowerShell, CMD, Terminal — `codewig-cli play`, `codewig-cli set tempo 120` |
| **Script** (`codewig-cli batch`) | Run commands from a file (`#` = comments) |
| **Extension** (`Codewig.bwextension`) | Bridge for UI + CLI (Controllers → Codewig Bridge) |

---

## Status

| Component | Status |
|-----------|--------|
| WIGSCRIPT parser | ✅ Music mode, chain, fluent, param, scene, clip, mute |
| Mini-notation | ✅ in `"…"` after `n` / `d` / `chord` |
| Device allowlist | ✅ 9 insertable (Java UUID ↔ Rust sync); drums = MIDI only |
| Expander | 🚧 Pattern → MIDI (partial) |
| CLI (`codewig-cli`) | ✅ Transport, tracks, devices, clips, parameters |
| Java extension (`Codewig.bwextension`) | ✅ Bridge; curated `DeviceCatalog` |
| Slint UI (`codewig-live`) | ✅ Sidebar; execute wire 🚧 |
| Fluent → Bitwig | 🚧 Parser ready, executor open |

---

## Quickstart

```powershell
# WIGSCRIPT (same lines as codewig-live UI)
codewig-cli eval "mute(kick)"
codewig-cli eval "s(1).start"
codewig-cli eval "new track(bass).device(Polymer).add(Delay+)"
codewig-cli eval "bass: n \"c e g\""
codewig-cli eval "tempo 128"
codewig-cli eval "play"

# Legacy clap (still usable)
codewig-cli play
codewig-cli set tempo 128
codewig-cli chain --name bass Polymer Delay+
codewig-cli clip launch bass 0
codewig-cli track mute kick

# Batch: WIGSCRIPT lines + legacy mixed
# codewig-cli batch session.wig
```
