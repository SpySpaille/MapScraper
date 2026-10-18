# Changelog

All notable changes to **MapScraper** will be documented in this file.

## [1.8.0] - 2026-10-19

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
- **Source Engine Sound Flag Handling**:
  - Automatic sanitization of sound script flags (`)`, `*`, `^`, `#`, `~`, `>`) to locate audio files reliably on disk.
- **Offline-First Getters Fallback**:
  - Bundled local `Getters/` files are used immediately, with non-blocking remote updates if connected.
- **Automated Test Suite**:
  - Comprehensive unit and integration test suite (`npm test`) covering VMT parsing, VMF parsing, mounts, sound flag cleanup, and instance resolution.

### 🔄 Changed & Refactored
- **Modular Codebase**:
  - Refactored monolithic `main.js` into structured ES/CommonJS modules under `src/`:
    - `src/cli/` (UI, ASCII banner, prompts)
    - `src/core/` (configuration, search path manager, scraper pipeline)
    - `src/parsers/` (robust VMF and VMT KeyValues parsers, instance resolver)
    - `src/extractors/` (materials, 3D models, sounds, publish files)
    - `src/utils/` (filesystem helpers, getters loader, concurrency pool)
- **Dependency Cleanup**:
  - Completely removed unnecessary `express` dependency and phantom HTTP server on port 3000.
  - Reduced dependencies and vulnerabilities.
- **Performance & Reliability**:
  - Added async concurrency limiter (`pLimit`) for batching file copies without exhausting file descriptors.
  - Set-based deduplication caches for materials, textures, models, and sounds.
  - Robust exception handling for unusual or corrupted MDL headers.

---

## [1.7.0] - 2025-06-28
- Handle StarTexture from `env_skypaint`.
- General maintenance.

## [1.6.5] - 2024-10-21
- Maintenance update.

## [1.6.0] - 2024-09-26
- Extraction improvements.
