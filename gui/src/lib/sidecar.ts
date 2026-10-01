/**
 * Thin typed wrappers over the Rust IPC commands.
 *
 * Every wrapper maps 1:1 to a `#[tauri::command]` in `src-tauri/src/lib.rs`.
 * Tauri converts camelCase JavaScript argument keys to the snake_case Rust
 * parameter names, so `{ batchId }` reaches `batch_id` and `{ apiKey }`
 * reaches `api_key`.
 */
import { invoke } from '@tauri-apps/api/core';
import type {
  BatchResult,
  ConfigBatchResult,
  ConfigUpdate,
  ConfigValidation,
  TestConnectionResult,
  UndoResult,
} from './types';

export type {
  BatchResult,
  ConfigBatchResult,
  ConfigUpdate,
  ConfigValidation,
  FileResult,
  TestConnectionResult,
  UndoFileResult,
  UndoResult,
} from './types';

export interface RenameOptions {
  dryRun?: boolean;
  provider?: string;
}

/**
 * Run the rename pipeline.
 *
 * Rejects when the backend itself fails (config could not be read, history
 * could not be loaded); per-file problems are reported inside the returned
 * `BatchResult.files` rather than thrown.
 */
export function renameFiles(
  paths: string[],
  options: RenameOptions = {},
): Promise<BatchResult> {
  return invoke<BatchResult>('rename_files', { paths, options });
}

/** Ask the running batch to stop. Always resolves; the flag is idempotent. */
export function cancelRename(): Promise<boolean> {
  return invoke<boolean>('cancel_rename');
}

/** Undo a batch, or the most recent un-undone batch when `batchId` is absent. */
export function undoRename(batchId?: string): Promise<UndoResult> {
  return invoke<UndoResult>('undo_rename', { batchId: batchId ?? null });
}

/** The persisted config, exactly as the backend sees it. */
export function getConfig(): Promise<Record<string, unknown>> {
  return invoke<Record<string, unknown>>('get_config');
}

/** Absolute path of the settings file. */
export function getConfigPath(): Promise<string> {
  return invoke<string>('get_config_path');
}

/**
 * Directory holding the undo log, resolved by the backend.
 *
 * This must not be derived from `appDataDir()` in the frontend: the backend
 * honours portable mode and a custom `undo.log_path`, which `appDataDir()`
 * knows nothing about.
 */
export function getUndoLogDir(): Promise<string> {
  return invoke<string>('get_undo_log_path');
}

export function validateConfig(): Promise<ConfigValidation> {
  return invoke<ConfigValidation>('validate_config');
}

export function testApiConnection(
  provider: string,
  apiKey: string,
  model: string,
): Promise<TestConnectionResult> {
  return invoke<TestConnectionResult>('test_connection', { provider, apiKey, model });
}

/**
 * Persist a set of `section.field` updates.
 *
 * The backend applies them to the raw on-disk config, so `${ENV_VAR}`
 * references survive a save instead of being replaced by their resolved
 * values.
 */
export function saveConfigBatch(updates: ConfigUpdate[]): Promise<ConfigBatchResult> {
  return invoke<ConfigBatchResult>('save_app_config_batch', { updates });
}

export function getVersion(): Promise<string> {
  return invoke<string>('get_version');
}

export function isPortableApp(): Promise<boolean> {
  return invoke<boolean>('is_portable_app');
}

/** Extensions the backend can extract from, without the leading dot. */
export function getSupportedExtensions(): Promise<string[]> {
  return invoke<string[]>('get_supported_extensions_list');
}
