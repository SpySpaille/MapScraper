const fs = require('fs');
const path = require('path');
const { removeQuotes, fileExists } = require('../utils/fileSystem');

class SearchPathManager {
  /**
   * @param {string} gamePath - Primary game directory (e.g. GarrysMod/garrysmod)
   * @param {string[]} [customMounts=[]] - Additional user-provided mount paths
   */
  constructor(gamePath, customMounts = []) {
    this.primaryPath = path.resolve(removeQuotes(gamePath));
    this.customMounts = customMounts.map(m => path.resolve(removeQuotes(m)));
    this.detectedMounts = [];
    this.allPaths = [];

    this.init();
  }

  init() {
    this.detectedMounts = this.discoverMounts();
    // Order: primary game path first, then detected mounts, then custom mounts
    const seen = new Set();
    this.allPaths = [];

    const addPath = (p, label) => {
      const normalized = path.resolve(p);
      if (!seen.has(normalized.toLowerCase()) && fileExists(normalized)) {
        seen.add(normalized.toLowerCase());
        this.allPaths.push({ path: normalized, label });
      }
    };

    addPath(this.primaryPath, 'Primary Game');

    for (const mount of this.detectedMounts) {
      addPath(mount.path, mount.name);
    }

    for (const custom of this.customMounts) {
      addPath(custom, 'Custom Mount');
    }
  }

  /**
   * Scans mount.cfg and gameinfo.txt to discover mounted directories.
   * @returns {Array<{name: string, path: string}>}
   */
  discoverMounts() {
    const discovered = [];
    const seenPaths = new Set();

    // 1. Scan garrysmod/cfg/mount.cfg
    const mountCfgPath = path.join(this.primaryPath, 'cfg', 'mount.cfg');
    if (fileExists(mountCfgPath)) {
      try {
        const content = fs.readFileSync(mountCfgPath, 'utf-8');
        const lines = content.split(/\r?\n/);
        for (const line of lines) {
          const cleanLine = line.split('//')[0].trim();
          if (!cleanLine) continue;
          // Match "name" "path" or name "path"
          const match = cleanLine.match(/"([^"]+)"\s+"([^"]+)"/);
          if (match) {
            const name = match[1].trim();
            const mountDir = path.resolve(match[2].trim());
            if (name.toLowerCase() !== 'mountcfg' && fileExists(mountDir) && !seenPaths.has(mountDir.toLowerCase())) {
              seenPaths.add(mountDir.toLowerCase());
              discovered.push({ name: `mount.cfg (${name})`, path: mountDir });
            }
          }
        }
      } catch (err) {
        // ignore parse error
      }
    }

    // 2. Scan gameinfo.txt for SearchPaths
    const gameinfoPath = path.join(this.primaryPath, 'gameinfo.txt');
    if (fileExists(gameinfoPath)) {
      try {
        const content = fs.readFileSync(gameinfoPath, 'utf-8');
        const searchPathsRegex = /SearchPaths\s*\{([^}]+)\}/is;
        const match = content.match(searchPathsRegex);
        if (match) {
          const block = match[1];
          const lines = block.split(/\r?\n/);
          for (const line of lines) {
            const cleanLine = line.split('//')[0].trim();
            if (!cleanLine) continue;
            // e.g. Game |gameinfo_path|. or Game cstrike or Game hl2
            const parts = cleanLine.split(/\s+/).map(p => removeQuotes(p));
            if (parts.length >= 2) {
              const target = parts[1];
              if (target && !target.includes('|gameinfo_path|') && !target.startsWith('|')) {
                // Check relative to game parent (common/GarrysMod/<target>)
                const candidate1 = path.resolve(path.dirname(this.primaryPath), target);
                if (fileExists(candidate1) && !seenPaths.has(candidate1.toLowerCase())) {
                  seenPaths.add(candidate1.toLowerCase());
                  discovered.push({ name: `gameinfo.txt (${target})`, path: candidate1 });
                }
              }
            }
          }
        }
      } catch (err) {
        // ignore parse error
      }
    }

    // 3. Scan extracted legacy addons folder (garrysmod/addons/*)
    const addonsDir = path.join(this.primaryPath, 'addons');
    if (fileExists(addonsDir)) {
      try {
        const entries = fs.readdirSync(addonsDir, { withFileTypes: true });
        for (const entry of entries) {
          if (entry.isDirectory()) {
            const addonPath = path.join(addonsDir, entry.name);
            // Check if directory contains standard source folders
            if (fileExists(path.join(addonPath, 'materials')) || fileExists(path.join(addonPath, 'models'))) {
              if (!seenPaths.has(addonPath.toLowerCase())) {
                seenPaths.add(addonPath.toLowerCase());
                discovered.push({ name: `addon (${entry.name})`, path: addonPath });
              }
            }
          }
        }
      } catch {
        // ignore
      }
    }

    return discovered;
  }

  /**
   * Resolves a relative asset path across all search roots.
   * @param {string} relativeAssetPath - e.g. 'materials/brick/wall.vmt'
   * @returns {{fullPath: string, rootDir: string, label: string} | null}
   */
  resolve(relativeAssetPath) {
    if (!relativeAssetPath) return null;
    const cleanSubPath = relativeAssetPath.replace(/\\/g, '/').replace(/^\/+/, '');

    for (const root of this.allPaths) {
      const candidate = path.join(root.path, cleanSubPath);
      if (fileExists(candidate)) {
        return {
          fullPath: candidate,
          rootDir: root.path,
          label: root.label
        };
      }
    }

    return null;
  }

  /**
   * Checks if a relative asset exists in any search root.
   * @param {string} relativeAssetPath
   * @returns {boolean}
   */
  exists(relativeAssetPath) {
    return this.resolve(relativeAssetPath) !== null;
  }

  /**
   * Returns list of all active search paths.
   */
  getSearchPaths() {
    return this.allPaths;
  }
}

module.exports = {
  SearchPathManager
};
