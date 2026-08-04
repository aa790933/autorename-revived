import { invoke } from '@tauri-apps/api/core';
import { join, appDataDir } from '@tauri-apps/api/path';

export interface FileResult {
  file: string;
  status: 'renamed' | 'skipped' | 'failed';
  new_name: string | null;
  new_path: string | null;
  error: string | null;
  warnings: string[];
  company: string | null;
  date: string | null;
  doc_type: string | null;
  provider: string | null;
  model: string | null;
}

export interface BatchResult {
  success: boolean;
  total: number;
  renamed: number;
  skipped: number;
  failed: number;
  files: FileResult[];
  dry_run: boolean;
  batch_id?: string;
}

export interface ErrorResult {
  success: false;
  error_type: string;
  message: string;
  suggestion: string;
}

export type SidecarResult = BatchResult | ErrorResult;

export interface UndoFileResult {
  old_path: string;
  new_path: string;
  status: 'restored' | 'failed';
  error?: string;
}

export interface UndoResult {
  success: boolean;
  restored: number;
  failed: number;
  files: UndoFileResult[];
  batch_id?: string;
}

export interface ConfigValidation {
  valid: boolean;
  issues: Array<{ field: string; level: string; message: string }>;
}

export interface TestConnectionResult {
  success: boolean;
  message: string;
  latency_ms: number;
  provider: string;
}

export function isErrorResult(result: SidecarResult): result is ErrorResult {
  return !result.success && 'error_type' in result;
}

export async function renamePdfs(
  paths: string[],
  options: {
    dryRun?: boolean;
    recursive?: boolean;
    provider?: string;
    model?: string;
    vision?: boolean;
    textOnly?: boolean;
  } = {},
  onProgress?: (line: string) => void,
): Promise<SidecarResult> {
  try {
    const result = await invoke<BatchResult>('rename_pdfs', {
      paths,
      options,
    });
    return result;
  } catch (e) {
    return {
      success: false,
      error_type: 'sidecar_error',
      message: String(e),
      suggestion: '',
    } as ErrorResult;
  }
}

export async function undoRename(batchId?: string): Promise<UndoResult | ErrorResult> {
  try {
    const result = await invoke<UndoResult>('undo_rename', { batchId });
    return result;
  } catch (e) {
    throw new Error(String(e));
  }
}

export async function getConfig(): Promise<Record<string, unknown>> {
  try {
    return await invoke<Record<string, unknown>>('get_config');
  } catch (e) {
    throw new Error(String(e));
  }
}

export async function getConfigPath(): Promise<string | null> {
  try {
    return await invoke<string | null>('get_config_path');
  } catch {
    return null;
  }
}

export async function getUndoLogDir(): Promise<string> {
  const dir = await appDataDir();
  return dir;
}

export async function validateConfig(): Promise<ConfigValidation> {
  try {
    return await invoke<ConfigValidation>('validate_config');
  } catch (e) {
    throw new Error(String(e));
  }
}

export async function testApiConnection(
  provider?: string,
  apiKey?: string,
  model?: string,
): Promise<TestConnectionResult> {
  try {
    return await invoke<TestConnectionResult>('test_connection', {
      provider: provider || 'gemini',
      apiKey: apiKey || '',
      model: model || '',
    });
  } catch (e) {
    return {
      success: false,
      message: String(e),
      latency_ms: 0,
      provider: provider || '',
    };
  }
}

export async function saveConfig(
  key: string,
  value: string,
): Promise<{ success: boolean; saved_path?: string; error?: string }> {
  try {
    const result = await invoke<{ success: boolean; saved_path?: string; error?: string }>('save_config', {
      key,
      value,
    });
    return result;
  } catch (e) {
    return { success: false, error: String(e) };
  }
}

export async function saveConfigBatch(
  pairs: Array<{ key: string; value: string }>,
): Promise<{ success: boolean; saved: number; failed: number; errors: string[]; saved_path?: string; error?: string }> {
  try {
    const result = await invoke<{ success: boolean; saved: number; failed: number; errors: string[]; saved_path?: string; error?: string }>('save_config_batch', {
      pairs,
    });
    return result;
  } catch (e) {
    return {
      success: false,
      saved: 0,
      failed: pairs.length,
      errors: [String(e)],
    };
  }
}