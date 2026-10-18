const fs = require('fs');
const path = require('path');
const { safeCopy, normalizeSourcePath } = require('../utils/fileSystem');

/**
 * Strips Source Engine sound flag characters commonly prefixing sound paths:
 *   ) = Spatialized
 *   ^ = Distance-attenuated / streamed
 *   * = Raw / streaming
 *   # = Centered sound
 *   ~ = Pitch modulated
 *   > = Sound script flag
 * @param {string} soundPath
 * @returns {string}
 */
function cleanSoundPath(soundPath) {
  if (!soundPath) return '';
  let clean = normalizeSourcePath(soundPath);
  clean = clean.replace(/^[*)\^#~><]+/, '').trim();
  if (clean.toLowerCase().startsWith('sound/')) {
    clean = clean.slice('sound/'.length);
  }
  return clean;
}

class SoundExtractor {
  /**
   * @param {import('../core/searchPaths').SearchPathManager} searchPaths
   * @param {string} outputDir
   * @param {string} mapName
   */
  constructor(searchPaths, outputDir, mapName) {
    this.searchPaths = searchPaths;
    this.outputDir = outputDir;
    this.mapName = mapName;

    this.processedSounds = new Set();
    this.extractedSoundsCount = 0;
  }

  /**
   * Extracts sounds referenced in soundscapes.
   * @returns {string[]}
   */
  extractSoundscapes() {
    const additionalSounds = [];
    const candidates = [
      `scripts/soundscapes_${this.mapName}.txt`,
      `scripts/soundscapes_${this.mapName}_addon.txt`,
      `scripts/soundscapes_manifest.txt`
    ];

    for (const relScript of candidates) {
      const resolved = this.searchPaths.resolve(relScript);
      if (resolved) {
        // Copy the script file to output
        const destScriptPath = path.join(this.outputDir, relScript);
        safeCopy(resolved.fullPath, destScriptPath);

        try {
          const content = fs.readFileSync(resolved.fullPath, 'utf-8');
          const waveRegex = /"wave"\s+"([^"]+)"/gi;
          let match;
          while ((match = waveRegex.exec(content)) !== null) {
            const rawWave = match[1].trim();
            const clean = cleanSoundPath(rawWave);
            if (clean) {
              additionalSounds.push(clean);
            }
          }
        } catch {
          // ignore read error
        }
      }
    }

    return additionalSounds;
  }

  /**
   * Extracts a single sound file.
   * @param {string} soundPath
   */
  async extractSound(soundPath) {
    if (!soundPath) return;
    const clean = cleanSoundPath(soundPath);
    if (!clean) return;

    const lower = clean.toLowerCase();
    if (this.processedSounds.has(lower)) return;
    this.processedSounds.add(lower);

    const relSoundPath = `sound/${clean}`;
    const resolved = this.searchPaths.resolve(relSoundPath);
    if (resolved) {
      const destPath = path.join(this.outputDir, 'sound', clean.toLowerCase());
      const copied = await safeCopy(resolved.fullPath, destPath);
      if (copied) {
        this.extractedSoundsCount++;
      }
    }
  }

  /**
   * Extracts all sounds.
   * @param {Set<string>|string[]} sounds
   * @param {boolean} [checkSoundscapes=true]
   */
  async extractAll(sounds, checkSoundscapes = true) {
    const allSounds = new Set(sounds);

    if (checkSoundscapes) {
      const soundscapeSounds = this.extractSoundscapes();
      soundscapeSounds.forEach(s => allSounds.add(s));
    }

    for (const snd of allSounds) {
      await this.extractSound(snd);
    }
  }
}

module.exports = {
  SoundExtractor,
  cleanSoundPath
};
