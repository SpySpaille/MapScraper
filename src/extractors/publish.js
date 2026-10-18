const fs = require('fs');
const fse = require('fs-extra');
const path = require('path');
const { spawn } = require('child_process');
const { safeCopy, fileExists, normalizeSourcePath } = require('../utils/fileSystem');

class PublishExtractor {
  /**
   * @param {import('../core/searchPaths').SearchPathManager} searchPaths
   * @param {string} outputDir
   * @param {string} vmfPath
   * @param {import('./materials').MaterialExtractor} materialExtractor
   * @param {Array<{sourcePath: string, relativePath: string}>} [instanceFiles=[]]
   */
  constructor(searchPaths, outputDir, vmfPath, materialExtractor, instanceFiles = []) {
    this.searchPaths = searchPaths;
    this.outputDir = outputDir;
    this.vmfPath = vmfPath;
    this.materialExtractor = materialExtractor;
    this.instanceFiles = instanceFiles;
    this.mapName = path.basename(vmfPath, '.vmf');

    this.additionalFilesCount = 0;
    this.packedFilesCount = 0;
  }

  /**
   * Finds and copies auxiliary map files (.ain, .nav, .png, particles, instances, bsp).
   */
  async process() {
    const bspVariants = [
      `maps/${this.mapName}.ain`,
      `maps/graphs/${this.mapName}.ain`,
      `maps/${this.mapName}.nav`,
      `maps/thumb/${this.mapName}.png`,
      `maps/${this.mapName}_particles.txt`
    ];

    const filesToCopy = [];
    const filesToPack = [];

    const addPackFile = (insidePath, diskFullPath) => {
      const formatted = insidePath.replace(/\\/g, '/');
      if (!filesToPack.includes(formatted)) {
        filesToPack.push(formatted, diskFullPath);
      }
    };

    // 1. Check for standard companion files
    for (const variant of bspVariants) {
      const resolved = this.searchPaths.resolve(variant);
      if (resolved) {
        filesToCopy.push({ relPath: variant, fullPath: resolved.fullPath });
        addPackFile(variant, resolved.fullPath);
      }
    }

    // 2. Check for particles manifest and extract PCF files
    const particlesRel = `maps/${this.mapName}_particles.txt`;
    const resolvedParticles = this.searchPaths.resolve(particlesRel);
    if (resolvedParticles) {
      addPackFile(particlesRel, resolvedParticles.fullPath);
      try {
        const content = fs.readFileSync(resolvedParticles.fullPath, 'utf-8');
        const pcfMatches = content.match(/"file"\s+"([^"]+)"/g);
        if (pcfMatches) {
          for (const line of pcfMatches) {
            const pcfMatch = line.match(/"file"\s+"([^"]+)"/);
            if (pcfMatch) {
              const relPcf = normalizeSourcePath(pcfMatch[1]);
              const resolvedPcf = this.searchPaths.resolve(relPcf);
              if (resolvedPcf) {
                filesToCopy.push({ relPath: relPcf, fullPath: resolvedPcf.fullPath });

                // Inspect PCF file for referenced materials
                try {
                  const pcfContent = fs.readFileSync(resolvedPcf.fullPath, 'latin1');
                  const vmtMatches = pcfContent.match(/[\w\/\\.-]+\.vmt/g);
                  if (vmtMatches) {
                    for (const vmt of vmtMatches) {
                      const cleanVmt = normalizeSourcePath(vmt).replace(/^materials\//i, '');
                      await this.materialExtractor.extractMaterial(cleanVmt);
                    }
                  }
                } catch {
                  // ignore PCF inspect error
                }
              }
            }
          }
        }
      } catch {
        // ignore
      }
    }

    // 3. Check for soundscapes script
    const soundscapesRel = `scripts/soundscapes_${this.mapName}.txt`;
    const resolvedSoundscapes = this.searchPaths.resolve(soundscapesRel);
    if (resolvedSoundscapes) {
      addPackFile(soundscapesRel, resolvedSoundscapes.fullPath);
    }

    // 4. Copy instance VMF files if any were discovered
    for (const inst of this.instanceFiles) {
      // Put instances under maps/instances or relative path
      const destInstPath = path.join(this.outputDir, 'maps', inst.relativePath);
      await safeCopy(inst.sourcePath, destInstPath);
      this.additionalFilesCount++;
    }

    // 5. Copy the main VMF itself
    const destMainVmf = path.join(this.outputDir, 'maps', `${this.mapName}.vmf`);
    await safeCopy(this.vmfPath, destMainVmf);
    this.additionalFilesCount++;

    // 6. Copy all collected companion files
    for (const item of filesToCopy) {
      const destPath = path.join(this.outputDir, item.relPath);
      if (await safeCopy(item.fullPath, destPath)) {
        this.additionalFilesCount++;
      }
    }

    // 7. Check if BSP and bspzip.exe exist to pack files
    const bspRel = `maps/${this.mapName}.bsp`;
    const resolvedBsp = this.searchPaths.resolve(bspRel);
    const bspzipPath = path.join(path.dirname(this.searchPaths.primaryPath), 'bin', 'bspzip.exe');

    let packed = false;
    if (resolvedBsp && fileExists(bspzipPath) && filesToPack.length > 0) {
      const zipfilePath = path.join(this.outputDir, 'zipfiles.txt');
      fs.writeFileSync(zipfilePath, filesToPack.join('\n'), 'utf-8');

      const destBspPath = path.join(this.outputDir, 'maps', `${this.mapName}.bsp`);
      await fse.ensureDir(path.dirname(destBspPath));

      packed = await new Promise((resolve) => {
        const proc = spawn(bspzipPath, [
          '-addorupdatelist',
          resolvedBsp.fullPath,
          zipfilePath,
          destBspPath
        ]);

        proc.on('close', (code) => {
          fse.removeSync(zipfilePath);
          resolve(code === 0);
        });

        proc.on('error', () => {
          fse.removeSync(zipfilePath);
          resolve(false);
        });
      });

      if (packed) {
        this.packedFilesCount = filesToPack.length / 2;
      }
    }

    return {
      additionalFilesCount: this.additionalFilesCount,
      packedFilesCount: this.packedFilesCount,
      packed
    };
  }
}

module.exports = {
  PublishExtractor
};
