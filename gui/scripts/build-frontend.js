import { execSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

/**
 * Absolute path to the `gui` directory.
 *
 * This script is invoked by Tauri as `beforeBuildCommand`/`beforeDevCommand`.
 * In Tauri v2 the command runs from the project root (workspace root),
 * so `gui/scripts/build-frontend.js` is the entry point.
 * We resolve the path relative to *this* file (which is in `gui/scripts`).
 */
const guiDir = path.resolve(__dirname, '..');

const mode = process.argv[2] || 'build';

if (mode === 'dev') {
  execSync('pnpm run colors && pnpm run dev', { cwd: guiDir, stdio: 'inherit' });
} else {
  execSync('pnpm run colors && pnpm run build', { cwd: guiDir, stdio: 'inherit' });
}