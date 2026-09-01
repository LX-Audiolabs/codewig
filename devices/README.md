# Device alias catalog

**Source of truth = `devices/aliases.yml`.**
Scanned at startup and on **reload** (codewig-live Devices tab → **↻**).
Nothing is compiled into the binary.

Per-device parameter YAMLs are no longer used. Codewig controls Bitwig through
**Remote Control pages** (device) and **Perform pages** (track) — 8 slots each.
You map the parameters you want on those pages in Bitwig, then address them by
name or slot from WIGSCRIPT.

## Install location (per user)

On first run Codewig creates a user data folder and seeds factory `aliases.yml` + this README into `devices/`
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

## Add or edit an alias

1. Edit `devices/aliases.yml` in the user folder (or this repo folder while developing).
2. In **codewig-live** → Devices → **↻**.
3. The device appears in the list with its aliases.

Example entry:

```yaml
devices:
  polymer:
    bitwig_name: Polymer
    kind: bitwig
    aliases: [poly, Polymer]

  v9kick:
    bitwig_name: V9 Kick
    kind: clap
    aliases: [v9 kick, kick.v9]
```

## Scope

| Supported | Not now |
|-----------|---------|
| **Bitwig** stock/library aliases | VST3/LV2 auto catalog |
| **CLAP** plugin aliases (`kind: clap`) | Auto-discover every plugin on disk |

Insert can still resolve Bitwig library devices by name without an alias.
The **Devices tab** simply makes aliases discoverable.

### Search order

1. `CODEWIG_DEVICES_DIR` (if set)
2. **User** `…/Codewig/devices` (created + seeded on start)
3. `./devices` (cwd) and next to the executable
4. Dev: repo `devices/` last (so local edits win while developing)

Later dirs / later entries with the same `id` win.

## Rules

1. **Aliases present** → listed in UI Devices tab as a **cheat sheet**.
2. **No alias** → not listed in help; **insert + page control still work** by Bitwig name.
3. `kind` must be `bitwig` or `clap`.
4. Community CLAP: add to `aliases.yml` → drop in folder → **↻**.
5. **Not everything needs an alias** — complex devices stay Bitwig-UI / raw CLI.

## WIGSCRIPT page model

List first, then set:

```wigscript
# Track Perform page (8 slots on the track)
t(bass).perform(list)
t(bass).perform(cutoff=0.3, resonance=0.7)

# Device Remote Control page (8 slots on the cursor device)
t(bass).device(Polymer).page(list)
t(bass).device(Polymer).page(cutoff=0.3)

# Inline on a note line
bass: n "c e g" +cutoff:0.3              # track Perform page
bass: n "c e g" +Polymer.cutoff:0.3     # Polymer device page
```

Values are wire-normalized **0..1**.

## Schema

| Field | Meaning |
|-------|---------|
| `id` | Stable id (= YAML key) |
| `bitwig_name` | Name as shown / matched in Bitwig |
| `kind` | `bitwig` \| `clap` |
| `aliases` | Short names usable in `.device(...)` and `+device.name:value` |

## Old per-device YAMLs

`devices/*.yaml` files with a `params:` section are **no longer loaded**. They can
be safely deleted; only `aliases.yml` is used now.
