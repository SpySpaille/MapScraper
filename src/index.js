const { showBanner, waitForExit } = require('./cli/ui');
const { promptUser } = require('./cli/prompts');
const { runScraper } = require('./core/scraper');

async function main() {
  try {
    await showBanner();
    console.log('📜 Please enter the following information:\n');

    const options = await promptUser();
    await runScraper(options);
  } catch (err) {
    if (err && err.message !== 'canceled') {
      console.error('\n❌ \x1b[31mAn unexpected error occurred:\x1b[0m');
      console.error(err);
    }
  } finally {
    waitForExit();
  }
}

// Global error handling
process.on('uncaughtException', (err) => {
  if (err && err.code !== 'ENOENT') {
    console.error('\n❌ \x1b[31mUncaught Exception:\x1b[0m', err.message);
  }
  waitForExit();
});

process.on('unhandledRejection', (reason) => {
  console.error('\n❌ \x1b[31mUnhandled Rejection:\x1b[0m', reason);
  waitForExit();
});

if (require.main === module) {
  main();
}

module.exports = { main };
