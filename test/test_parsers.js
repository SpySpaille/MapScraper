const assert = require('assert');
const fs = require('fs');
const path = require('path');
const fse = require('fs-extra');

const { parseVMT, normalizeTexturePath, stripVMTComments } = require('../src/parsers/vmtParser');
const { parseVMF } = require('../src/parsers/vmfParser');
const { cleanSoundPath } = require('../src/extractors/sounds');
const { SearchPathManager } = require('../src/core/searchPaths');
const { InstanceResolver } = require('../src/parsers/instanceResolver');

console.log('🧪 Starting MapScraper v1.8 Test Suite...\n');

let passedTests = 0;
function test(name, fn) {
  try {
    fn();
    console.log(`  ✅ ${name}`);
    passedTests++;
  } catch (err) {
    console.error(`  ❌ ${name}`);
    console.error(err);
    process.exit(1);
  }
}

async function runAsyncTest(name, fn) {
  try {
    await fn();
    console.log(`  ✅ ${name}`);
    passedTests++;
  } catch (err) {
    console.error(`  ❌ ${name}`);
    console.error(err);
    process.exit(1);
  }
}

// --- Test 1: VMT Comments & KeyValues Parsing ---
test('VMT Parser: comments and normal KeyValues', () => {
  const sampleVMT = `
    // This is a comment
    /* Multi line
       comment here */
    "LightmappedGeneric"
    {
      "$basetexture" "brick/brickwall001"
      "$bumpmap"     "brick/brickwall001_normal.vtf"
      "$surfaceprop" "brick"
      "%keywords"    "c17industrial"
    }
  `;

  const map = parseVMT(sampleVMT);
  assert(map.has('basetexture'), 'Should have basetexture');
  assert.strictEqual(map.get('basetexture')[0], 'brick/brickwall001');
  assert(map.has('bumpmap'), 'Should have bumpmap');
  assert.strictEqual(map.get('bumpmap')[0], 'brick/brickwall001_normal.vtf');
  assert(map.has('keywords'), 'Should have keywords without % prefix');
  assert.strictEqual(map.get('keywords')[0], 'c17industrial');
});

// --- Test 2: VMT Patch Shaders & Includes ---
test('VMT Parser: Patch shader and include directives', () => {
  const samplePatch = `
    "Patch"
    {
      "include" "materials/models/props_junk/woodcrates01.vmt"
      "insert"
      {
        "$detail" "detail/wood_detail"
      }
    }
  `;

  const map = parseVMT(samplePatch);
  assert(map.has('include'), 'Should extract include');
  assert.strictEqual(map.get('include')[0], 'materials/models/props_junk/woodcrates01.vmt');
  assert(map.has('detail'), 'Should extract detail from insert block');
});

// --- Test 3: Texture Path Normalization ---
test('VMT Parser: normalizeTexturePath', () => {
  assert.strictEqual(normalizeTexturePath('materials\\models\\crate.vtf'), 'models/crate');
  assert.strictEqual(normalizeTexturePath('concrete/floor01.vmt'), 'concrete/floor01');
  assert.strictEqual(normalizeTexturePath('"brick/wall"'), 'brick/wall');
});

// --- Test 4: Sound Flag Stripping ---
test('Sounds: cleanSoundPath strips Source Engine flags', () => {
  assert.strictEqual(cleanSoundPath(')weapons/c4/c4_beep1.wav'), 'weapons/c4/c4_beep1.wav');
  assert.strictEqual(cleanSoundPath('^ambient/wind/wind_gust1.wav'), 'ambient/wind/wind_gust1.wav');
  assert.strictEqual(cleanSoundPath('*ambient/machines/generator.wav'), 'ambient/machines/generator.wav');
  assert.strictEqual(cleanSoundPath('sound/doors/door_stop1.wav'), 'doors/door_stop1.wav');
});

// --- Test 5: VMF Parser ---
test('VMF Parser: skybox, brush materials, models, sounds, instances', () => {
  const sampleVMF = `
    world
    {
      "id" "1"
      "mapversion" "1"
      "classname" "worldspawn"
      "skyname" "sky_day01_01"
      solid
      {
        "id" "2"
        side
        {
          "id" "3"
          "material" "BRICK/BRICKWALL001"
        }
      }
    }
    entity
    {
      "id" "10"
      "classname" "prop_static"
      "model" "models/props_c17/chair02a.mdl"
    }
    entity
    {
      "id" "11"
      "classname" "ambient_generic"
      "message" ")ambient/levels/canals/water_rivulet_loop2.wav"
    }
    entity
    {
      "id" "12"
      "classname" "env_skypaint"
      "startexture" "skybox/clouds.vtf"
    }
    entity
    {
      "id" "13"
      "classname" "func_instance"
      "file" "instances/doors/door_instance.vmf"
    }
  `;

  const parsed = parseVMF(sampleVMF, ['message']);

  // Skybox (6 faces)
  assert(parsed.materials.has('skybox/sky_day01_01up'), 'Should have skybox up face');
  assert(parsed.materials.has('skybox/sky_day01_01bk'), 'Should have skybox bk face');

  // Brush material
  assert(parsed.materials.has('BRICK/BRICKWALL001'), 'Should have brush material');

  // Direct VTF
  assert(parsed.directVtfs.has('skybox/clouds'), 'Should have startexture without .vtf');

  // Models
  assert(parsed.models.has('models/props_c17/chair02a.mdl'), 'Should have static prop model');

  // Sounds
  assert(parsed.sounds.has('ambient/levels/canals/water_rivulet_loop2.wav'), 'Should have sound without ) flag');

  // Func instance
  assert(parsed.instances.has('instances/doors/door_instance.vmf'), 'Should have func_instance');
});

// --- Test 6: Search Paths & Mount Resolution ---
test('SearchPathManager: resolve and mounts parsing', () => {
  const tmpDir = path.join(__dirname, 'tmp_test_search');
  fse.emptyDirSync(tmpDir);

  const primaryDir = path.join(tmpDir, 'primary');
  const mountDir = path.join(tmpDir, 'mount_css');
  fse.ensureDirSync(path.join(primaryDir, 'cfg'));
  fse.ensureDirSync(path.join(primaryDir, 'materials', 'primary_mat'));
  fse.ensureDirSync(path.join(mountDir, 'materials', 'cstrike_mat'));

  // Create test files
  fs.writeFileSync(path.join(primaryDir, 'materials', 'primary_mat', 'wood.vmt'), 'test');
  fs.writeFileSync(path.join(mountDir, 'materials', 'cstrike_mat', 'de_dust.vmt'), 'test');

  // Create mount.cfg pointing to mountDir
  const mountCfgContent = `
    "mountcfg"
    {
      "cstrike" "${mountDir.replace(/\\/g, '\\\\')}"
    }
  `;
  fs.writeFileSync(path.join(primaryDir, 'cfg', 'mount.cfg'), mountCfgContent);

  const manager = new SearchPathManager(primaryDir);
  assert.strictEqual(manager.allPaths.length, 2, 'Should discover mount.cfg path');

  const resPrimary = manager.resolve('materials/primary_mat/wood.vmt');
  assert(resPrimary, 'Should find primary asset');

  const resMount = manager.resolve('materials/cstrike_mat/de_dust.vmt');
  assert(resMount, 'Should find asset located in mounted directory');

  // Cleanup
  fse.removeSync(tmpDir);
});

// --- Test 7: Instance Resolver & Loop Prevention ---
test('InstanceResolver: recursive resolution and cycle avoidance', () => {
  const tmpDir = path.join(__dirname, 'tmp_test_inst');
  fse.emptyDirSync(tmpDir);

  const gameDir = path.join(tmpDir, 'game');
  fse.ensureDirSync(path.join(gameDir, 'maps', 'instances'));

  const inst1Path = path.join(gameDir, 'maps', 'instances', 'inst1.vmf');
  const inst2Path = path.join(gameDir, 'maps', 'instances', 'inst2.vmf');

  // inst1 references inst2
  fs.writeFileSync(inst1Path, `
    entity {
      "classname" "func_instance"
      "file" "instances/inst2.vmf"
    }
    entity {
      "classname" "prop_static"
      "model" "models/inst1_model.mdl"
    }
  `);

  // inst2 references inst1 (circular cycle test!)
  fs.writeFileSync(inst2Path, `
    entity {
      "classname" "func_instance"
      "file" "instances/inst1.vmf"
    }
    entity {
      "classname" "prop_static"
      "model" "models/inst2_model.mdl"
    }
  `);

  const rootVmf = path.join(gameDir, 'maps', 'root.vmf');
  fs.writeFileSync(rootVmf, 'root');

  const searchManager = new SearchPathManager(gameDir);
  const resolver = new InstanceResolver(searchManager);

  // Process starting with inst1
  const result = resolver.processInstances(['instances/inst1.vmf'], rootVmf);

  assert.strictEqual(result.instanceFiles.length, 2, 'Should process both instances without hanging in infinite loop');
  assert(result.models.has('models/inst1_model.mdl'), 'Should collect inst1 model');
  assert(result.models.has('models/inst2_model.mdl'), 'Should collect inst2 model');

  // Cleanup
  fse.removeSync(tmpDir);
});

console.log(`\n🎉 All ${passedTests} tests passed successfully!\n`);
