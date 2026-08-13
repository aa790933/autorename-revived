import { rename as fsRename, readTextFile, writeTextFile } from '@tauri-apps/plugin-fs';
import { appDataDir } from '@tauri-apps/api/path';
import type { FileEntry } from './state';
import type { BatchResult, FileResult } from './types';

function joinPath(dir: string, name: string): string {
  const sep = dir.includes('\\') ? '\\' : '/';
  return dir + sep + name;
}

/** Generate a batch ID matching the Python format: 8-char hex. */
export function generateBatchId(): string {
  return Array.from(crypto.getRandomValues(new Uint8Array(4)))
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('');
}

interface UndoLogV2 {
  version: 2;
  batches: Array<{
    batch_id: string;
    timestamp: string;
    source: string;
    undone: boolean;
    files: Array<{ old_path: string; new_path: string; timestamp: string }>;
  }>;
}

async function getUndoLogPath(): Promise<string> {
  const dataDir = await appDataDir();
  return joinPath(dataDir, 'rename_history.json');
}

/** Read existing undo log and migrate v1 (bare array) to v2 if needed. */
async function readUndoLog(logPath: string): Promise<UndoLogV2> {
  try {
    const raw = JSON.parse(await readTextFile(logPath));
    if (Array.isArray(raw)) {
      return {
        version: 2,
        batches: raw.length > 0
          ? [{
              batch_id: 'migrated-v1',
              timestamp: raw[0]?.timestamp ?? '',
              source: 'cli',
              undone: false,
              files: raw,
            }]
          : [],
      };
    }
    return raw as UndoLogV2;
  } catch {
    return { version: 2, batches: [] };
  }
}

/**
 * Apply cached dry-run results by renaming files directly via Tauri FS.
 * Writes a CLI-compatible v2 undo log so `undo` works regardless of how
 * the rename was performed (cached or full CLI pipeline).
 */
export async function applyCachedRenames(
  files: FileEntry[],
  _baseDir: string,
  onProgress?: (msg: string) => void,
): Promise<BatchResult> {
  const results: FileResult[] = [];
  let completed = 0;
  let skipped = 0;
  let failed = 0;
  const undoEntries: Array<{ old_path: string; new_path: string; timestamp: string }> = [];

  const batchId = generateBatchId();
  const total = files.length;
  for (let i = 0; i < files.length; i++) {
    const entry = files[i];
    const cached = entry.result;

    if (!cached?.new_path || cached.status === 'skipped') {
      skipped++;
      results.push(cached
        ? { ...cached, status: 'skipped' }
        : { file: entry.path, status: 'skipped', new_name: null, new_path: null, error: null, warnings: [], company: null, date: null, doc_type: null, provider: null, model: null, suggestion_names: [], suggestion_languages: [] },
      );
      continue;
    }

    onProgress?.(`Renaming [${i + 1}/${total}] ${entry.name}`);

    try {
      await fsRename(entry.path, cached.new_path);
      completed++;
      results.push({ ...cached, status: 'completed' });
      undoEntries.push({
        old_path: entry.path,
        new_path: cached.new_path,
        timestamp: new Date().toISOString(),
      });
    } catch (err) {
      failed++;
      results.push({ ...cached, status: 'failed', error: String(err) });
    }
  }

  if (undoEntries.length > 0) {
    const logPath = await getUndoLogPath();
    try {
      const logData = await readUndoLog(logPath);
      logData.batches.push({
        batch_id: batchId,
        timestamp: new Date().toISOString(),
        source: 'gui',
        undone: false,
        files: undoEntries,
      });
      await writeTextFile(logPath, JSON.stringify(logData, null, 2));
    } catch { /* undo log write failure is non-critical */ }
  }

  return { success: failed === 0, total, completed, skipped, failed, files: results, dry_run: false, batch_id: batchId };
}
