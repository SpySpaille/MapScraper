# Changelog

All notable changes to **MapScraper** will be documented in this file.

## [1.8.1] - 2026-10-19

### 🐛 Fixed
- **Source Engine Compiled Patch Materials & WVT Resolution (`_wvt_patch`)**:
  - Automatically resolves Source Engine compiled patch materials (`maps/<mapname>/..._wvt_patch.vmt`, cubemap materials `_<x>_<y>_<z>`, and water depth variations) to their master base materials and underlying textures (`$basetexture`, `$basetexture2`, `$bumpmap`, `$ssbump`, etc.).
  - Extracts both the compiled patch material and the original full-fidelity base materials and textures.
- **Robust Multi-Encoding Support (Windows-1252 / ANSI / UTF-8)**:
  - Fixed VMF parser failure on instance files saved by Hammer Editor in Windows-1252/ANSI with non-UTF8 bytes, ensuring non-UTF8 sub-instances and all their brush/entity materials are never dropped.
- **VMT Comment Stripping & Texture Extraction Fix**:
  - Fixed a critical parser bug where single-line comments (`// ...`) on Windows CRLF (`\r\n`) lines failed to be stripped properly.
  - This previously caused key-value pairs in VMT files (such as `$basetexture`, `$bumpmap`, `$basetexture2`, `$bumpmap2`, `$detail`, etc.) to be shifted or skipped, resulting in missing VTF textures in the output.
- **Source Engine Case-Sensitivity Fix for Textures & Materials**:
  - In Source Engine (and on Linux servers or packaged BSP/addons), paths in VMT and game assets are case-sensitive or referenced in lowercase (`custom_folder/subfolder/texture_n`).
  - Previously, when assets on disk or in VMF had uppercase directories (`CUSTOM_FOLDER/SUBFOLDER`), the copied destination files retained uppercase characters while the game engine requested lowercase paths, causing broken/pink checkerboard textures in-game.
  - All extracted materials (`.vmt`) and textures (`.vtf` / `.hdr.vtf`) are now normalized to lowercase destination paths while resolving source files case-insensitively across all mounted search paths.
- **Strict BSP Companion Packing**:
  - Confirmed and enforced that "Ready to Publish" packfile only injects auxiliary companion files (`.nav`, `.ain`, thumbnail, soundscapes, `_particles.txt`) and never dumps loose models/materials/sounds into the BSP pakfile when packaging for addon distribution.

### 🚀 Added
- **VMF Detail VBSP (`detailvbsp`) Support & Foliage Model Extraction**:
  - Automatically parses `"detailvbsp`" properties in `worldspawn` (and sub-instances).
  - Copies the custom `.vbsp` detail definition file directly into the output root during `"Ready to Publish`".
  - Recursively inspects the `.vbsp` file for referenced foliage models (`.mdl`) and triggers full model, material, and texture scraping when model extraction is enabled.
- **Native BSP Pakfile Asset Provider (`Lump 40`)**:
  - Automatically mounts the map's `.bsp` pakfile archive directly as an asset provider, allowing seamless extraction of packed materials, textures, and cubemaps.
- **Unified Virtual File System (VFS)**:
  - Full asset resolution across loose directories, VPK v1/v2 packages, Garry's Mod GMA addons, and BSP pakfiles.
- **Dynamic Terminal Spinners**:
  - Added smooth rotating animated spinners (`| / - \`) during VMF parsing, func_instance recursive resolution, sound extraction, material & texture scraping, 3D model processing, and BSP packing.
  - Eliminated frozen console screens during large map extractions.
- **Hierarchical Prompts & Question Connectors**:
  - Clean tree-style branch connectors (`└─`, `├─`) for nested configuration prompts (e.g. game path reuse prompts).
- **Cleaner Mount Log Output**:
  - Reduced verbosity when numerous search paths or loose addons are detected, aggregating them into a structured tree summary rather than flooding the console screen.

### ⚡ Complete Rust Rewrite & Ultra-Lightweight Binary
- **Ground-up rewrite in Rust**:
  - Replaced the legacy Node.js/pkg bundle (~39 MB) with a blazing-fast, compiled native Windows binary.
  - Executable size reduced from **38.6 MB down to 472 KB** (an **~99% size reduction**).
  - Instant startup with zero V8 engine overhead, zero OpenSSL bloat (using Windows native Schannel), and zero runtime dependencies.
- **Embedded Custom Icon**:
  - Custom favicon/app icon directly compiled and embedded into PE executable resources (`.rsrc`) via `winresource`.

### 🚀 Added
- **`func_instance` Support**:
  - Full recursive parsing of Source Engine `func_instance` sub-VMF files.
  - Automatic resolution of instance paths (relative to map, `maps/instances/`, game path, and mounted games).
  - Cycle detection using visited sets to prevent infinite loops on recursive references.
  - Inclusion of instance `.vmf` files in output when publishing.
- **Mounted Directories Support**:
  - Automatic discovery of mounted games from `cfg/mount.cfg` (Garry's Mod mounts like Counter-Strike: Source, Team Fortress 2, etc.).
  - Automatic discovery of `SearchPaths` from `gameinfo.txt`.
  - Seamless asset resolution across all mounted directories, copying external assets into the map's single distribution package.
- **Native StudioMDL Binary Parser**:
  - Implemented exact Valve StudioMDL binary header parsing (`IDST`, textures offset 204, texture dirs offset 212) directly in Rust.
- **Source Engine Sound Flag Handling**:
  - Automatic sanitization of sound script flags (`)`, `*`, `^`, `#`, `~`, `>`) to locate audio files reliably on disk.
- **Offline-First Bundled Getters**:
  - Embedded local asset getters compiled directly into the binary with zero network delay.
- **Automated Native Test Suite**:
  - Comprehensive unit and integration test suite (`cargo test`) covering real MDL binary parsing, VMT parsing, VMF parsing, mounts, sound flag cleanup, and recursive instance resolution.

### 🧹 Removed & Cleaned
- Removed Node.js, `npm`, `package.json`, `node_modules`, and the legacy monolithic `main.js`.
- Removed phantom web server dependencies (`express`).

---

## [1.7.0] - 2025-06-28
- Handle StarTexture from `env_skypaint`.
- General maintenance.

## [1.6.5] - 2024-10-21
- Maintenance update.

## [1.6.0] - 2024-09-26
- Extraction improvements.
