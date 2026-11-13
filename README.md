# MapScraper

MapScraper is a high-performance Source Engine utility that extracts a map's referenced content (textures, materials, 3D models, soundscapes, audio files, particle systems, instances, and navigation files) directly into an organized, publish-ready folder.

[![Latest Release](https://img.shields.io/github/downloads/SpySpaille/MapScraper/total)](https://github.com/SpySpaille/MapScraper/releases/latest)
[![Version](https://img.shields.io/github/v/release/SpySpaille/MapScraper)](https://github.com/SpySpaille/MapScraper/releases/latest)
[![License: ISC](https://img.shields.io/badge/License-ISC-blue.svg)](https://opensource.org/licenses/ISC)

---

## Features

- **🧱 Material & Texture Extraction**: Recursively parses `.vmt` files (including `Patch`, `$include`, `$bottommaterial`, and Proxies) to copy referenced `.vtf` and `.hdr.vtf` textures.
- **📐 3D Model Extraction**: Parses `.mdl` files via `source-mdl`, copies all model variants (`.vvd`, `.phy`, `.dx90.vtx`, `.dx80.vtx`, `.sw.vtx`, `.ani`), and extracts all textures mapped onto the model.
- **🔊 Sound Extraction**: Extracts ambient and entity audio, parses `soundscapes_<mapname>.txt`, and automatically cleans Source Engine sound flags (`)`, `*`, `^`, `#`, `~`, `>`).
- **🔄 Func_instance Support**: Resolves and recursively analyzes sub-VMF instance entities, extracting all nested assets and copying instance files for publishing.
- **🔗 Mounted Directories Support**: Automatically detects mounted games from `cfg/mount.cfg` and `gameinfo.txt` (e.g. Counter-Strike: Source, Team Fortress 2, Half-Life 2), and seamlessly collects assets from all mounted folders into your map's output.
- **📤 Ready to Publish & Packing**: Copies the compiled BSP as-is alongside map companion files and source VMFs. Use `--pack-bsp` to embed new or changed companions with `bspzip.exe`.
- **⚡ Offline-First & Optimized**: Uses local bundled asset getters with optional online updates, concurrent async file copying, and duplicate-safe caching.

---

## Installation

- Download the latest standalone executable from **[GitHub Releases](https://github.com/SpySpaille/MapScraper/releases/latest)**.
- Place `MapScraper.exe` in a dedicated working folder.
- Run `MapScraper.exe`.

---

## Usage

```console
$ - 📂 Game Path > C:\Steam\steamapps\common\GarrysMod\garrysmod
$ - 📄 VMF File > C:\Steam\steamapps\common\GarrysMod\garrysmod\maps\rp_downtown_v4c.vmf
$ - 🧱 Extract Materials [y/n] > y
$ - 📐 Extract Models [y/n] > y
$ -    Include foliage models listed in detail VBSP files [y/n] > n
$ - 🔊 Extract Sounds [y/n] > y
$ - 📤 Ready to Publish (copy BSP and companion files) [y/n] > y
$ -    Embed new or changed companion files into the BSP [y/n] > n
```

Use `--include-detail-models` to collect models listed in detail VBSP definitions when you need the full foliage library. Models explicitly referenced by the map are collected whenever model extraction is enabled.

Output:
```console
📂 Game Path: C:\Steam\steamapps\common\GarrysMod\garrysmod
🔗 Mounted directories detected (2):
   - mount.cfg (cstrike): C:\Steam\steamapps\common\Counter-Strike Source\cstrike
   - mount.cfg (tf): C:\Steam\steamapps\common\Team Fortress 2\tf

🔄 • ✅ 4 func_instance VMFs processed
🔊 • ✅ 28 sounds extracted
🧱 • ✅ 142 materials & 180 textures extracted
📐 • ✅ 65 models extracted
📤 • ✅ 6 additional files extracted

Extraction completed 🎉
📁 Output directory: D:\Développement\MapScraper\rp_downtown_v4c
```

---

## Development & Building

Written in **Rust** for maximum execution speed, zero external runtime dependency, and an ultra-compact executable size (~1.4 MB).

```bash
# Run automated tests
cargo test

# Run directly
cargo run

# Build standalone release executable
cargo build --release
```

---

## Author

- **SpySpaille** ([@SpySpaille](https://github.com/SpySpaille))
