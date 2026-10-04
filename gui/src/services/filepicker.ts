/**
 * File picker service using Tauri dialog and fs plugins
 */
import { open } from '@tauri-apps/plugin-dialog';
import { readDir } from '@tauri-apps/plugin-fs';
import { join } from '@tauri-apps/api/path';
import { isSupportedFile, SUPPORTED_EXTENSIONS } from '@/utils/helpers';

const MAX_FOLDER_DEPTH = 8;

/**
 * Recursively collect supported files from a folder
 */
export async function expandFolder(
  folderPath: string,
  depth = 0
): Promise<string[]> {
  if (depth > MAX_FOLDER_DEPTH) return [];

  let entries;
  try {
    entries = await readDir(folderPath);
  } catch {
    return [];
  }

  const files: string[] = [];
  for (const entry of entries) {
    const name = entry.name;
    if (!name) continue;

    const fullPath = await join(folderPath, name);

    if (entry.isDirectory) {
      files.push(...await expandFolder(fullPath, depth + 1));
    } else if (entry.isFile && isSupportedFile(fullPath)) {
      files.push(fullPath);
    }
  }
  return files;
}

/**
 * Open file picker for multiple files
 */
export async function pickFiles(): Promise<string[]> {
  const result = await open({
    multiple: true,
    filters: [
      {
        name: 'Supported Documents',
        extensions: SUPPORTED_EXTENSIONS.map(e => e.slice(1)),
      },
    ],
  });

  if (result === null) return [];
  return Array.isArray(result) ? result : [result];
}

/**
 * Open folder picker and return all supported files
 * Returns null if cancelled
 */
export async function pickFolder(): Promise<string[] | null> {
  const folder = await open({ directory: true, multiple: false });
  if (folder === null || Array.isArray(folder)) return null;
  return expandFolder(folder);
}