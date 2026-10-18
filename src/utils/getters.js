const fs = require('fs');
const path = require('path');
const fetch = require('node-fetch');

const GITHUB_USER = 'SpySpaille';
const BASE_RAW_URL = `https://raw.githubusercontent.com/${GITHUB_USER}/MapScraper/main/Getters`;

const URLS = {
  textures: `${BASE_RAW_URL}/TextureGetter.txt`,
  materials: `${BASE_RAW_URL}/MaterialGetter.txt`,
  sounds: `${BASE_RAW_URL}/SoundGetter.txt`
};

/**
 * Resolves the path to the local Getters directory.
 * Works both when running directly via Node and inside a pkg executable.
 */
function getLocalGettersDir() {
  const possiblePaths = [
    path.join(__dirname, '..', '..', 'Getters'),
    path.join(process.cwd(), 'Getters'),
    path.join(path.dirname(process.execPath), 'Getters')
  ];

  for (const p of possiblePaths) {
    if (fs.existsSync(p)) return p;
  }
  return path.join(__dirname, '..', '..', 'Getters');
}

/**
 * Loads a getter file from disk.
 * @param {string} filename
 * @returns {string[]}
 */
function loadLocalGetterFile(filename) {
  try {
    const filePath = path.join(getLocalGettersDir(), filename);
    if (fs.existsSync(filePath)) {
      return fs.readFileSync(filePath, 'utf-8')
        .split(/\r?\n/)
        .map(l => l.trim().toLowerCase().replace(/^\$/, ''))
        .filter(l => l.length > 0 && !l.startsWith('//'));
    }
  } catch (err) {
    // ignore
  }
  return [];
}

/**
 * Attempts to fetch a getter file from GitHub with a timeout, falls back to local file.
 * @param {string} type
 * @param {string} filename
 * @returns {Promise<string[]>}
 */
async function loadGetterList(type, filename) {
  const localList = loadLocalGetterFile(filename);
  const url = URLS[type];
  if (!url) return localList;

  try {
    const controller = new (global.AbortController || require('abort-controller'))();
    const timeout = setTimeout(() => controller.abort(), 2500);

    const res = await fetch(url, { signal: controller.signal });
    clearTimeout(timeout);

    if (res.ok) {
      const text = await res.text();
      const list = text
        .split(/\r?\n/)
        .map(l => l.trim().toLowerCase().replace(/^\$/, ''))
        .filter(l => l.length > 0 && !l.startsWith('//'));

      if (list.length > 0) {
        return Array.from(new Set([...list, ...localList]));
      }
    }
  } catch {
    // Network failure / offline: fallback to local list
  }

  return localList;
}

/**
 * Loads all getters (textures, materials, sounds) in parallel.
 * @returns {Promise<{textureGetters: string[], materialGetters: string[], soundGetters: string[]}>}
 */
async function loadAllGetters() {
  const [textures, materials, sounds] = await Promise.all([
    loadGetterList('textures', 'TextureGetter.txt'),
    loadGetterList('materials', 'MaterialGetter.txt'),
    loadGetterList('sounds', 'SoundGetter.txt')
  ]);

  return {
    textureGetters: textures,
    materialGetters: materials,
    soundGetters: sounds
  };
}

module.exports = {
  loadAllGetters,
  getLocalGettersDir
};
