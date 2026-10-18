const fs = require('fs');
const fse = require('fs-extra');
const path = require('path');

/**
 * Removes surrounding double quotes from a string or path.
 * @param {string} str
 * @returns {string}
 */
function removeQuotes(str) {
  if (!str) return '';
  return str.trim().replace(/^["']|["']$/g, '');
}

/**
 * Normalizes a relative Source Engine path:
 * replaces backslashes with forward slashes and trims whitespace.
 * @param {string} p
 * @returns {string}
 */
function normalizeSourcePath(p) {
  if (!p) return '';
  return removeQuotes(p).replace(/\\/g, '/').replace(/^\/+/, '');
}

/**
 * Checks whether a file exists at the given path (case-insensitive fallback on non-Windows if needed).
 * @param {string} filePath
 * @returns {boolean}
 */
function fileExists(filePath) {
  if (!filePath) return false;
  try {
    return fs.existsSync(filePath);
  } catch {
    return false;
  }
}

/**
 * Safely copies a file from source to destination, ensuring the destination directory exists.
 * @param {string} src
 * @param {string} dest
 * @returns {Promise<boolean>}
 */
async function safeCopy(src, dest) {
  try {
    if (!fileExists(src)) return false;
    await fse.ensureDir(path.dirname(dest));
    await fse.copy(src, dest, { overwrite: true });
    return true;
  } catch (err) {
    // Suppress ENOENT errors if file disappeared, log others if necessary
    if (err.code !== 'ENOENT') {
      // console.warn(`Copy warning: ${err.message}`);
    }
    return false;
  }
}

/**
 * Safely copies a file synchronously.
 * @param {string} src
 * @param {string} dest
 * @returns {boolean}
 */
function safeCopySync(src, dest) {
  try {
    if (!fileExists(src)) return false;
    fse.ensureDirSync(path.dirname(dest));
    fse.copySync(src, dest, { overwrite: true });
    return true;
  } catch {
    return false;
  }
}

/**
 * Simple async concurrency limiter.
 * @param {number} concurrency
 * @returns {function(function(): Promise<any>): Promise<any>}
 */
function pLimit(concurrency = 16) {
  const queue = [];
  let activeCount = 0;

  const next = () => {
    activeCount--;
    if (queue.length > 0) {
      queue.shift()();
    }
  };

  const run = async (fn, resolve, reject) => {
    activeCount++;
    try {
      const result = await fn();
      resolve(result);
    } catch (err) {
      reject(err);
    } finally {
      next();
    }
  };

  return (fn) =>
    new Promise((resolve, reject) => {
      if (activeCount < concurrency) {
        run(fn, resolve, reject);
      } else {
        queue.push(() => run(fn, resolve, reject));
      }
    });
}

module.exports = {
  removeQuotes,
  normalizeSourcePath,
  fileExists,
  safeCopy,
  safeCopySync,
  pLimit
};
