# Device param catalog

Curated **YAML** defs for devices WIGSCRIPT can set params on.

## Scope (v1)

| Supported | Not now |
|-----------|---------|
| **Bitwig** factory / library devices | VST3 |
| **CLAP** in OS system paths only | LV2 |
| | Random Bitwig plugin-location queries |

We do **not** poll Bitwig for search paths. Stock Bitwig + well-known CLAP dirs only.  
Optional extra CLAP folders = future UI setting (not yet).

### Known paths (reference)

**Bitwig library (user)**

| OS | Typical |
|----|---------|
| Windows | `%USERPROFILE%\Documents\Bitwig Studio\Library\…` |
| Linux | `~/Bitwig Studio/Library/…` |

**CLAP (system)**

| OS | Typical |
|----|---------|
| Windows | `%COMMONPROGRAMFILES%\CLAP`, `%LOCALAPPDATA%\Programs\Common\CLAP` |
| Linux | `/usr/lib/clap`, `/usr/local/lib/clap`, `~/.clap`, `~/.local/share/clap` |

## Rules

1. **YAML present** → device is param-aware (even if `params: {}`).
2. **No file** → insert may still work via insert allowlist; **params unsupported**.
3. **Param omitted** → tool does not know it (no set / no autocomplete).
4. `kind` must be `bitwig` or `clap`. Other kinds are ignored.
5. Extra defs: drop `*.yaml` here or set `CODEWIG_DEVICES_DIR`.

## Why YAML (not MD / not TOML)

- We only needed structured data — MD frontmatter was a wrapper around YAML.
- One format, one parser (`serde_yaml`), git-diff friendly, no `---` split.
- TOML fine too; YAML already in core — no second dep.

## Syntax

```
track&device: param(displayValue) other(displayValue)
```

```
kick&v9kick: decay(50) pitch(40)
```

## Schema

| Field | Meaning |
|-------|---------|
| `id` | Stable id (= filename without `.yaml`) |
| `bitwig_name` | Name as shown / matched in Bitwig |
| `kind` | `bitwig` \| `clap` only |
| `path_hint` | Human docs (`windows` / `linux` strings) — not scanned for resolve yet |
| `aliases` | Names after `&` |
| `params.<name>` | Canonical name for `param.set` name match |
| `params.*.wire` | Range on the wire (usually `[0, 1]`) |
| `params.*.display` | What the user types (e.g. `[0, 100]`) |
| `params.*.unit` | UI hint (`%`, …) |
| `params.*.aliases` | e.g. `res` / `resonance` |
| `params.*.type` | `float` (default) or `bool` |

Comments with `#` are fine in YAML.
