const { run, logError } = require('@tauri-apps/cli/main');

if (process.platform === 'win32') {
  process.env.RUSTUP_TOOLCHAIN ||= 'stable-x86_64-pc-windows-msvc';
}

const args = process.argv.slice(2);
const binName = `npm run ${process.env.npm_lifecycle_event || 'tauri'}`;

run(args, binName).catch((error) => {
  logError(error.message);
  process.exit(1);
});
