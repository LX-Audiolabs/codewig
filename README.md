# CLIwig

CLI + thin Bitwig extension so humans and agents can drive Bitwig from the shell — like `cargo check`, for your DAW.

```
cliwig play | cliwig set tempo 120 | cliwig status
        ↓  TCP + JSON  (localhost :9470)
Java extension  (.bwextension)
        ↓  Controller API
Bitwig Studio
```

**Not** a Moss clone, **not** MCP-in-Bitwig. Extension = cable. Intelligence stays with the caller.

Design notes: [`research/`](./research/) — start with [`DECISIONS.md`](./research/DECISIONS.md).

---

## Status

**Phase 1a (now):** transport bridge

| Command | What |
|---------|------|
| `cliwig ping` | health check |
| `cliwig status` | playing, tempo, port |
| `cliwig play` / `cliwig stop` | transport |
| `cliwig set tempo <bpm>` | project tempo |
| `cliwig track new --name bass` | instrument track at end (`--at -1`) |
| `cliwig track new --name lead --at 0` | insert instrument track at top |
| `cliwig track list` / `select` / `rename` / `delete` / `move` | tracks |
| `cliwig track rename "Inst 1" bass` | track umbenennen |
| `cliwig track mute 1 3 6` / `track solo 0 2` | multi mute/solo (`--off` to clear) |
| `cliwig track volume bass 0.8` | volume 0..1 |
| `cliwig device add Polymer` | native device on selected track |
| `cliwig param list` / `param set` | direct params |
| `cliwig clip new bass --name A` | empty launcher clip (first free slot) |
| `cliwig clip launch bass 0` / `clip list bass` | live clip switch |
| `cliwig clip note bass 0 0:C3:100:1 4:E3` | write MIDI notes (step:key[:vel[:dur]], C3 = 60) |
| `cliwig clip clear-notes bass 0` | clear clip (`--step 0 --key C3` for one cell) |
| `cliwig batch session.cliwig` | run commands from file, one per line (`#` = comment) |
| `cliwig completions powershell` | shell completion script generieren |
| **`cliwig chain --name bass Polymer Delay+`** | **one line → track + devices (+ clip A)** |

### Workflow (3 steps — not Strudel live-coding)

One command → Enter → next. We drive a DAW; we don’t write a music script.

```powershell
# 1 ADD  — structure (+ empty clip A for live)
cliwig chain --name bass Polymer Delay+
cliwig clip new bass --name B --beats 4    # second slot for switching

# 2 SET / 3 PARAM
cliwig device select 0
cliwig param list
cliwig param set --set cutoff=0.3

# Live
cliwig clip note bass 0 0:C3:100:1 4:E3 8:G3   # notes into clip
cliwig clip launch bass 0
cliwig clip launch bass 1
cliwig track mute 1 3 6
cliwig track solo 0 2
cliwig track solo 0 2 --off

# Batch (one connection, stops on first error, compact JSON per line)
cliwig batch session.cliwig
# file content example:
#   chain --name bass Polymer Delay+
#   clip new bass --name A
#   clip note bass 0 0:C3:100:1 4:E3 8:G3
#   clip launch bass 0
```

Design: **ADD → SET → PARAM** + Clip Launcher live — see DECISIONS §5.

**Praxis-Prinzip:** Loop, Metronom, Clip-Launch-Quantization und andere globale Projekt-Einstellungen bleiben in Bitwig. CLIwig steuert Tracks, Devices, Clips, Parameter und Noten — nicht das Projekt-Setup.

**Next:** WIGSCRIPT language — UZU-adapted flat syntax for Bitwig: `bass: n "c e g" +cutoff:0.3`  
Specs: [`research/MUSIC-SPEC.md`](./research/MUSIC-SPEC.md) · [`research/WIGSCRIPT.md`](./research/WIGSCRIPT.md) · [`research/API-REALITY-CHECK.md`](./research/API-REALITY-CHECK.md)  
**Now:** **`codewig-live`** (Slint) · Music-Mode REPL mit `♫` Prompt · Bitwig-native Devices statt Samples.

Surfaces (locked): CLI · **Slint app** (no browser) — [`research/DECISIONS.md`](./research/DECISIONS.md).

---

## Prerequisites

- Bitwig Studio (Controller API ≥ 18)
- JDK 21+ (to build the extension)
- Rust stable (to build `cliwig`)

---

## Build

### Extension

```powershell
cd extension
# first time: bootstrap Gradle wrapper (see below) or use a system Gradle
.\gradlew.bat jar
# → extension\build\libs\CLIwig.bwextension
.\gradlew.bat installExtension   # copies into Documents\Bitwig Studio\Extensions
```

Without wrapper yet, from a machine with Gradle 8+:

```powershell
cd extension
gradle jar
gradle installExtension
```

### Rust workspace (CLI + core + codewig-live)

```powershell
cargo build --workspace --release
# binaries:
#   target\release\cliwig.exe
#   target\release\codewig-live.exe

# Nur CLI installieren
cargo install --path cli

# Nur UI installieren
cargo install --path ui
```

### Shell-Autovervollständigung

```powershell
# PowerShell (aktuelle Session)
cliwig completions powershell | Out-String | Invoke-Expression

# Dauerhaft: in dein PowerShell-Profil schreiben
cliwig completions powershell | Out-File (Join-Path $PROFILE "..\cliwig-completion.ps1") -Encoding utf8
# und im Profil dot-sourcen:
# . (Join-Path $PROFILE "..\cliwig-completion.ps1")
```

Weitere Shells: `cliwig completions bash|zsh|fish|elvish`.

---

## Enable in Bitwig

1. Copy `CLIwig.bwextension` to  
   `%USERPROFILE%\Documents\Bitwig Studio\Extensions\`  
   (or run `gradlew installExtension`)
2. Restart Bitwig (or reload extensions)
3. **Settings → Controllers → + Add controller → CLIwig → CLIwig**
4. Popup: *CLIwig ready on port 9470*
5. Optional: controller preferences → **Network / Port**

Windows may prompt for firewall access on first load — allow for private networks (localhost only).

---

## Use

```powershell
cliwig ping
cliwig set tempo 120
cliwig track new --name bass
cliwig track new --name lead --at 0
cliwig track list
cliwig track select bass
cliwig device add Polymer
cliwig device add Dynamics
cliwig device list
cliwig track move bass --to 0
cliwig play
cliwig status
cliwig stop
```

Globals:

```text
--host 127.0.0.1          # or CLIWIG_HOST
--port 9470              # or CLIWIG_PORT
--timeout-ms 2000
```

---

## Protocol (for integrators)

Bitwig `RemoteConnection` framing: **4-byte big-endian length** + UTF-8 JSON.

```json
→ {"id":1,"c":"ping"}
← {"id":1,"ok":true}

→ {"id":2,"c":"set","k":"tempo","v":120}
← {"id":2,"ok":true}

→ {"id":3,"c":"status"}
← {"id":3,"ok":true,"result":{"bitwig":"connected","playing":false,"tempo":120.0,"port":9470}}
```

---

## Layout

```text
CLIwig/
  research/     # vision, decisions, prior art
  extension/    # Java → CLIwig.bwextension
  core/         # Rust library `cliwig-core` (protocol + shared client)
  cli/          # Rust binary `cliwig`
  ui/           # Rust + Slint binary `codewig-live`
  README.md
```

---

## Credits / Inspiration

CLIwig is heavily inspired by:

- **[DrivenByMoss](https://github.com/git-moss/DrivenByMoss)** by Jürgen Moßgraber — the de-facto reference for Bitwig Controller API extensions. We studied its architecture and patterns while building our own thin bridge.
- **[Strudel](https://strudel.cc)** by Felix Roos — for the vision of code-first, live-friendly music expression. CLIwig does not try to be Strudel; we borrow the spirit of "type music, get sound" for the Bitwig context.

---

## License

MIT (planned) — tbd when published.
