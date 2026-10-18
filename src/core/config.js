const fs = require('fs');
const fse = require('fs-extra');
const path = require('path');

function getConfigDir() {
  const base = process.env.APPDATA || (process.platform === 'darwin' ? path.join(process.env.HOME, 'Library/Preferences') : path.join(process.env.HOME, '.config'));
  return path.join(base, 'MapScraper');
}

function getConfigPath() {
  return path.join(getConfigDir(), 'config.json');
}

function loadConfig() {
  const configPath = getConfigPath();
  try {
    if (fs.existsSync(configPath)) {
      const content = fs.readFileSync(configPath, 'utf-8');
      return JSON.parse(content);
    }
  } catch (err) {
    // If corrupt, return empty config
  }
  return {};
}

function saveConfig(config) {
  try {
    const configDir = getConfigDir();
    fse.ensureDirSync(configDir);
    fs.writeFileSync(getConfigPath(), JSON.stringify(config, null, 2), 'utf-8');
  } catch (err) {
    // ignore write errors
  }
}

function clearConfig() {
  try {
    const configPath = getConfigPath();
    if (fs.existsSync(configPath)) {
      fse.removeSync(configPath);
    }
  } catch {
    // ignore
  }
}

module.exports = {
  loadConfig,
  saveConfig,
  clearConfig,
  getConfigPath
};
