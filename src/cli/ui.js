const colors = require('@colors/colors');
const figlet = require('figlet');
const fetch = require('node-fetch');
const { version } = require('../../package.json');

const githubUser = 'SpySpaille';
const apiReleasesURL = `https://api.github.com/repos/${githubUser}/MapScraper/releases/latest`;
const rawLatestURL = `https://raw.githubusercontent.com/${githubUser}/MapScraper/main/package.json`;

const yn = '\x1b[37m[\x1b[32my\x1b[37m/\x1b[31mn\x1b[37m]\x1b[90m >';

/**
 * Checks GitHub for the latest release version.
 * First queries GitHub Releases API (instant, no CDN caching), then falls back to raw package.json.
 * @returns {Promise<string|null>}
 */
async function checkLatestVersion() {
  // 1. Try GitHub Releases API first (instant, avoids raw.githubusercontent.com 5-minute CDN cache)
  try {
    const controller = new (global.AbortController || require('abort-controller'))();
    const timeout = setTimeout(() => controller.abort(), 2000);
    const res = await fetch(apiReleasesURL, {
      signal: controller.signal,
      headers: { 'User-Agent': 'MapScraper' }
    });
    clearTimeout(timeout);
    if (res.ok) {
      const data = await res.json();
      if (data && data.tag_name) {
        return data.tag_name.replace(/^v/i, '');
      }
    }
  } catch {
    // API failed or rate-limited, fallback below
  }

  // 2. Fallback to raw package.json
  try {
    const controller = new (global.AbortController || require('abort-controller'))();
    const timeout = setTimeout(() => controller.abort(), 2000);
    const res = await fetch(rawLatestURL, { signal: controller.signal });
    clearTimeout(timeout);
    if (res.ok) {
      const data = await res.json();
      return data.version;
    }
  } catch {
    // Offline or unreachable
  }
  return null;
}

/**
 * Prints the welcome banner with version comparison.
 */
async function showBanner() {
  return new Promise((resolve) => {
    figlet('MapScraper', async (err, data) => {
      if (err) {
        console.log(colors.green('=== MapScraper ==='));
      } else {
        console.log(colors.green(data));
      }
      console.log(`\x1b[90mMade by \x1b[37mSpySpaille \x1b[90mwith ❤️`);

      const latestVersion = await checkLatestVersion();
      if (!latestVersion || latestVersion === version) {
        console.log(`\x1b[90m-\x1b[32m Version: \x1b[37m${version}\n`);
      } else if (parseFloat(latestVersion) > parseFloat(version)) {
        console.log(`\x1b[90m-\x1b[32m Version: \x1b[37m${version}\x1b[31m (Outdated)\n\x1b[90m-\x1b[32m Latest Version: \x1b[37m${latestVersion} \x1b[90m(https://github.com/${githubUser}/MapScraper)\n`);
      } else {
        console.log(`\x1b[90m-\x1b[32m Version: \x1b[37m${version}\x1b[32m (Latest)\n`);
      }
      resolve();
    });
  });
}

/**
 * Displays an animated loading spinner.
 * @param {string} message
 * @returns {NodeJS.Timeout}
 */
function showLoadingIndicator(message) {
  const loadingChars = ['|', '/', '-', '\\'];
  let loadingIndex = 0;
  const loadingInterval = setInterval(() => {
    process.stdout.write(`\r\x1b[90m- \x1b[90m${message} \x1b[37m${loadingChars[loadingIndex]}\x1b[0m`);
    loadingIndex = (loadingIndex + 1) % loadingChars.length;
  }, 200);
  return loadingInterval;
}

/**
 * Stops the spinner and clears the line.
 * @param {NodeJS.Timeout} interval
 */
function stopLoadingIndicator(interval) {
  if (interval) clearInterval(interval);
  process.stdout.write(`\r\x1b[90m${' '.repeat(70)}\r`);
}

/**
 * Waits for the user to press Enter before closing the CLI.
 */
function waitForExit() {
  console.log('\n\x1b[90mPress Enter to close the program...\x1b[0m');
  process.stdin.resume();
  process.stdin.on('data', () => process.exit(0));
}

module.exports = {
  yn,
  showBanner,
  showLoadingIndicator,
  stopLoadingIndicator,
  waitForExit
};
