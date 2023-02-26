const fs = require('fs');
const fse = require('fs-extra');
const MDL = require('source-mdl');
const path = require('path');

const outputDir = './output';
const gpath = "E:\\Program Files (x86)\\Steam\\steamapps\\common\\GarrysMod\\garrysmod";
const file = "rp_site_kb_v1.vmf";
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
    const regex = /"material"\s*"([^"]+)"/g;
    const materials = [];

    let match;
    while (match = regex.exec(vmfContent)) {
        const material = match[1];
        if (!materials.includes(material)) {
            materials.push(material);
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
    });
}

getMaterials();
getModels();
getSounds();