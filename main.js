const express = require('express');
const app = express();
const fs = require('fs');
const fse = require('fs-extra');
const MDL = require('source-mdl');
const path = require('path');
const prompt = require('prompt');
const fetch = require('node-fetch');
const { version } = require('./package.json');
const { spawn } = require('child_process');

const githubUser = 'SpySpaille'; // Change this to your github username
const latestURL = `https://raw.githubusercontent.com/${githubUser}/MapScraper/main/package.json`;
const TextureGettersURL = `https://raw.githubusercontent.com/${githubUser}/MapScraper/main/Getters/TextureGetter.txt`;
const MaterialGettersURL = `https://raw.githubusercontent.com/${githubUser}/MapScraper/main/Getters/MaterialGetter.txt`;
const SoundGettersURL = `https://raw.githubusercontent.com/${githubUser}/MapScraper/main/Getters/SoundGetter.txt`;

let TextureGettersCache = [];
let MaterialGettersCache = [];
let SoundGettersCache = [];
let AllTexturesCache = [];

const colors = require('@colors/colors');
const figlet = require('figlet');

app.listen(3000, async () => {
      try {
            await setup();
      } catch (error) {
            handleError(error);
      }
});

function waitForExit() {
      console.log('Press enter to close the program');
      process.stdin.resume();
      process.stdin.on('data', function () {
            process.exit();
      });
}

function removePathQuotes(value) {
      return value.replace(/^"|"$/g, '');
}

// Put [y/n] with white [, y in green, n in red and / in grey
const yn = '\x1b[37m[\x1b[32my\x1b[37m/\x1b[31mn\x1b[37m]\x1b[90m >';

async function setup(skip, gamepath) {
      const response = await fetch(latestURL);
      const json = await response.json();

      prompt.message = '\x1b[90m-';
      prompt.delimiter = ' ';
      prompt.start();

      figlet('MapScraper', function (err, data) {
            if (!skip) {
                  if (err) throw err;
                  console.log(colors.green(data));
                  console.log(`\x1b[90mMade by \x1b[37mSpySpaille \x1b[90mwith ❤️`);

                  if (json.version === version) {
                        console.log(`\x1b[90m-\x1b[32m Version: \x1b[37m${version}\n`);
                  } else {
                        console.log(`\x1b[90m-\x1b[32m Version: \x1b[37m${version}\x1b[31m (Outdated)\n\x1b[90m-\x1b[32m Latest Version: \x1b[37m${json.version} \x1b[90m(https://github.com/SpySpaille/MapScraper)\n`);
                  }
                  console.log(`📜 Please enter the following information`);
            }

            const commonProperties = {
                  file: {
                        description: '📄 VMF File (C:\\path\\to\\file.vmf) >',
                        required: true,
                        conform: value => removePathQuotes(value).endsWith('.vmf') && fs.existsSync(removePathQuotes(value)),
                        message: 'Invalid file',
                        before: value => removePathQuotes(value) // remove double quotes
                  },
                  sounds: getInputValidator('🔊 Extract Sounds'),
                  materials: getInputValidator('🧱 Extract Materials'),
                  models: getInputValidator('📐 Extract Models'),
                  publish: getInputValidator('📤 Ready to Publish (all other contents)')
            };

            if (fs.existsSync(path.join(process.env.APPDATA, 'MapScraper', 'config.json')) && !gamepath) {

                  const lastGamePath = JSON.parse(fs.readFileSync(path.join(process.env.APPDATA, 'MapScraper', 'config.json'))).gamepath;
                  const configProperties = {
                        gamepath: {
                              description: `💾 Use last Game Path ? (${lastGamePath}) ${yn}`,
                              required: true,
                              conform: value => value === 'y' || value === 'n',
                              message: `Invalid argument (use 'y' for yes, 'n' for no)`
                        }
                  };
                  prompt.get({ properties: { ...configProperties } }, async function (err, result) {
                        if (err) throw err;
                        await script(result.gamepath);
                  });

            } else if (!gamepath) {

                  const inputProperties = {
                        gamepath: {
                              description: '📂 Game Path (C:\\Steam\\steamapps\\common\\GarrysMod\\garrysmod) >',
                              required: true,
                              conform: value => fs.existsSync(path.join(removePathQuotes(value), 'materials')),
                              message: 'Invalid path',
                              before: value => removePathQuotes(value) // remove double quotes
                        },
                        ...commonProperties
                  };
                  prompt.get({ properties: { ...inputProperties } }, async function (err, result) {
                        if (err) throw err;
                        await script(result.gamepath, result.file, result.materials === 'y', result.models === 'y', result.sounds === 'y', result.publish === 'y');
                  });

            } else if (gamepath) {

                  const inputProperties = { ...commonProperties };
                  prompt.get({ properties: { ...inputProperties } }, async function (err, result) {
                        if (err) throw err;
                        await script(gamepath, result.file, result.materials === 'y', result.models === 'y', result.sounds === 'y', result.publish === 'y');
                  });

            }
      });
}

function getInputValidator(description) {
      return {
            description: `${description} ${yn}`,
            required: true,
            conform: value => value === 'y' || value === 'n',
            message: 'Invalid input'
      };
}


// All models variants
const modelsvariants = ['.mdl', '.vvd', '.phy', '.dx90.vtx', '.dx80.vtx', '.sw.vtx', '.ani'];

async function script(gamepath, file, matbool, mdlbool, soundbool, publishbool) {
      // Get textures getters
      const response = await fetch(TextureGettersURL);
      const text = await response.text();
      TextureGettersCache = text.split('\n').map(line => line.trim());

      // Get materials getters
      const response1 = await fetch(MaterialGettersURL);
      const text1 = await response1.text();
      MaterialGettersCache = text1.split('\n').map(line => line.trim());

      // Get sounds getters
      const response2 = await fetch(SoundGettersURL);
      const text2 = await response2.text();
      SoundGettersCache = text2.split('\n').map(line => line.trim());

      // Config System
      if (gamepath === 'y') {
            gamepath = JSON.parse(fs.readFileSync(path.join(process.env.APPDATA, 'MapScraper', 'config.json'))).gamepath;
            await setup(true, gamepath);
            return;
      } else if (gamepath === 'n') {
            fse.removeSync(path.join(process.env.APPDATA, 'MapScraper', 'config.json'));
            await setup(true);
            return;
      }

      const config = {
            gamepath: gamepath
      };
      fse.ensureDirSync(path.join(process.env.APPDATA, 'MapScraper'));
      fs.writeFileSync(path.join(process.env.APPDATA, 'MapScraper', 'config.json'), JSON.stringify(config, null, 2));

      try {
            const vmfContent = fs.readFileSync(file, 'utf-8');
            const outputDir = `./${path.basename(file).replace('.vmf', '')}`;

            fse.removeSync(outputDir);
            fse.ensureDirSync(outputDir);

            if (soundbool) {
                  await getSounds(gamepath, vmfContent, outputDir, file);
            }

            if (matbool) {
                  await getMaterials(gamepath, vmfContent, outputDir);
            }

            if (mdlbool) {
                  await getModels(gamepath, vmfContent, outputDir);
            }
            if (publishbool) {
                  await getOthers(gamepath, vmfContent, outputDir, file);
            }

            console.log('\n\x1b[1m\x1b[32mExtraction completed 🎉\nThanks for using \x1b[37mMapScraper 👍\x1b[0m');
            await waitForExit();
      } catch (error) {
            handleError(error);
      }
}

// Show the loading indicator
const loadingChars = ['|', '/', '-', '\\'];
function showLoadingIndicator(message) {
      let loadingIndex = 0;
      const loadingInterval = setInterval(() => {
            process.stdout.write(`\r\x1b[90m- \x1b[90m${message} \x1b[37m${loadingChars[loadingIndex]}\x1b[0m`);
            loadingIndex = (loadingIndex + 1) % loadingChars.length;
      }, 250);

      return loadingInterval;
}

function parseVMT(vmtContent) {
      const lines = vmtContent.replace(/([^\n])\s*(\$\w+|\%\w+)/g, '$1\n$2').split('\n'); // Force \n after $/%key value
      const map = new Map();

      lines.forEach(line => {
            const cleanLine = line.split('//')[0].trim();

            if (cleanLine) {
                  let [key, ...valueParts] = cleanLine.split(/\s+/);
                  key = key.trim().replace(/^"|"$/g, '').toLowerCase();

                  if (key.startsWith('$') || key.startsWith('%')) {
                        key = key.slice(1);

                        const value = valueParts.join(' ').trim().replace(/^"|"$/g, '');
                        if (key && value) {
                              if (map.has(key)) {
                                    map.get(key).push(value);
                              } else {
                                    map.set(key, [value]);
                              }
                        }
                  }
            }
      });

      return map;
}

// Get VTFs from materials
async function VTFfromVMT(gamepath, sourcePath, outputDir) {
      try {
            let textures = [];
            const materialContent = fs.readFileSync(sourcePath, 'utf-8').trim();

            const vmtMap = parseVMT(materialContent);
            if (!vmtMap) return;

            TextureGettersCache.forEach(getter => {

                  const values = vmtMap.get(getter);
                  if (values && values.length > 0) {
                        values.forEach(value => {
                              const updatedPath = value.lastIndexOf('\\') !== -1 ? value.slice(0, value.lastIndexOf('\\')) + '/' + value.slice(value.lastIndexOf('\\') + 1) : value;
                              textures.push(updatedPath.replace('.vmt', '').replace('.vtf', ''));
                        });
                  }

            });

            // Materials are linked vmt files that we need to extract too
            MaterialGettersCache.forEach(getter => {

                  const values = vmtMap.get(getter);
                  if (values && values.length > 0) {

                        values.forEach(async (value) => {
                              const vmtPath = path.join(gamepath, 'materials', value.replace('.vmt', '') + '.vmt');
                              if (fs.existsSync(vmtPath)) {
                                    fse.copy(vmtPath, path.join(outputDir, 'materials', value.replace('.vmt', '') + '.vmt'));
                                    await VTFfromVMT(gamepath, vmtPath, outputDir);
                              }
                        });

                  }

            });

            for (const texture of textures) {
                  if (!AllTexturesCache.includes(texture)) {
                        AllTexturesCache.push(texture);

                        const sourcePath = path.join(gamepath, 'materials', texture + '.vtf');
                        if (fs.existsSync(sourcePath)) {
                              const destPath = path.join(outputDir, 'materials', texture + '.vtf');
                              await fse.copy(sourcePath, destPath);
                        }

                        const hdrSourcePath = path.join(gamepath, 'materials', texture + '.hdr.vtf');
                        if (fs.existsSync(hdrSourcePath)) {
                              const hdrDestPath = path.join(outputDir, 'materials', texture + '.hdr.vtf');
                              await fse.copy(hdrSourcePath, hdrDestPath);
                        }
                  }
            }

            return textures;
      } catch (error) {
            handleError(error);
      }
}

async function getMaterials(gamepath, vmfContent, outputDir) {
      const loadingInterval = showLoadingIndicator('🧱 Extracting Materials');
      const matgeters = ['material', 'texture', 'detailmaterial'];
      let materials = [];

      // Get the skybox
      const regex = /"skyname"\s*"([^"]+)"/g;
      let match;
      while (match = regex.exec(vmfContent)) {
            const skybox = match[1];
            if (!materials.includes(skybox)) {
                  materials.push(`skybox/${skybox}up`);
                  materials.push(`skybox/${skybox}dn`);
                  materials.push(`skybox/${skybox}lf`);
                  materials.push(`skybox/${skybox}rt`);
                  materials.push(`skybox/${skybox}ft`);
                  materials.push(`skybox/${skybox}bk`);
            }
      }

      matgeters.forEach(getter => {
            const regex = new RegExp(`\\"${getter}\\"\\s*\\"([^"]+)\\"`, 'g');
            let match;

            while ((match = regex.exec(vmfContent))) {
                  const material = match[1];

                  if (!materials.includes(material)) {
                        materials.push(material);
                  }
            }
      });

      for (const material of materials) {
            const sourcePath = path.join(gamepath, 'materials', material + '.vmt');
            const destPath = path.join(outputDir, 'materials', material.toLowerCase() + '.vmt');

            try {
                  fse.copySync(sourcePath, destPath);
                  await VTFfromVMT(gamepath, sourcePath, outputDir);
            } catch (error) {
                  handleError(error);
            }
      }
      clearInterval(loadingInterval);
      process.stdout.write(`\r\x1b[90m${' '.repeat(50)}\r`);
      console.log(`\n🧱 • ✅ \x1b[32m${materials.length} \x1b[37mmaterials have been extracted \x1b[0m`);
}

async function getModels(gamepath, vmfContent, outputDir) {
      const loadingInterval = showLoadingIndicator('📐 Extracting Models');
      const regex = /"model"\s*"([^"]+)"/g;
      let models = [];

      let match;
      while ((match = regex.exec(vmfContent))) {
            const model = match[1];

            if (!models.includes(model) && model.endsWith('.mdl')) {
                  models.push(model);
            }
      }

      for (const model of models) {
            for (const variant of modelsvariants) {
                  const sourcePath = path.join(gamepath, model.replace('.mdl', variant));
                  const destPath = path.join(outputDir, model.replace('.mdl', variant));

                  try {
                        fse.copySync(sourcePath, destPath);
                  } catch (error) {
                        handleError(error);
                  }
            }

            try {
                  const mdlData = fs.readFileSync(path.join(gamepath, model));
                  const mdl = new MDL();
                  mdl.import({ mdlData });

                  const textures = mdl.getMetadata().textures.map(texture => texture.toLowerCase());
                  const textureDirs = mdl.getMetadata().textureDirs.map(textureDir => textureDir.toLowerCase());

                  for (const textureDir of textureDirs) {
                        for (const texture of textures) {
                              const sourcePath = path.join(gamepath, 'materials', textureDir, texture + '.vmt');
                              const destPath = path.join(outputDir, 'materials', textureDir, texture + '.vmt');

                              try {
                                    fse.copySync(sourcePath, destPath);
                                    await VTFfromVMT(gamepath, sourcePath, outputDir);
                              } catch (error) {
                                    handleError(error);
                              }
                        }
                  }
            } catch (error) {
                  handleError(error);
            }
      }
      clearInterval(loadingInterval);
      process.stdout.write(`\r\x1b[90m${' '.repeat(50)}\r`); // Remove the line
      console.log(`\n📐 • ✅ \x1b[32m${models.length} \x1b[37mmodels have been extracted \x1b[0m`);
}

async function getSounds(gamepath, vmfContent, outputDir, file) {
      const loadingInterval = showLoadingIndicator('Extracting sounds');

      let sounds = [];
      if (vmfContent.includes('soundscape')) {
            const filename = `soundscapes_${file.split('\\').pop().split('/').pop().replace('.vmf', '.txt')}`;
            const soundscapesPath = path.join(gamepath, 'scripts', filename);

            if (fs.existsSync(soundscapesPath)) {
                  try {

                        fse.copySync(soundscapesPath, path.join(outputDir, 'scripts', filename));
                        const soundscapesContent = fs.readFileSync(soundscapesPath, 'utf-8');
                        const soundLines = soundscapesContent.match(/"wave"\s+"([^"]+)"/g);
                        if (soundLines) {
                              for (const line of soundLines) {
                                    const soundFile = line.match(/"wave"\s+"([^"]+)"/)[1];
                                    sounds.push(soundFile);
                              }
                        }

                  } catch (error) {
                        handleError(error);
                  }
            }
      }

      for (const getter of SoundGettersCache) {
            const regex = new RegExp(`\\"${getter}\\"\\s*\\"([^"]+)\\"`, 'g');
            let match;

            while ((match = regex.exec(vmfContent))) {
                  const sound = match[1];

                  if (!sounds.includes(sound)) {
                        sounds.push(sound);
                  }
            }
      }

      for (const sound of sounds) {
            const sourcePath = path.join(gamepath, 'sound', sound);
            const destPath = path.join(outputDir, 'sound', sound.toLowerCase());

            try {
                  fse.copySync(sourcePath, destPath);
            } catch (error) {
                  handleError(error);
            }
      }

      clearInterval(loadingInterval);
      process.stdout.write(`\r\x1b[90m${' '.repeat(50)}\r`); // Remove the line
      console.log(`\n🔊 • ✅ \x1b[32m${sounds.length} \x1b[37msounds have been extracted \x1b[0m`);
}

async function getOthers(gamepath, vmfContent, outputDir, file) {
      const loadingInterval = showLoadingIndicator('Searching for other files');
      const mapname = path.basename(file, '.vmf');
      const bspvariants = [
            '%mapname%.ain',
            '/graphs/%mapname%.ain',
            '%mapname%.nav',
            '/thumb/%mapname%.png',
            '%mapname%_particles.txt'
      ];

      const files = [];
      const filetopack = [];

      const addpackfile = (insidepath) => {
            const formattedPath = insidepath.replace(/\\/g, '/');
            if (!filetopack.includes(formattedPath)) {
                  filetopack.push(formattedPath, path.join(gamepath, insidepath));
            }
      };

      // Check for file existence and push to files array
      await Promise.all(bspvariants.map(async (variant) => {
            const sourcePath = path.join(gamepath, 'maps', variant.replace('%mapname%', mapname));
            if (fs.existsSync(sourcePath)) files.push(sourcePath.slice(gamepath.length + 1));
      }));

      const particlesPath = path.join(gamepath, `maps/${mapname}_particles.txt`);
      if (fs.existsSync(particlesPath)) {
            addpackfile(`maps/${mapname}_particles.txt`);
            fs.readFileSync(particlesPath, 'utf-8').match(/"file"\s+"([^"]+)"/g)?.forEach(line => {
                  files.push(line.match(/"file"\s+"([^"]+)"/)[1]);
            });
      }

      // Check for soundscapes file
      const soundscapesPath = path.join(gamepath, 'scripts', `soundscapes_${mapname}.txt`);
      if (fs.existsSync(soundscapesPath)) {
            addpackfile(`scripts/soundscapes_${mapname}.txt`);
      }

      // Pack the files into the bsp
      fs.writeFileSync(path.join(outputDir, 'zipfiles.txt'), filetopack.join('\n'));
      let packed = false;
      const bspzipPath = path.join(gamepath.slice(0, gamepath.lastIndexOf('\\')), 'bin', 'bspzip.exe');
      if (fs.existsSync(bspzipPath)) {
            const bspzip = spawn(bspzipPath, [
                  '-addorupdatelist',
                  path.join(gamepath, 'maps', `${mapname}.bsp`),
                  path.join(outputDir, 'zipfiles.txt'),
                  path.join(outputDir, 'maps', `${mapname}.bsp`)
            ]);

            bspzip.on('close', (code) => {
                  if (code === 0) {
                        packed = false;
                  } else {
                        console.error(`\n❌ \x1b[31mAn error occurred while packing files\x1b[0m`);
                  }
                  fse.removeSync(path.join(outputDir, 'zipfiles.txt'));
            });
      } else {
            console.error(`\n❌ \x1b[31mbspzip.exe not found, files have not been packed\x1b[0m`);
      }

      // Copy files to output directory
      await Promise.all(files.map(async (file) => {
            const sourcePath = path.join(gamepath, file);
            const destPath = path.join(outputDir, file);
            try {
                  await fse.copy(sourcePath, destPath);
            } catch (error) {
                  handleError(error);
            }
      }));

      clearInterval(loadingInterval);
      process.stdout.write(`\r\x1b[90m${' '.repeat(50)}\r`); // Remove the loading line
      console.log(`\n📤 • \x1b[32mReady to Publish process:`);
      console.log(`- ✅ \x1b[32m${files.length} \x1b[37madditional files have been extracted \x1b[0m`);
      if (packed) console.log(`- 📦 \x1b[32m${filetopack.length / 2} \x1b[37madditional files have been packed \x1b[0m`);
}

function handleError(error) {
      if (error.code !== 'ENOENT') {
            console.error('\n❌ \x1b[31mAn error occurred, please report it.\x1b[0m');
            console.error(error);
            waitForExit();
      }
}

process.on('uncaughtException', function (err) {
      handleError(err);
});