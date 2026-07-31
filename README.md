# codewig-live

**Live-Coding-UI für Bitwig Studio** — programmier deine DAW live mit WIGSCRIPT, einer
eigenen Musiksprache, die speziell für Bitwig und Live-Performance entwickelt wurde.

```
┌─────────────────────────────────────────────┐
│  codewig-live (Slint UI)                    │
│  ┌─────────────────────────────────────────┐│
│  │  WIGSCRIPT — Musiksprache für Bitwig    ││
│  │  bass: n "c e g" +cutoff:0.3           ││
│  └─────────────────────────────────────────┘│
│         ↓  TCP + JSON  (localhost :9470)     │
│  Java Extension  (.bwextension)              │
│         ↓  Controller API                    │
│  Bitwig Studio                               │
└─────────────────────────────────────────────┘
```

## Was ist codewig-live?

Eine **Slint-UI** mit einer **fast eigenen Coding-Sprache für Musik** — entwickelt für
**Live-Performance in Bitwig**. Du steuerst Tracks, Devices, Clips und Parameter entweder
über die grafische Oberfläche oder direkt via **CMD / PowerShell / Terminal** mit `cliwig`.

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
| **CLI** (`cliwig`) | PowerShell, CMD, Terminal — `cliwig play`, `cliwig set tempo 120` |
| **Skript** (`cliwig batch`) | Befehle aus Datei ausführen (`#` = Kommentare) |

---

## Status

| Komponente | Stand |
|------------|-------|
| WIGSCRIPT Parser | ✅ Music-Mode, Chain, Fluent, Param, Scene, Clip, Mute |
| Mini-Notation | ✅ in `"…"` hinter `n`/`d`/`chord` |
| Device-Allowlist | ✅ 9 insertable (Java UUID ↔ Rust sync); Drums = MIDI only |
| Expander | 🚧 Pattern → MIDI (teilweise) |
| CLI (`cliwig`) | ✅ Transport, Tracks, Devices, Clips, Parameter |
| Java Extension | ✅ Bridge; curated `DeviceCatalog` |
| Slint UI | ✅ Sidebar; Execute-Wire 🚧 |
| Fluent → Bitwig | 🚧 Parser fertig, Executor offen |

---

## Quickstart

```powershell
# WIGSCRIPT (gleiche Zeilen wie codewig-live UI)
cliwig eval "mute(kick)"
cliwig eval "s(1).start"
cliwig eval "new track(bass).device(Polymer).add(Delay+)"
cliwig eval "bass: n \"c e g\""
cliwig eval "tempo 128"
cliwig eval "play"

# Legacy clap (weiter nutzbar)
cliwig play
cliwig set tempo 128
cliwig chain --name bass Polymer Delay+
cliwig clip launch bass 0
cliwig track mute kick

# Batch: WIGSCRIPT-Zeilen + legacy gemischt
# cliwig batch session.wig
```
