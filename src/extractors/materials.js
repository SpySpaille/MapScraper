const fs = require('fs');
const path = require('path');
const { parseVMT, normalizeTexturePath } = require('../parsers/vmtParser');
const { safeCopy, pLimit, normalizeSourcePath } = require('../utils/fileSystem');

class MaterialExtractor {
  /**
   * @param {import('../core/searchPaths').SearchPathManager} searchPaths
   * @param {string} outputDir
   * @param {string[]} textureGetters
   * @param {string[]} materialGetters
   */
  constructor(searchPaths, outputDir, textureGetters = [], materialGetters = []) {
    this.searchPaths = searchPaths;
    this.outputDir = outputDir;
    this.textureGetters = textureGetters;
    this.materialGetters = materialGetters;

    this.processedMaterials = new Set(); // lowercase material names
    this.processedTextures = new Set(); // lowercase texture names
    this.extractedMaterialsCount = 0;
    this.extractedTexturesCount = 0;
    this.limit = pLimit(20);
  }

  /**
   * Recursively extracts a material VMT and its referenced textures.
   * @param {string} materialName - e.g. 'concrete/concretefloor001a'
   */
  async extractMaterial(materialName) {
    if (!materialName) return;
    const cleanMat = normalizeSourcePath(materialName).replace(/\.vmt$/i, '');
    const lowerMat = cleanMat.toLowerCase();

    if (this.processedMaterials.has(lowerMat)) return;
    this.processedMaterials.add(lowerMat);

    const relVmtPath = `materials/${cleanMat}.vmt`;
    const resolved = this.searchPaths.resolve(relVmtPath);
    if (!resolved) return;

    // Copy the VMT file
    const destVmtPath = path.join(this.outputDir, 'materials', `${cleanMat}.vmt`);
    const copied = await safeCopy(resolved.fullPath, destVmtPath);
    if (copied) {
      this.extractedMaterialsCount++;
    }

    // Read and parse VMT content
    let vmtContent = '';
    try {
      vmtContent = fs.readFileSync(resolved.fullPath, 'utf-8');
    } catch {
      return;
    }

    const vmtMap = parseVMT(vmtContent);

    // 1. Extract textures
    const texturesToExtract = [];
    for (const getter of this.textureGetters) {
      const values = vmtMap.get(getter);
      if (values && values.length > 0) {
        for (const rawVal of values) {
          const normTex = normalizeTexturePath(rawVal);
          if (normTex && !this.processedTextures.has(normTex.toLowerCase())) {
            this.processedTextures.add(normTex.toLowerCase());
            texturesToExtract.push(normTex);
          }
        }
      }
    }

    // Copy extracted textures in parallel with concurrency limiter
    await Promise.all(
      texturesToExtract.map(tex =>
        this.limit(async () => {
          await this.extractTexture(tex);
        })
      )
    );

    // 2. Recursively extract linked materials (include, bottommaterial, etc.)
    const linkedMaterials = [];
    for (const getter of this.materialGetters) {
      const values = vmtMap.get(getter);
      if (values && values.length > 0) {
        for (const rawVal of values) {
          const linked = normalizeTexturePath(rawVal);
          if (linked && !this.processedMaterials.has(linked.toLowerCase())) {
            linkedMaterials.push(linked);
          }
        }
      }
    }

    for (const linked of linkedMaterials) {
      await this.extractMaterial(linked);
    }
  }

  /**
   * Copies a VTF texture (and .hdr.vtf if present).
   * @param {string} textureName
   */
  async extractTexture(textureName) {
    const relVtfPath = `materials/${textureName}.vtf`;
    const resolvedVtf = this.searchPaths.resolve(relVtfPath);
    if (resolvedVtf) {
      const destPath = path.join(this.outputDir, 'materials', `${textureName}.vtf`);
      if (await safeCopy(resolvedVtf.fullPath, destPath)) {
        this.extractedTexturesCount++;
      }
    }

    // Check for HDR texture variant
    const relHdrPath = `materials/${textureName}.hdr.vtf`;
    const resolvedHdr = this.searchPaths.resolve(relHdrPath);
    if (resolvedHdr) {
      const destHdrPath = path.join(this.outputDir, 'materials', `${textureName}.hdr.vtf`);
      await safeCopy(resolvedHdr.fullPath, destHdrPath);
    }
  }

  /**
   * Extracts direct VTF files (e.g. startexture).
   * @param {Set<string>|string[]} directVtfs
   */
  async extractDirectVtfs(directVtfs) {
    for (const vtf of directVtfs) {
      const cleanVtf = normalizeSourcePath(vtf).replace(/\.vtf$/i, '');
      await this.extractTexture(cleanVtf);
    }
  }

  /**
   * Main entry method to extract all materials in a set.
   * @param {Set<string>|string[]} materials
   * @param {Set<string>|string[]} [directVtfs=[]]
   */
  async extractAll(materials, directVtfs = []) {
    const list = Array.from(materials);
    for (const mat of list) {
      await this.extractMaterial(mat);
    }
    if (directVtfs && directVtfs.size > 0) {
      await this.extractDirectVtfs(directVtfs);
    }
  }
}

module.exports = {
  MaterialExtractor
};
