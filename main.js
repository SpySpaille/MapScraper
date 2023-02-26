const fs = require('fs');
const fse = require('fs-extra');
const path = require('path');

const outputDir = './output';
const gpath = "E:\\Program Files (x86)\\Steam\\steamapps\\common\\GarrysMod\\garrysmod";
const file = "texturefinder.vmf";
const vmfContent = fs.readFileSync(file, 'utf-8');
const TextureGettersURL = "https://pastebin.com/raw/1cR1XKww";

fetch(TextureGettersURL).then(response => response.text()).then(text => {
    // For each line in the pastebin, add it to the array
    const TextureGetters = text.split('\n').map(line => line.trim());
    
    fse.removeSync(outputDir);
    fse.ensureDirSync(outputDir);

    // Get all materials
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
        // Copy materials to output directory
        const sourcePath = path.join(gpath, 'materials', material + '.vmt');
        const destPath = path.join(outputDir, 'materials', material.toLowerCase() + '.vmt');
        fse.copySync(sourcePath, destPath);

        // Get VTFs from materials
        const textures = [];
        const materialContent = fs.readFileSync(sourcePath, 'utf-8').replace(/"/g, '').replace(/\\/g, '/');

        TextureGetters.forEach(getter => {
            if (materialContent.includes(getter)) {
                const regex = new RegExp(`\\${getter}\\s*([^\\s]+)`, 'g');
                let match;
                while (match = regex.exec(materialContent)) {
                    const texture = match[1];
                    if (!textures.includes(texture)) {
                        textures.push(texture);
                    }
                }
            }
        });

        // Copy VTFs to output directory
        textures.forEach(texture => {
            const sourcePath = path.join(gpath, 'materials', texture + '.vtf');
            const destPath = path.join(outputDir, 'materials', texture.toLowerCase() + '.vtf');
            fse.copySync(sourcePath, destPath);
        });
    });
});