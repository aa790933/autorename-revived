/**
 * Tauri IPC wrappers
 */
import { invoke } from '@tauri-apps/api/core';
import type {
  FileResult,
  BatchResult,
  UndoResult,
  UndoFileResult,
  TestConnectionResult,
  ConfigValidation,
  ConfigValidationIssue,
  ConfigBatchResult,
  ConfigUpdate,
  RenameOptions,
  AppConfig,
  AiConfig,
  DocumentConfig,
  NamingConfig,
  UndoConfig,
} from '@/types';

export type {
  FileResult,
  BatchResult,
  UndoResult,
  UndoFileResult,
  TestConnectionResult,
  ConfigValidation,
  ConfigValidationIssue,
  ConfigBatchResult,
  ConfigUpdate,
  RenameOptions,
  AppConfig,
  AiConfig,
  DocumentConfig,
  NamingConfig,
  UndoConfig,
};

/**
 * Run the rename pipeline
 */
export async function renameFiles(
  paths: string[],
  options: RenameOptions = {}
): Promise<BatchResult> {
  return invoke<BatchResult>('rename_files', { paths, options });
}

/**
 * Ask the running batch to stop
 */
export async function cancelRename(): Promise<boolean> {
  return invoke<boolean>('cancel_rename');
}

/**
 * Undo a batch, or the most recent un-undone batch
 */
export async function undoRename(batchId?: string): Promise<UndoResult> {
  return invoke<UndoResult>('undo_rename', { batchId: batchId ?? null });
}

/**
 * Get the persisted config from backend
 */
export async function getConfig(): Promise<AppConfig> {
  return invoke<AppConfig>('get_config');
}

/**
 * Get absolute path of the settings file
 */
export async function getConfigPath(): Promise<string> {
  return invoke<string>('get_config_path');
}

/**
 * Get directory holding the undo log (resolved by backend)
 */
export async function getUndoLogDir(): Promise<string> {
  return invoke<string>('get_undo_log_path');
}

/**
 * Validate current configuration
 */
export async function validateConfig(): Promise<ConfigValidation> {
  return invoke<ConfigValidation>('validate_config');
}

/**
 * Test API connection to a provider
 */
export async function testApiConnection(
  provider: string,
  apiKey: string,
  model: string
): Promise<TestConnectionResult> {
  return invoke<TestConnectionResult>('test_connection', { provider, apiKey, model });
}

/**
 * Persist a set of section.field updates
 */
export async function saveConfigBatch(updates: ConfigUpdate[]): Promise<ConfigBatchResult> {
  return invoke<ConfigBatchResult>('save_app_config_batch', { updates });
}

/**
 * Get app version
 */
export async function getVersion(): Promise<string> {
  return invoke<string>('get_version');
}

/**
 * Check if running in portable mode
 */
export async function isPortableApp(): Promise<boolean> {
  return invoke<boolean>('is_portable_app');
}

/**
 * Get supported file extensions from backend
 */
export async function getSupportedExtensions(): Promise<string[]> {
  return invoke<string[]>('get_supported_extensions_list');
}

/**
 * Load app config (alias for getConfig for clarity)
 */
export async function loadAppConfig(): Promise<AppConfig> {
  return invoke<AppConfig>('load_app_config');
}

/**
 * Reload config from backend (force re-read)
 */
export async function reloadConfig(): Promise<AppConfig> {
  return invoke<AppConfig>('load_app_config');
}

/**
 * Save app config (full config)
 */
export async function saveAppConfig(config: AppConfig): Promise<void> {
  return invoke<void>('save_app_config', { config });
}

/**
 * Get supported extensions list (alias)
 */
export async function getSupportedExtensionsList(): Promise<string[]> {
  return invoke<string[]>('get_supported_extensions_list');
}