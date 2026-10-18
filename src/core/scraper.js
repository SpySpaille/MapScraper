const fs = require('fs');
const fse = require('fs-extra');
const path = require('path');
const { SearchPathManager } = require('./searchPaths');
const { parseVMF } = require('../parsers/vmfParser');
const { InstanceResolver } = require('../parsers/instanceResolver');
const { MaterialExtractor } = require('../extractors/materials');
const { ModelExtractor } = require('../extractors/models');
const { SoundExtractor } = require('../extractors/sounds');
const { PublishExtractor } = require('../extractors/publish');
const { loadAllGetters } = require('../utils/getters');
const { showLoadingIndicator, stopLoadingIndicator } = require('../cli/ui');

/**
 * Executes the complete MapScraper pipeline.
 * @param {object} options
 * @param {string} options.gamePath - Primary game directory path
 * @param {string} options.vmfPath - VMF map file path
 * @param {boolean} options.extractMaterials - Extract materials & textures
 * @param {boolean} options.extractModels - Extract 3D models & model materials
 * @param {boolean} options.extractSounds - Extract sounds & soundscapes
 * @param {boolean} options.readyToPublish - Extract auxiliary files and pack bsp
 * @param {string[]} [options.customMounts=[]] - Optional custom mount directories
 * @param {string} [options.outputDir] - Optional custom output directory
 */
async function runScraper(options) {
  const {
    gamePath,
    vmfPath,
    extractMaterials = true,
    extractModels = true,
    extractSounds = true,
    readyToPublish = false,
    customMounts = []
  } = options;

  const mapName = path.basename(vmfPath, '.vmf');
  const outputDir = path.resolve(options.outputDir || `./${mapName}`);

  // 1. Initialize Search Paths & Mounts
  const searchPaths = new SearchPathManager(gamePath, customMounts);
  const activeMounts = searchPaths.getSearchPaths().filter(p => p.label !== 'Primary Game');

  console.log(`\n📂 Game Path: \x1b[36m${searchPaths.primaryPath}\x1b[0m`);
  if (activeMounts.length > 0) {
    console.log(`🔗 Mounted directories detected (${activeMounts.length}):`);
    for (const m of activeMounts) {
      console.log(`   \x1b[90m- \x1b[32m${m.label}: \x1b[37m${m.path}\x1b[0m`);
    }
  }

  // 2. Prepare output directory
  fse.ensureDirSync(outputDir);

  // 3. Load Getters (offline-first with online cache)
  let spinner = showLoadingIndicator('Loading asset getters...');
  const { textureGetters, materialGetters, soundGetters } = await loadAllGetters();
  stopLoadingIndicator(spinner);

  // 4. Parse Root VMF
  spinner = showLoadingIndicator('Parsing VMF map file...');
  const vmfContent = fs.readFileSync(vmfPath, 'utf-8');
  const rootParsed = parseVMF(vmfContent, soundGetters);
  stopLoadingIndicator(spinner);

  // 5. Handle func_instances
  let instanceFiles = [];
  if (rootParsed.instances.size > 0) {
    spinner = showLoadingIndicator(`Resolving ${rootParsed.instances.size} func_instance(s)...`);
    const instanceResolver = new InstanceResolver(searchPaths, soundGetters);
    const instanceData = instanceResolver.processInstances(rootParsed.instances, vmfPath);
    stopLoadingIndicator(spinner);

    // Merge instance assets into root collections
    instanceData.materials.forEach(m => rootParsed.materials.add(m));
    instanceData.models.forEach(m => rootParsed.models.add(m));
    instanceData.sounds.forEach(s => rootParsed.sounds.add(s));
    instanceData.directVtfs.forEach(v => rootParsed.directVtfs.add(v));
    instanceFiles = instanceData.instanceFiles;

    console.log(`🔄 • ✅ \x1b[32m${instanceFiles.length}\x1b[0m \x1b[37mfunc_instance VMFs processed\x1b[0m`);
  }

  // 6. Material Extractor instance (shared with models and publish)
  const materialExtractor = new MaterialExtractor(
    searchPaths,
    outputDir,
    textureGetters,
    materialGetters
  );

  // 7. Extract Sounds
  if (extractSounds) {
    spinner = showLoadingIndicator('🔊 Extracting sounds...');
    const soundExtractor = new SoundExtractor(searchPaths, outputDir, mapName);
    await soundExtractor.extractAll(rootParsed.sounds, rootParsed.hasSoundscapes);
    stopLoadingIndicator(spinner);
    console.log(`🔊 • ✅ \x1b[32m${soundExtractor.extractedSoundsCount}\x1b[0m \x1b[37msounds extracted\x1b[0m`);
  }

  // 8. Extract Materials
  if (extractMaterials) {
    spinner = showLoadingIndicator('🧱 Extracting materials & textures...');
    await materialExtractor.extractAll(rootParsed.materials, rootParsed.directVtfs);
    stopLoadingIndicator(spinner);
    console.log(`🧱 • ✅ \x1b[32m${materialExtractor.extractedMaterialsCount}\x1b[0m \x1b[37mmaterials & \x1b[32m${materialExtractor.extractedTexturesCount}\x1b[0m \x1b[37mtextures extracted\x1b[0m`);
  }

  // 9. Extract Models
  if (extractModels) {
    spinner = showLoadingIndicator('📐 Extracting 3D models...');
    const modelExtractor = new ModelExtractor(searchPaths, outputDir, materialExtractor);
    await modelExtractor.extractAll(rootParsed.models);
    stopLoadingIndicator(spinner);
    console.log(`📐 • ✅ \x1b[32m${modelExtractor.extractedModelsCount}\x1b[0m \x1b[37mmodels extracted\x1b[0m`);
  }

  // 10. Ready to Publish
  if (readyToPublish) {
    spinner = showLoadingIndicator('📤 Processing publish files & BSP packing...');
    const publishExtractor = new PublishExtractor(
      searchPaths,
      outputDir,
      vmfPath,
      materialExtractor,
      instanceFiles
    );
    const pubResult = await publishExtractor.process();
    stopLoadingIndicator(spinner);
    console.log(`📤 • ✅ \x1b[32m${pubResult.additionalFilesCount}\x1b[0m \x1b[37madditional files extracted\x1b[0m`);
    if (pubResult.packed) {
      console.log(`📦 • ✅ \x1b[32m${pubResult.packedFilesCount}\x1b[0m \x1b[37mfiles packed into BSP\x1b[0m`);
    }
  }

  console.log('\n\x1b[1m\x1b[32mExtraction completed 🎉\x1b[0m');
  console.log(`📁 Output directory: \x1b[36m${outputDir}\x1b[0m`);
  console.log('\x1b[90mThanks for using \x1b[37mMapScraper 👍\x1b[0m');

  return {
    outputDir,
    instancesCount: instanceFiles.length,
    materialsCount: materialExtractor.extractedMaterialsCount,
    texturesCount: materialExtractor.extractedTexturesCount
  };
}

module.exports = {
  runScraper
};
