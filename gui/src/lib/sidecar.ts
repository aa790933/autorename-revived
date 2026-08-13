import { invoke } from '@tauri-apps/api/core';
import { appDataDir } from '@tauri-apps/api/path';
import type { BatchResult, ErrorResult, SidecarResult, UndoResult, FileResult } from './types';

export type { BatchResult, ErrorResult, SidecarResult, UndoResult, FileResult };

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

export async function renameFiles(
  paths: string[],
  options: {
    dryRun?: boolean;
    recursive?: boolean;
    provider?: string;
    model?: string;
    vision?: boolean;
    textOnly?: boolean;
  } = {},
): Promise<SidecarResult> {
  try {
    const result = await invoke<BatchResult>('rename_files', {
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

export function cancelRename(): Promise<boolean> {
  return invoke<boolean>('cancel_rename');
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