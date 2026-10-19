# Changelog

All notable changes to **MapScraper** will be documented in this file.

## [1.8.0] - 2026-10-19

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
