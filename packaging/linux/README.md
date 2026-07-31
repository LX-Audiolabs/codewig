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
mkdir -p AppDir/usr/bin AppDir/usr/share/applications \
         AppDir/usr/share/icons/hicolor/256x256/apps \
         AppDir/usr/share/icons/hicolor/512x512/apps

cp target/release/codewig-live AppDir/usr/bin/
cp packaging/linux/codewig-live.desktop AppDir/usr/share/applications/
cp assets/icon-256.png AppDir/usr/share/icons/hicolor/256x256/apps/codewig-live.png
cp assets/icon-512.png AppDir/usr/share/icons/hicolor/512x512/apps/codewig-live.png
# optional AppDir root copies for linuxdeploy
cp packaging/linux/codewig-live.desktop AppDir/
cp assets/icon-256.png AppDir/codewig-live.png

# then: linuxdeploy --appdir AppDir --output appimage
# or:   appimagetool AppDir
```

`Icon=codewig-live` in the `.desktop` must match the installed icon basename.
