const fs = require('fs');
const path = require('path');
const MDL = require('source-mdl');
const { safeCopy, normalizeSourcePath } = require('../utils/fileSystem');

const MODEL_VARIANTS = [
  '.mdl',
  '.vvd',
  '.phy',
  '.dx90.vtx',
  '.dx80.vtx',
  '.sw.vtx',
  '.ani',
  '.xbox.vtx'
];

class ModelExtractor {
  /**
   * @param {import('../core/searchPaths').SearchPathManager} searchPaths
   * @param {string} outputDir
   * @param {import('./materials').MaterialExtractor} materialExtractor
   */
  constructor(searchPaths, outputDir, materialExtractor) {
    this.searchPaths = searchPaths;
    this.outputDir = outputDir;
    this.materialExtractor = materialExtractor;

    this.processedModels = new Set();
    this.extractedModelsCount = 0;
  }

  /**
   * Extracts a single 3D model and its variant companion files, plus its referenced materials.
   * @param {string} modelPath - e.g. 'models/props_c17/furniturecouch001a.mdl'
   */
  async extractModel(modelPath) {
    if (!modelPath) return;
    const cleanModel = normalizeSourcePath(modelPath);
    if (!cleanModel.toLowerCase().endsWith('.mdl')) return;

    const baseNoExt = cleanModel.slice(0, -4);
    const lowerKey = cleanModel.toLowerCase();

    if (this.processedModels.has(lowerKey)) return;
    this.processedModels.add(lowerKey);

    let foundAny = false;

    // 1. Copy all model variant files (.mdl, .vvd, .phy, .vtx, etc.)
    for (const variant of MODEL_VARIANTS) {
      const relVariant = `${baseNoExt}${variant}`;
      const resolved = this.searchPaths.resolve(relVariant);
      if (resolved) {
        const destPath = path.join(this.outputDir, relVariant);
        const copied = await safeCopy(resolved.fullPath, destPath);
        if (copied && variant === '.mdl') {
          foundAny = true;
        }
      }
    }

    if (foundAny) {
      this.extractedModelsCount++;
    }

    // 2. Read .mdl binary to inspect referenced materials & textures
    const resolvedMdl = this.searchPaths.resolve(cleanModel);
    if (!resolvedMdl) return;

    try {
      const mdlData = fs.readFileSync(resolvedMdl.fullPath);
      const mdl = new MDL();
      mdl.import({ mdlData });

      const metadata = mdl.getMetadata();
      const textures = (metadata.textures || []).map(t => t.toLowerCase());
      const textureDirs = (metadata.textureDirs || []).map(d => d.toLowerCase());

      for (const dir of textureDirs) {
        for (const tex of textures) {
          const matSubPath = normalizeSourcePath(path.join(dir, tex));
          await this.materialExtractor.extractMaterial(matSubPath);
        }
      }
    } catch (err) {
      // Some MDLs may have unsupported versions or non-standard headers
      // console.warn(`MDL parse warning for ${cleanModel}: ${err.message}`);
    }
  }

  /**
   * Extracts all models from a collection.
   * @param {Set<string>|string[]} models
   */
  async extractAll(models) {
    const list = Array.from(models);
    for (const mdl of list) {
      await this.extractModel(mdl);
    }
  }
}

module.exports = {
  ModelExtractor,
  MODEL_VARIANTS
};
