const prompt = require('prompt');
const fs = require('fs');
const path = require('path');
const { yn } = require('./ui');
const { removeQuotes, fileExists } = require('../utils/fileSystem');
const { loadConfig, saveConfig, clearConfig } = require('../core/config');

prompt.message = '\x1b[90m-';
prompt.delimiter = ' ';

function getYesNoValidator(description) {
  return {
    description: `${description} ${yn}`,
    required: true,
    conform: val => val === 'y' || val === 'n',
    message: "Invalid input (type 'y' for yes, 'n' for no)"
  };
}

/**
 * Prompts user for all required parameters.
 * @returns {Promise<{
 *   gamePath: string,
 *   vmfPath: string,
 *   extractMaterials: boolean,
 *   extractModels: boolean,
 *   extractSounds: boolean,
 *   readyToPublish: boolean
 * }>}
 */
async function promptUser() {
  prompt.start();

  const config = loadConfig();
  let gamePath = config.gamepath;

  // If a saved game path exists, ask whether to reuse it
  if (gamePath && fileExists(path.join(gamePath, 'materials'))) {
    const reuse = await new Promise((resolve, reject) => {
      prompt.get({
        properties: {
          reuse: {
            description: `💾 Use last Game Path ? (${gamePath}) ${yn}`,
            required: true,
            conform: val => val === 'y' || val === 'n',
            message: "Invalid input (type 'y' or 'n')"
          }
        }
      }, (err, res) => {
        if (err) return reject(err);
        resolve(res.reuse === 'y');
      });
    });

    if (!reuse) {
      clearConfig();
      gamePath = null;
    }
  } else {
    gamePath = null;
  }

  // Ask for game path if not set
  if (!gamePath) {
    const res = await new Promise((resolve, reject) => {
      prompt.get({
        properties: {
          gamePath: {
            description: '📂 Game Path (e.g. C:\\Steam\\steamapps\\common\\GarrysMod\\garrysmod) >',
            required: true,
            conform: val => {
              const clean = removeQuotes(val);
              return fileExists(path.join(clean, 'materials')) || fileExists(path.join(clean, 'gameinfo.txt'));
            },
            message: 'Invalid game path: directory does not contain materials/ or gameinfo.txt',
            before: val => removeQuotes(val)
          }
        }
      }, (err, r) => {
        if (err) return reject(err);
        resolve(r);
      });
    });

    gamePath = res.gamePath;
    saveConfig({ ...config, gamepath: gamePath });
  }

  // Ask for VMF file and extraction options
  const answers = await new Promise((resolve, reject) => {
    prompt.get({
      properties: {
        vmfPath: {
          description: '📄 VMF File (C:\\path\\to\\map.vmf) >',
          required: true,
          conform: val => {
            const clean = removeQuotes(val);
            return clean.toLowerCase().endsWith('.vmf') && fileExists(clean);
          },
          message: 'Invalid file: must be an existing .vmf file',
          before: val => removeQuotes(val)
        },
        materials: getYesNoValidator('🧱 Extract Materials'),
        models: getYesNoValidator('📐 Extract Models'),
        sounds: getYesNoValidator('🔊 Extract Sounds'),
        publish: getYesNoValidator('📤 Ready to Publish (all other contents & pack)')
      }
    }, (err, res) => {
      if (err) return reject(err);
      resolve(res);
    });
  });

  return {
    gamePath,
    vmfPath: answers.vmfPath,
    extractMaterials: answers.materials === 'y',
    extractModels: answers.models === 'y',
    extractSounds: answers.sounds === 'y',
    readyToPublish: answers.publish === 'y'
  };
}

module.exports = {
  promptUser
};
