# Brand assets — codewig-live

| File | Use |
|------|-----|
| `icon.svg` | Vector master (edit this) |
| `icon-source.png` | 1024² raster master |
| `icon.png` / `icon-512.png` | 512² |
| `icon-256.png` … `icon-16.png` | Desktop / taskbar sizes |
| `icon.ico` | Windows EXE embed (multi-size) |

**Palette:** bg `#171717` · accent `#ff731a` (matches Slint UI).

**Windows:** `ui/build.rs` embeds `icon.ico` into `codewig-live.exe`.

**Linux:** `packaging/linux/codewig-live.desktop` + PNG/SVG for AppImage.
