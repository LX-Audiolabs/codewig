# codewig-live

**Live-Coding-UI für Bitwig Studio** — programmier deine DAW live mit WIGSCRIPT, einer
eigenen Musiksprache, die speziell für Bitwig und Live-Performance entwickelt wurde.

```
┌─────────────────────────────────────────────┐
│  codewig-live (Slint UI)                    │
│  ┌─────────────────────────────────────────┐│
│  │  WIGSCRIPT — Musiksprache für Bitwig    ││
│  │  bass c3 [eb3 g3] ~ bb3*2 | kick ~ ~   ││
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
# Live-Coding direkt in Bitwig
bass c3 [eb3 g3] ~ bb3*2          # Bassline mit Akkorden
kick ~ ~ snare ~                   # Drum-Pattern
chord "Am F C G"                   # Akkordfolge
arp up c3 e3 g3                    # Arpeggio
new track(kick).device(kick.v9).beat(4_).mute().clip(start)
```

- **Mini-Notation wie Tidal/UZU:** `~ [ ] * / < > ! _ ? @ | (,) { } % :`
- **Fluent Chain API:** `track(name).device(dev).beat(spec).clip(action)`
- **11 Skalen + 17 Akkord-Qualitäten + römische Ziffern**
- **40+ Bitwig Devices** mit Aliases (Synths, Drums, FX, Note FX)
- **Euclid-Patterns, Arpeggios, Scene-Control, Clip-Launch, Mute/Solo**

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
| WIGSCRIPT Parser | ✅ 16 Mini-Notation-Tokens, Fluent API, Scene/Clip/Mute/Param |
| Skalen & Akkorde | ✅ 11 Skalen, 17 Qualities, römische Ziffern |
| Device-Katalog | ✅ 40+ Devices mit Bitwig-Mappings |
| Expander (Pattern → MIDI) | ✅ Akkorde, Arpeggios, Euclid, Suffixe |
| CLI (`cliwig`) | ✅ Transport, Tracks, Devices, Clips, Parameter |
| Java Bitwig Extension | ✅ Controller API Bridge |
| Slint UI (`codewig-live`) | ✅ Sidebar mit WIGSCRIPT-Referenz |
| Fluent → Wire-Commands | 🚧 In Arbeit |
| Tests | ✅ 52 Parser-Tests, 0 Clippy-Warnings |

---

## Quickstart

```powershell
# Transport
cliwig play
cliwig set tempo 128

# Track + Device + Clip in einer Zeile
cliwig chain --name bass Polymer Delay+

# Notes in Clip schreiben
cliwig clip note bass 0 0:C3:100:1 4:E3 8:G3

# Live-Clip wechseln
cliwig clip launch bass 0
```
