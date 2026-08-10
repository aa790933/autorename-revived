import { open } from '@tauri-apps/plugin-dialog';
import { readDir } from '@tauri-apps/plugin-fs';
import { SUPPORTED_EXTENSIONS, isSupportedFile } from './utils';

export async function expandFolder(folderPath: string): Promise<string[]> {
  const clean = folderPath.replace(/[\\/]+$/, '');
  const sep = clean.includes('\\') ? '\\' : '/';
  const entries = await readDir(clean);
  return entries
    .filter((e) => e.isFile && isSupportedFile(e.name))
    .map((e) => clean + sep + e.name);
}

export async function pickFiles(): Promise<string[]> {
  const result = await open({
    multiple: true,
    filters: [{ name: 'Supported Documents', extensions: SUPPORTED_EXTENSIONS.map((e) => e.slice(1)) }],
  });
  if (!result) return [];
  return Array.isArray(result) ? result : [result];
}

export async function pickFolder(): Promise<string[] | null> {
  const folder = await open({ directory: true, multiple: false }) as string | null;
  if (!folder) return null;
  return expandFolder(folder);
}
