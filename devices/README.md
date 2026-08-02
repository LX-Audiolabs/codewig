# Device param catalog

**Source of truth = YAML files on disk.**  
Scanned at startup and on **reload** (codewig-live Devices tab → **↻**).  
Nothing is compiled into the binary.

## Install location (per user)

On first run Codewig creates a user data folder and seeds factory YAMLs into `devices/`
(never overwrites files you already edited):

| OS | Path |
|----|------|
| **Windows** | `%LOCALAPPDATA%\Codewig\devices` |
| **Linux** | `$XDG_DATA_HOME/Codewig/devices` or `~/.local/share/Codewig/devices` (AppImage: same — never write into the image) |
| **macOS** | `~/Library/Application Support/Codewig/devices` |

Parent folder `Codewig/` is the app home — later we can put more under it (presets, logs, …).

| Env | Meaning |
|-----|---------|
| `CODEWIG_HOME` | Override whole user data root |
| `CODEWIG_DEVICES_DIR` | Override only the devices catalog dir |

## Add a device (Bitwig or CLAP)

1. Copy schema from e.g. `v9kick.yaml`.
2. Drop `mydevice.yaml` into the **user** `devices/` folder (or this repo folder while developing).
3. In **codewig-live** → Devices → **↻**.
4. Device appears with params. WIGSCRIPT: `track&device: param(50)`.

No rebuild for new YAML.

## Scope (v1)

| Supported | Not now |
|-----------|---------|
| **Bitwig** stock/library (YAML-documented) | VST3 |
| **CLAP** params via your YAML (`kind: clap`) | LV2 |
| | Auto-discover every plugin on disk |

Insert can still resolve Bitwig library devices by name without YAML.  
**Params + Devices list** need a YAML.

### Search order

1. `CODEWIG_DEVICES_DIR` (if set)
2. **User** `…/Codewig/devices` (created + seeded on start)
3. `./devices` (cwd) and next to the executable
4. Dev: repo `devices/` last (so local edits win while developing)

Later dirs / later files with the same `id` win.

## Rules

1. **YAML present** → listed in UI Devices tab as **help** (aliases + display ranges).
2. **No file** → not listed in help; **insert + raw params still work** (wire 0..1).
3. **Param omitted from YAML** → no help alias/range for it; user can still set raw by Bitwig name.
4. `kind` must be `bitwig` or `clap`.
5. Community CLAP: write YAML → drop in folder → **↻**.
6. **Knobs/sliders only** in help — skip buttons, toggles, mode/type, presets.
7. **Long lists OK**. UI loads light rows; full params on **click**.
8. **Not everything needs YAML** — complex devices stay Bitwig-UI / raw CLI.

## Why YAML

Structured data, one parser (`serde_yaml`), git-diff friendly. No VST3/LV2 kinds.

## Syntax

```
track&device: param(displayValue) other(displayValue)
```

```
kick&v9kick: decay(50) punch(70)
```

## Schema

| Field | Meaning |
|-------|---------|
| `id` | Stable id (= filename without `.yaml`) |
| `bitwig_name` | Name as shown / matched in Bitwig |
| `kind` | `bitwig` \| `clap` only |
| `path_hint` | Human docs — not scanned for resolve yet |
| `aliases` | Names after `&` |
| `params.<name>` | Canonical name for `param.set` name match |
| `params.*.wire` | Range on the wire (usually `[0, 1]`) |
| `params.*.display` | What the user types (e.g. `[0, 100]`) |
| `params.*.unit` | UI hint (`%`, …) |
| `params.*.aliases` | e.g. `pitch` → `tune` |
| `params.*.type` | `float` (default) or `bool` |

Comments with `#` are fine in YAML.
