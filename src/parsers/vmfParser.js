const { normalizeSourcePath } = require('../utils/fileSystem');

const MATERIAL_KEYS = [
  'material',
  'texture',
  'detailmaterial',
  'overlayname',
  'bottommaterial'
];

const DIRECT_VTF_KEYS = [
  'startexture'
];

/**
 * Parses a VMF string content and extracts all asset references.
 * @param {string} vmfContent
 * @param {string[]} [soundGetters=[]]
 * @returns {{
 *   materials: Set<string>,
 *   models: Set<string>,
 *   sounds: Set<string>,
 *   directVtfs: Set<string>,
 *   instances: Set<string>,
 *   hasSoundscapes: boolean
 * }}
 */
function parseVMF(vmfContent, soundGetters = []) {
  const materials = new Set();
  const models = new Set();
  const sounds = new Set();
  const directVtfs = new Set();
  const instances = new Set();

  if (!vmfContent) {
    return { materials, models, sounds, directVtfs, instances, hasSoundscapes: false };
  }

  // 1. Skybox
  const skyboxRegex = /"skyname"\s*"([^"]+)"/gi;
  let match;
  while ((match = skyboxRegex.exec(vmfContent)) !== null) {
    const sky = match[1].trim();
    if (sky) {
      materials.add(`skybox/${sky}up`);
      materials.add(`skybox/${sky}dn`);
      materials.add(`skybox/${sky}lf`);
      materials.add(`skybox/${sky}rt`);
      materials.add(`skybox/${sky}ft`);
      materials.add(`skybox/${sky}bk`);
    }
  }

  // 2. Brush & Entity Materials
  for (const key of MATERIAL_KEYS) {
    const regex = new RegExp(`"${key}"\\s*"([^"]+)"`, 'gi');
    while ((match = regex.exec(vmfContent)) !== null) {
      const mat = normalizeSourcePath(match[1]);
      if (mat && !mat.startsWith('tools/') && !mat.startsWith('maps/')) {
        materials.add(mat.replace(/\.vmt$/i, ''));
      }
    }
  }

  // 3. Direct VTF keys (e.g. startexture from env_skypaint)
  for (const key of DIRECT_VTF_KEYS) {
    const regex = new RegExp(`"${key}"\\s*"([^"]+)"`, 'gi');
    while ((match = regex.exec(vmfContent)) !== null) {
      const vtf = normalizeSourcePath(match[1]).replace(/\.vtf$/i, '');
      if (vtf) {
        directVtfs.add(vtf);
      }
    }
  }

  // 4. Models
  const modelRegex = /"model"\s*"([^"]+)"/gi;
  while ((match = modelRegex.exec(vmfContent)) !== null) {
    const mdl = normalizeSourcePath(match[1]);
    if (mdl && mdl.toLowerCase().endsWith('.mdl')) {
      models.add(mdl);
    }
  }

  // 5. Sounds (using SoundGettersCache and default keys)
  const allSoundKeys = Array.from(new Set([
    'message',
    'noise1',
    'noise2',
    'closesound',
    'startclosesound',
    ...soundGetters
  ]));

  for (const key of allSoundKeys) {
    const regex = new RegExp(`"${key}"\\s*"([^"]+)"`, 'gi');
    while ((match = regex.exec(vmfContent)) !== null) {
      let snd = match[1].trim();
      if (snd) {
        // Strip Source sound script prefixes: *, ), ^, #, ~, >, etc.
        snd = snd.replace(/^[*)\^#~><]+/, '').trim();
        if (snd.match(/\.(wav|mp3|ogg)$/i)) {
          sounds.add(normalizeSourcePath(snd));
        }
      }
    }
  }

  // 6. Func_instance extraction
  // Matches entity blocks with "classname" "func_instance" and extracts "file" or "vmf"
  const entityRegex = /entity\s*\{([^}]+)\}/gi;
  while ((match = entityRegex.exec(vmfContent)) !== null) {
    const block = match[1];
    if (/"classname"\s*"func_instance"/i.test(block)) {
      const fileMatch = block.match(/"(?:file|vmf)"\s*"([^"]+)"/i);
      if (fileMatch) {
        let instanceFile = normalizeSourcePath(fileMatch[1]);
        if (!instanceFile.toLowerCase().endsWith('.vmf')) {
          instanceFile += '.vmf';
        }
        instances.add(instanceFile);
      }
    }
  }

  const hasSoundscapes = /soundscape/i.test(vmfContent);

  return {
    materials,
    models,
    sounds,
    directVtfs,
    instances,
    hasSoundscapes
  };
}

module.exports = {
  parseVMF,
  MATERIAL_KEYS,
  DIRECT_VTF_KEYS
};
