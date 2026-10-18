// MapScraper v1.8 - Entry Point
const { main } = require('./src/index');

main().catch((err) => {
  console.error('\n❌ Fatal error:', err);
  console.log('\nPress Enter to exit...');
  process.stdin.resume();
  process.stdin.on('data', () => process.exit(1));
});