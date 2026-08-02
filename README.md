# Codewig

**Live-Coding für Bitwig Studio** — programmier deine DAW live mit WIGSCRIPT, einer
eigenen Musiksprache, die speziell für Bitwig und Live-Performance entwickelt wurde.

```
┌─────────────────────────────────────────────┐
│  codewig-live (Slint UI)  ·  codewig-cli    │
│  ┌─────────────────────────────────────────┐│
│  │  WIGSCRIPT — Musiksprache für Bitwig    ││
│  │  bass: n "c e g" +cutoff:0.3           ││
│  └─────────────────────────────────────────┘│
│         ↓  TCP + JSON  (localhost :9470)     │
│  Codewig.bwextension  (Bridge)               │
│         ↓  Controller API                    │
│  Bitwig Studio                               │
└─────────────────────────────────────────────┘
```

## Was ist Codewig?

Eine **Slint-UI** (`codewig-live`) und eine **CLI** (`codewig-cli`) mit einer **fast
eigenen Coding-Sprache für Musik** — entwickelt für **Live-Performance in Bitwig**.
Du steuerst Tracks, Devices, Clips und Parameter über die grafische Oberfläche oder
direkt via **CMD / PowerShell / Terminal**.

Beide Clients sprechen mit derselben Extension: **`Codewig.bwextension`**.

Zusätzlich können auch AI-Agents Bitwig über die gleiche Schnittstelle ansteuern — aber
der Fokus liegt auf **Mensch + Maschine live auf der Bühne**.

---

## WIGSCRIPT — die Musiksprache

```wigscript
# Music-Mode: trackname: aktion "pattern"
bass: n "c e g" +cutoff:0.3        # Noten in Track "bass", Parameter inline
drums:909: d "bd hh sd"            # Drums mit 909-Kit

# Chain: Track + Devices in einer Zeile
!bass Polymer Filter Delay+

# Fluent: Track + insertable Device + Pattern
new track(bass).device(Polymer).add(Delay+).n("0 2 4 0")
new track(drums).device(layer)   # pads (v9Kick…) manuell im Layer

# Live: Clips / Scenes / Mute (Hauptworkflow)
s(1).start                         # Scene 1
c(bass.0).start                    # Clip Slot 0 auf Track "bass"
mute(kick)                         # Track stumm
unmute(kick)
```

- **Music-Mode:** `trackname: n "pattern"` / `d "bd hh"` (Mini-Notation nur in Quotes)
- **Fluent:** `new track(name).device(dev).add(fx)…` — nur **curated** Devices
- **Insert-Allowlist (9):** Polymer, Polysynth, Organ, Instrument Layer, Filter, Reverb, Delay+, Chorus+, Saturator  
  (kein Sampler, keine Drum Machine, keine v\* Pads via `device.add`)
- **Live-Fokus:** Clip/Scene launch + mute/unmute; restliche Devices legt der User in Bitwig an
- **Drum-Patterns:** MIDI-Aliases (`bd`, `kick.v9`…) schreiben Notes — Pads müssen schon existieren

---

## Steuerung

| Weg | Was |
|-----|-----|
| **Slint UI** (`codewig-live`) | Grafische Oberfläche, Sidebar mit WIGSCRIPT-Referenz, Live-Eingabe |
| **CLI** (`codewig-cli`) | PowerShell, CMD, Terminal — `codewig-cli play`, `codewig-cli set tempo 120` |
| **Skript** (`codewig-cli batch`) | Befehle aus Datei ausführen (`#` = Kommentare) |
| **Extension** (`Codewig.bwextension`) | Bridge für UI + CLI (Controllers → Codewig Bridge) |

---

## Status

| Komponente | Stand |
|------------|-------|
| WIGSCRIPT Parser | ✅ Music-Mode, Chain, Fluent, Param, Scene, Clip, Mute |
| Mini-Notation | ✅ in `"…"` hinter `n`/`d`/`chord` |
| Device-Allowlist | ✅ 9 insertable (Java UUID ↔ Rust sync); Drums = MIDI only |
| Expander | 🚧 Pattern → MIDI (teilweise) |
| CLI (`codewig-cli`) | ✅ Transport, Tracks, Devices, Clips, Parameter |
| Java Extension (`Codewig.bwextension`) | ✅ Bridge; curated `DeviceCatalog` |
| Slint UI (`codewig-live`) | ✅ Sidebar; Execute-Wire 🚧 |
| Fluent → Bitwig | 🚧 Parser fertig, Executor offen |

---

## Quickstart

```powershell
# WIGSCRIPT (gleiche Zeilen wie codewig-live UI)
codewig-cli eval "mute(kick)"
codewig-cli eval "s(1).start"
codewig-cli eval "new track(bass).device(Polymer).add(Delay+)"
codewig-cli eval "bass: n \"c e g\""
codewig-cli eval "tempo 128"
codewig-cli eval "play"

# Legacy clap (weiter nutzbar)
codewig-cli play
codewig-cli set tempo 128
codewig-cli chain --name bass Polymer Delay+
codewig-cli clip launch bass 0
codewig-cli track mute kick

# Batch: WIGSCRIPT-Zeilen + legacy gemischt
# codewig-cli batch session.wig
```
