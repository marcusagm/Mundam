const { run, logError } = require('@tauri-apps/cli/main');
const { accessSync, constants } = require('node:fs');
const { homedir } = require('node:os');
const { delimiter, join } = require('node:path');

// Setup cannot export PATH back to the terminal that launched it. Find rustup's
// Cargo when running from that same terminal, while preserving an existing Cargo.
const cargoName = process.platform === 'win32' ? 'cargo.exe' : 'cargo';
const canExecuteCargo = (directory) => {
  try {
    accessSync(join(directory, cargoName), constants.X_OK);
    return true;
  } catch {
    return false;
  }
};
const pathDirectories = (process.env.PATH || '').split(delimiter);
if (!pathDirectories.some(canExecuteCargo)) {
  const cargoBin = join(process.env.CARGO_HOME || join(homedir(), '.cargo'), 'bin');
  if (canExecuteCargo(cargoBin)) {
    process.env.PATH = [cargoBin, ...pathDirectories].join(delimiter);
  }
}

if (process.platform === 'win32') {
  process.env.RUSTUP_TOOLCHAIN ||= 'stable-x86_64-pc-windows-msvc';
}

const args = process.argv.slice(2);
const binName = `npm run ${process.env.npm_lifecycle_event || 'tauri'}`;

run(args, binName).catch((error) => {
  logError(error.message);
  process.exit(1);
});
