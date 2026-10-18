const { removeQuotes } = require('../utils/fileSystem');

/**
 * Removes both single-line (//) and multi-line (/* * /) comments from VMT content.
 * @param {string} content
 * @returns {string}
 */
function stripVMTComments(content) {
  if (!content) return '';
  // Remove multi-line comments
  let cleaned = content.replace(/\/\*[\s\S]*?\*\//g, '');
  // Remove single-line comments
  cleaned = cleaned.replace(/\/\/.*$/gm, '');
  return cleaned;
}

/**
 * Parses a VMT file content into a Map of key -> string[].
 * Supports shaders, nested blocks, Patch includes, and handles $, %, or unadorned keys.
 * @param {string} vmtContent
 * @returns {Map<string, string[]>}
 */
function parseVMT(vmtContent) {
  const map = new Map();
  if (!vmtContent) return map;

  const cleaned = stripVMTComments(vmtContent);

  // Regex to match:
  // 1) Quoted key and quoted value: "key" "value"
  // 2) Quoted key and unquoted value: "key" value
  // 3) Unquoted key and quoted value: key "value"
  // 4) Unquoted key and unquoted value: key value
  // While ignoring block braces { }
  const kvRegex = /(?:\"([^\"]+)\"|(\$?[%a-zA-Z0-9_\-\.\/]+))\s+(?:\"([^\"]+)\"|([^\s\{\}\"]+))/g;

  let match;
  while ((match = kvRegex.exec(cleaned)) !== null) {
    let rawKey = match[1] || match[2];
    let rawVal = match[3] || match[4];

    if (!rawKey || !rawVal) continue;

    // Clean key: lowercase, strip quotes and leading $ or %
    let key = rawKey.trim().toLowerCase().replace(/^["']|["']$/g, '');
    if (key.startsWith('$') || key.startsWith('%')) {
      key = key.slice(1);
    }

    // Clean value: trim and strip quotes
    let value = rawVal.trim().replace(/^["']|["']$/g, '');

    if (key && value) {
      if (!map.has(key)) {
        map.set(key, []);
      }
      map.get(key).push(value);
    }
  }

  // Also specifically detect "include" directives for Patch shaders:
  // e.g. include "materials/models/props/test.vmt"
  const includeRegex = /["']?include["']?\s+["']([^"']+)["']/gi;
  let incMatch;
  while ((incMatch = includeRegex.exec(cleaned)) !== null) {
    const incVal = incMatch[1].trim();
    if (incVal) {
      if (!map.has('include')) {
        map.set('include', []);
      }
      if (!map.get('include').includes(incVal)) {
        map.get('include').push(incVal);
      }
    }
  }

  return map;
}

/**
 * Normalizes a texture path referenced in a VMT:
 * - Replaces backslashes with forward slashes
 * - Removes leading 'materials/' prefix if redundant
 * - Removes '.vmt' or '.vtf' extension
 * @param {string} texturePath
 * @returns {string}
 */
function normalizeTexturePath(texturePath) {
  if (!texturePath) return '';
  let p = texturePath.trim().replace(/\\/g, '/');
  p = p.replace(/^["']|["']$/g, '');
  p = p.replace(/^\/+/, '');
  if (p.toLowerCase().startsWith('materials/')) {
    p = p.slice('materials/'.length);
  }
  p = p.replace(/\.(vmt|vtf)$/i, '');
  return p;
}

module.exports = {
  parseVMT,
  normalizeTexturePath,
  stripVMTComments
};
