const express = require('express');
app = express();

app.listen(3000, () => {

    const fs = require('fs');
    const fse = require('fs-extra');
    const MDL = require('source-mdl');
    const path = require('path');

    const prompt = require('prompt');
    const colors = require('@colors/colors');
    const figlet = require('figlet');
    const { version } = require('./package.json');
    const latestURL = "https://raw.githubusercontent.com/SpySpaille/MapScraper/main/package.json";

    fetch(latestURL).then(response => response.json()).then(json => {
        // Prompt for input
        prompt.message = '\x1b[90m-';
        prompt.delimiter = ' ';
        prompt.start();
        figlet('MapScrapper', function (err, data) {
            if (err) return console.log(err);
            console.log(colors.green(data));
            console.log(`\x1b[90mMade by \x1b[37mSpySpaille \x1b[90mwith ❤️`);
            if (json.version == version) {
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
                        // Check if the path is valid by checking if the /materials folder exists
                        conform: function (value) {
                            return fs.existsSync(path.join(value, 'materials'));
                        },
                        message: 'Invalid path'
                    },
                    file: {
                        description: '🗒️ \x1b[37mVMF File\x1b[90m >',
                        required: true,
                        conform: function (value) {
                            return value.endsWith('.vmf') && fs.existsSync(value);
                        },
                        message: 'Invalid file'
                    },
                    materials: {
                        description: '🧱 \x1b[37mExtract Materials\x1b[90m (y/n) >',
                        required: true,
                        conform: function (value) {
                            return value == 'y' || value == 'n';
                        },
                        message: 'Invalid input'
                    },
                    models: {
                        description: '📐 \x1b[37mExtract Models\x1b[90m (y/n) >',
                        required: true,
                        conform: function (value) {
                            return value == 'y' || value == 'n';
                        },
                        message: 'Invalid input'
                    },
                    sounds: {
                        description: '🔊 \x1b[37mExtract Sounds\x1b[90m (y/n) >',
                        required: true,
                        conform: function (value) {
                            return value == 'y' || value == 'n';
                        },
                        message: 'Invalid input'
                    },
                }
            }, function (err, result) {
                if (err) return console.log(err);
                script(result.gamepath, result.file, result.materials == 'y', result.models == 'y', result.sounds == 'y');
            });
        });
    });

    // Begin of the script
    function script(gpath, file, matbool, mdlbool, soundbool) {
        const outputDir = './output';
        const vmfContent = fs.readFileSync(file, 'utf-8');
        const TextureGettersURL = "https://raw.githubusercontent.com/SpySpaille/MapScraper/main/Getters/TextureGetter.txt";
        const modelsvariants = [".mdl", ".vvd", ".phy", ".dx90.vtx", ".dx80.vtx", ".sw.vtx", ".ani"];
        const SoundGettersURL = "https://raw.githubusercontent.com/SpySpaille/MapScraper/main/Getters/SoundGetter.txt";

        fse.removeSync(outputDir);
        fse.ensureDirSync(outputDir);


        // Get VTFs from materials
        function VTFfromVMT(sourcePath, outputDir) {
            fetch(TextureGettersURL).then(response => response.text()).then(text => {
                const TextureGetters = text.split('\n').map(line => line.trim());
                const textures = [];
                let materialContent;
                try {
                    materialContent = fs.readFileSync(sourcePath, 'utf-8').replace(/"+/g, '').replace(/\s+/g, ' ').trim().toLowerCase();
                } catch (err) { return; }

                TextureGetters.forEach(getter => {
                    if (materialContent.includes(getter)) {
                        const regex = new RegExp(`\\${getter} \\s*([^\\s]+)`, 'g');
                        let match;
                        while (match = regex.exec(materialContent)) {
                            const texture = match[1];
                            if (!textures.includes(texture)) {
                                textures.push(texture);
                            }
                        }
                    }
                });

                textures.forEach(texture => {
                    const sourcePath = path.join(gpath, 'materials', texture + '.vtf');
                    const destPath = path.join(outputDir, 'materials', texture.toLowerCase() + '.vtf');
                    try {
                        fse.copySync(sourcePath, destPath);
                    } catch (err) { return; }
                });
            });
        }

        // Get all materials
        function getMaterials() {
            const matgeters = ["material", "texture", "detailmaterial"];
            const materials = [];

            matgeters.forEach(getter => {
                const regex = new RegExp(`\\"${getter}\\"\\s*\\"([^"]+)\\"`, 'g');

                let match;
                while (match = regex.exec(vmfContent)) {
                    const material = match[1];
                    if (!materials.includes(material)) {
                        materials.push(material);
                    }
                }
            });

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

            materials.forEach(material => {
                const sourcePath = path.join(gpath, 'materials', material + '.vmt');
                const destPath = path.join(outputDir, 'materials', material.toLowerCase() + '.vmt');
                try {
                    fse.copySync(sourcePath, destPath);
                } catch (err) { return; }

                VTFfromVMT(sourcePath, outputDir);
            });

            console.log(`\n✅ \x1b[32m${materials.length} \x1b[37mmaterials have been extracted to the output folder`);
        }

        // Get all models
        function getModels() {
            const regex = /"model"\s*"([^"]+)"/g;
            const models = [];

            let match;
            while (match = regex.exec(vmfContent)) {
                const model = match[1];
                if (!models.includes(model)) {
                    if (model.endsWith('.mdl')) {
                        models.push(model);
                    }
                }
            }

            models.forEach(async model => {
                modelsvariants.forEach(variant => {
                    const sourcePath = path.join(gpath, model.replace('.mdl', variant));
                    const destPath = path.join(outputDir, model.replace('.mdl', variant));
                    try {
                        fse.copySync(sourcePath, destPath);
                    } catch (err) { return; }
                });

                try {
                    let mdlData = fs.readFileSync(path.join(gpath, model));
                    let mdl = new MDL();
                    mdl.import({ mdlData });
                    // Put mdl.getMetadata().textureDirs to lowercase into an array 
                    const textures = mdl.getMetadata().textures.map(texture => texture.toLowerCase());
                    const textureDirs = mdl.getMetadata().textureDirs.map(textureDir => textureDir.toLowerCase());
                    textureDirs.forEach(textureDir => {
                        textures.forEach(texture => {
                            const sourcePath = path.join(gpath, 'materials', textureDir, texture + '.vmt');
                            const destPath = path.join(outputDir, 'materials', textureDir, texture + '.vmt');
                            try {
                                fse.copySync(sourcePath, destPath);
                            } catch (err) { return; }
                            // Get VTFs from materials
                            VTFfromVMT(sourcePath, outputDir);
                        });
                    });
                } catch (err) { return; }
            });
            console.log(`\n✅ \x1b[32m${models.length} \x1b[37mmodels have been extracted to the output folder.\x1b[0m`);
        }

        // Get all sounds
        function getSounds() {
            fetch(SoundGettersURL).then(response => response.text()).then(text => {
                const SoundGetters = text.split('\n').map(line => line.trim());
                const sounds = [];

                SoundGetters.forEach(getter => {
                    const regex = new RegExp(`\\"${getter}\\"\\s*\\"([^"]+)\\"`, 'g');
                    let match;
                    while (match = regex.exec(vmfContent)) {
                        const sound = match[1];
                        if (!sounds.includes(sound)) {
                            sounds.push(sound);
                        }
                    }
                });

                sounds.forEach(sound => {
                    const sourcePath = path.join(gpath, 'sound', sound);
                    const destPath = path.join(outputDir, 'sound', sound.toLowerCase());
                    try {
                        fse.copySync(sourcePath, destPath);
                    } catch (err) { return; }
                });
                console.log(`\n✅ \x1b[32m${sounds.length} \x1b[37msounds have been extracted to the output folder.\x1b[0m`);
            });
        }

        // Get soundscapes
        function getSoundscapes() {
            if (vmfContent.includes('soundscape')) {
                const sourcePath = path.join(gpath, `scripts`, `soundscapes_${file.replace('.vmf', '.txt')}`);
                const destPath = path.join(outputDir, `scripts`, `soundscapes_${file.replace('.vmf', '.txt')}`);
                try {
                    fse.copySync(sourcePath, destPath);
                } catch (err) { return; }
            }
        }

        if (matbool) {
            getMaterials();
        }
        if (mdlbool) {
            getModels();
        }
        if (soundbool) {
            getSounds();
            getSoundscapes();
        }
        if (!matbool && !mdlbool && !soundbool) {
            console.log('\n❌ \x1b[31mYou have not selected anything. So I\'m not going to do anything, goodbye!\x1b[0m');
        }
    }
});
