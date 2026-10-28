const { run, logError } = require('@tauri-apps/cli/main');

if (process.platform === 'win32') {
  // rsraw-sys needs the GNU toolchain. MinGW binutils also needs a target
  // directory without spaces; keep that Windows-only choice out of Cargo's
  // project-wide configuration so macOS and Linux use their native defaults.
  process.env.CARGO_BUILD_TARGET ||= 'x86_64-pc-windows-gnu';
  process.env.CARGO_TARGET_DIR ||= 'C:/cargo-target/mundam';
}

const args = process.argv.slice(2);
const binName = `npm run ${process.env.npm_lifecycle_event || 'tauri'}`;

run(args, binName).catch((error) => {
  logError(error.message);
  process.exit(1);
});
