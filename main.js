const express = require('express');
const app = express();
const fs = require('fs');
const fse = require('fs-extra');
const MDL = require('source-mdl');
const path = require('path');
const prompt = require('prompt');
const fetch = require('node-fetch'); // Added missing require
const { version } = require('./package.json');
const latestURL = "https://raw.githubusercontent.com/SpySpaille/MapScraper/main/package.json";
const TextureGettersURL = "https://raw.githubusercontent.com/SpySpaille/MapScraper/main/Getters/TextureGetter.txt";
const SoundGettersURL = "https://raw.githubusercontent.com/SpySpaille/MapScraper/main/Getters/SoundGetter.txt";

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

async function setup() {
    const response = await fetch(latestURL);
    const json = await response.json();

    prompt.message = '\x1b[90m-';
    prompt.delimiter = ' ';
    prompt.start();

    figlet('MapScrapper', function (err, data) {
        if (err) throw err;
        console.log(colors.green(data));
        console.log(`\x1b[90mMade by \x1b[37mSpySpaille \x1b[90mwith ❤️`);

        if (json.version === version) {
            console.log(`\x1b[90m-\x1b[32m Version: \x1b[37m${version}\n`);
        } else {
            console.log(`\x1b[90m-\x1b[32m Version: \x1b[37m${version}\x1b[31m (Outdated)\n\x1b[90m-\x1b[32m Latest Version: \x1b[37m${json.version} \x1b[90m(https://github.com/SpySpaille/MapScraper)\n`);
        }

        console.log(`📜 Please enter the following information`);
        prompt.get({
            properties: {
                gamepath: {
                    description: '📂 \x1b[37mGame Path\x1b[90m (C:\\Steam\\steamapps\\common\\GarrysMod\\garrysmod) >',
                    required: true,
                    conform: value => fs.existsSync(path.join(value, 'materials')),
                    message: 'Invalid path'
                },
                file: {
                    description: '🗒️ \x1b[37mVMF File\x1b[90m >',
                    required: true,
                    conform: value => value.endsWith('.vmf') && fs.existsSync(value),
                    message: 'Invalid file'
                },
                materials: getInputValidator('🧱 Extract Materials'),
                models: getInputValidator('📐 Extract Models'),
                sounds: getInputValidator('🔊 Extract Sounds')
            }
        }, async function (err, result) {
            if (err) throw err;
            await script(result.gamepath, result.file, result.materials === 'y', result.models === 'y', result.sounds === 'y');
        });
    });
}

function getInputValidator(description) {
    return {
        description: `${description}\x1b[90m (y/n) >`,
        required: true,
        conform: value => value === 'y' || value === 'n',
        message: 'Invalid input'
    };
}


// Liste des variants de modèle
const modelsvariants = [".mdl", ".vvd", ".phy", ".dx90.vtx", ".dx80.vtx", ".sw.vtx", ".ani"];

async function script(gamepath, file, matbool, mdlbool, soundbool) {
    const outputDir = './output';

    try {
        const vmfContent = fs.readFileSync(file, 'utf-8');

        fse.removeSync(outputDir);
        fse.ensureDirSync(outputDir);

        if (matbool) {
            await getMaterials(gamepath, vmfContent, outputDir);
        }

        if (mdlbool) {
            await getModels(gamepath, vmfContent, outputDir);
        }

        if (soundbool) {
            await getSounds(gamepath, vmfContent, outputDir);
        }

        console.log('\n✅ \x1b[32mAll done, goodbye!\x1b[0m');
        waitForExit();
    } catch (error) {
        handleError(error);
    }
}

// Fonction pour afficher l'indicateur de chargement
const loadingChars = ['|', '/', '-', '\\'];
function showLoadingIndicator(message) {
    let loadingIndex = 0;
    const loadingInterval = setInterval(() => {
        process.stdout.write(`\r\x1b[90m- \x1b[90m${message} \x1b[37m${loadingChars[loadingIndex]}\x1b[0m`);
        loadingIndex = (loadingIndex + 1) % loadingChars.length;
    }, 250);

    return loadingInterval;
}

// Get VTFs from materials
async function VTFfromVMT(gamepath, sourcePath, outputDir) {
    try {
        const response = await fetch(TextureGettersURL);
        const text = await response.text();
        const TextureGetters = text.split('\n').map(line => line.trim());

        let textures = [];
        let materialContent;

        try {
            materialContent = fs.readFileSync(sourcePath, 'utf-8').replace(/"+/g, '').replace(/\s+/g, ' ').trim().toLowerCase();
        } catch (err) {
            return;
        }

        TextureGetters.forEach(getter => {
            if (materialContent.includes(getter)) {
                const regex = new RegExp(`\\${getter} \\s*([^\\s]+)`, 'g');
                let match;

                while ((match = regex.exec(materialContent))) {
                    const texture = match[1];
                    if (!textures.includes(texture)) {
                        textures.push(texture);
                    }
                }
            }
        });

        for (const texture of textures) {
            const sourcePath = path.join(gamepath, 'materials', texture + '.vtf');
            const destPath = path.join(outputDir, 'materials', texture.toLowerCase() + '.vtf');

            try {
                await fse.copy(sourcePath, destPath);
            } catch (err) {
                handleError(err);
            }
        }
    } catch (error) {
        handleError(error);
    }
}

async function getMaterials(gamepath, vmfContent, outputDir) {
    const loadingInterval = showLoadingIndicator('Extracting materials');
    const matgeters = ["material", "texture", "detailmaterial"];
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
    process.stdout.write(`\r\x1b[90m${' '.repeat(50)}\r`); // Efface complètement la ligne
    console.log(`\n✅ \x1b[32m${materials.length} \x1b[37mmaterials have been extracted to the output folder\x1b[0m`);
}

async function getModels(gamepath, vmfContent, outputDir) {
    const loadingInterval = showLoadingIndicator('Extracting models');
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
    process.stdout.write(`\r\x1b[90m${' '.repeat(50)}\r`); // Efface complètement la ligne
    console.log(`\n✅ \x1b[32m${models.length} \x1b[37mmaterials have been extracted to the output folder\x1b[0m`);
}

async function getSounds(gamepath, vmfContent, outputDir) {
    const loadingInterval = showLoadingIndicator('Extracting sounds');
    const response = await fetch(SoundGettersURL);
    const text = await response.text();
    const SoundGetters = text.split('\n').map(line => line.trim());

    let sounds = [];

    for (const getter of SoundGetters) {
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
    process.stdout.write(`\r\x1b[90m${' '.repeat(50)}\r`); // Efface complètement la ligne
    console.log(`\n✅ \x1b[32m${sounds.length} \x1b[37msounds have been extracted to the output folder.\x1b[0m`);
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