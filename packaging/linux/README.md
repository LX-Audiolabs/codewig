# Linux packaging (AppImage)

## Assets

| File | Role |
|------|------|
| `codewig-live.desktop` | FreeDesktop launcher entry |
| `../../assets/icon.svg` | Vector master |
| `../../assets/icon-256.png` / `icon-512.png` | Raster for AppImage / desktop |

## AppImage (outline)

Build release binary on Linux, then:

```bash
# after: cargo build -p codewig-live --release
#        cargo build -p codewig-cli --release   # optional, same AppImage or separate
mkdir -p AppDir/usr/bin AppDir/usr/share/applications \
         AppDir/usr/share/icons/hicolor/256x256/apps \
         AppDir/usr/share/icons/hicolor/512x512/apps \
         AppDir/usr/share/codewig/devices

cp target/release/codewig-live AppDir/usr/bin/
# optional: cp target/release/codewig-cli AppDir/usr/bin/
cp packaging/linux/codewig-live.desktop AppDir/usr/share/applications/
cp assets/icon-256.png AppDir/usr/share/icons/hicolor/256x256/apps/codewig-live.png
cp assets/icon-512.png AppDir/usr/share/icons/hicolor/512x512/apps/codewig-live.png
# Factory device YAMLs (seeded into user dir on first run — AppImage is read-only)
cp devices/*.yaml AppDir/usr/share/codewig/devices/
# optional AppDir root copies for linuxdeploy
cp packaging/linux/codewig-live.desktop AppDir/
cp assets/icon-256.png AppDir/codewig-live.png
# AppRun = entry point the AppImage runtime execs (missing AppRun = "execv error")
printf '#!/bin/sh\nHERE="$(dirname "$(readlink -f "$0")")"\nexec "$HERE/usr/bin/codewig-live" "$@"\n' > AppDir/AppRun
chmod +x AppDir/AppRun

# then: linuxdeploy --appdir AppDir --output appimage
# or:   appimagetool AppDir
```

`Icon=codewig-live` in the `.desktop` must match the installed icon basename.

## User data (AppImage-safe)

The AppImage filesystem is **read-only**. Codewig never writes inside the mount.

| What | Where |
|------|--------|
| Writable home | `~/.local/share/Codewig/` (`$XDG_DATA_HOME/Codewig`) |
| Device YAML | `…/Codewig/devices/` |
| Factory seed | Copied from `$APPDIR/usr/share/codewig/devices` on first run (missing files only) |

Runtime also sets `APPDIR` while the AppImage runs — we use that to find factory YAML.
