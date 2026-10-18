const fs = require('fs');
const path = require('path');
const { parseVMF } = require('./vmfParser');
const { fileExists, normalizeSourcePath } = require('../utils/fileSystem');

class InstanceResolver {
  /**
   * @param {import('../core/searchPaths').SearchPathManager} searchPaths
   * @param {string[]} [soundGetters=[]]
   */
  constructor(searchPaths, soundGetters = []) {
    this.searchPaths = searchPaths;
    this.soundGetters = soundGetters;
    this.visited = new Set(); // Stores lowercase absolute paths of processed instances
    this.resolvedInstances = []; // { sourcePath, relativePath }
  }

  /**
   * Resolves an instance file path on disk.
   * @param {string} instanceRelPath - e.g. 'instances/prefabs/door.vmf'
   * @param {string} parentVmfPath - Absolute path to parent VMF
   * @returns {string|null} - Absolute path if found, or null
   */
  resolveInstancePath(instanceRelPath, parentVmfPath) {
    const cleanRel = normalizeSourcePath(instanceRelPath);
    const parentDir = path.dirname(parentVmfPath);

    // Candidates in order of precedence:
    const candidates = [
      // 1. Direct path relative to current VMF directory
      path.resolve(parentDir, cleanRel),
      // 2. Relative to gamepath/maps
      path.resolve(this.searchPaths.primaryPath, 'maps', cleanRel),
      // 3. Relative to gamepath
      path.resolve(this.searchPaths.primaryPath, cleanRel)
    ];

    // 4. Relative to mounted directories
    for (const root of this.searchPaths.getSearchPaths()) {
      candidates.push(path.resolve(root.path, 'maps', cleanRel));
      candidates.push(path.resolve(root.path, cleanRel));
    }

    for (const cand of candidates) {
      if (fileExists(cand)) {
        return cand;
      }
    }

    return null;
  }

  /**
   * Recursively processes all func_instances starting from a root list.
   * @param {Set<string>|string[]} initialInstances
   * @param {string} rootVmfPath
   * @returns {{
   *   materials: Set<string>,
   *   models: Set<string>,
   *   sounds: Set<string>,
   *   directVtfs: Set<string>,
   *   instanceFiles: Array<{sourcePath: string, relativePath: string}>
   * }}
   */
  processInstances(initialInstances, rootVmfPath) {
    const aggregate = {
      materials: new Set(),
      models: new Set(),
      sounds: new Set(),
      directVtfs: new Set(),
      instanceFiles: []
    };

    const queue = [];
    for (const inst of initialInstances) {
      queue.push({ relPath: inst, parentPath: rootVmfPath });
    }

    while (queue.length > 0) {
      const { relPath, parentPath } = queue.shift();
      const resolvedPath = this.resolveInstancePath(relPath, parentPath);

      if (!resolvedPath) {
        // console.warn(`Warning: Instance VMF not found: ${relPath}`);
        continue;
      }

      const lowerPath = resolvedPath.toLowerCase();
      if (this.visited.has(lowerPath)) {
        continue; // Prevent circular reference loops
      }
      this.visited.add(lowerPath);

      aggregate.instanceFiles.push({
        sourcePath: resolvedPath,
        relativePath: normalizeSourcePath(relPath)
      });

      try {
        const content = fs.readFileSync(resolvedPath, 'utf-8');
        const parsed = parseVMF(content, this.soundGetters);

        // Merge assets
        parsed.materials.forEach(m => aggregate.materials.add(m));
        parsed.models.forEach(m => aggregate.models.add(m));
        parsed.sounds.forEach(s => aggregate.sounds.add(s));
        parsed.directVtfs.forEach(v => aggregate.directVtfs.add(v));

        // Queue child instances
        for (const childInst of parsed.instances) {
          queue.push({ relPath: childInst, parentPath: resolvedPath });
        }
      } catch (err) {
        // console.warn(`Error reading instance ${resolvedPath}:`, err.message);
      }
    }

    return aggregate;
  }
}

module.exports = {
  InstanceResolver
};
