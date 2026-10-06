/**
 * Shared type definitions for AutoRename-Revived v4.0.0
 * Mirror the Rust backend types for type safety
 */

export interface FileResult {
  file: string;
  status: 'completed' | 'skipped' | 'failed';
  new_name: string | null;
  new_path: string | null;
  error: string | null;
  warnings: string[];
  company: string | null;
  date: string | null;
  doc_type: string | null;
  provider: string | null;
  model: string | null;
  suggestion_names: string[];
  suggestion_languages: string[];
}

export interface BatchResult {
  success: boolean;
  total: number;
  completed: number;
  skipped: number;
  failed: number;
  files: FileResult[];
  dry_run: boolean;
  batch_id: string | null;
}

export interface UndoFileResult {
  old_path: string;
  new_path: string;
  status: 'restored' | 'failed';
  error?: string | null;
}

export interface UndoResult {
  success: boolean;
  restored: number;
  failed: number;
  files: UndoFileResult[];
  batch_id: string | null;
}

export interface TestConnectionResult {
  success: boolean;
  message: string;
  latency_ms: number;
  provider: string;
}

export interface ConfigValidationIssue {
  field: string;
  level: 'error' | 'warning';
  message: string;
}

export interface ConfigValidation {
  valid: boolean;
  issues: ConfigValidationIssue[];
}

export interface ConfigBatchResult {
  success: boolean;
  saved: number;
  failed: number;
  errors: string[];
}

export interface ConfigUpdate {
  key: string;
  value: string;
}

export interface RenameOptions {
  dryRun?: boolean;
  provider?: string;
}

export interface AppConfig {
  ai: AiConfig;
  document: DocumentConfig;
  naming: NamingConfig;
  undo: UndoConfig;
  backup: BackupConfig;
  debug: boolean;
  max_workers: number;
  harmonized_companies: Record<string, unknown>[];
}

export interface BackupConfig {
  keep: number;
  dir: string;
}

export interface AiConfig {
  provider: string;
  api_key: string;
  model: string;
  gemini_model: string;
  base_url: string;
  gemini_base_url: string;
  custom_model: string;
  custom_base_url: string;
  ollama_base_url: string;
  temperature: number;
  timeout: number;
  system_prompt: string;
}

export interface DocumentConfig {
  vision: string;
  vision_provider: string;
  text_quality_threshold: number;
}

export interface NamingConfig {
  template: string;
  fallback: string;
  date_format: string;
  separator: string;
  max_length: number;
  sequence_zerofill: number;
  primary_language: string;
  suggestion_languages: string[];
}

export interface UndoConfig {
  enabled: boolean;
  log_path: string;
  max_entries: number;
}

export type AppView = 'files' | 'settings' | 'history' | 'about';

export interface FileEntry {
  id: string;
  path: string;
  name: string;
  status: 'pending' | 'processing' | 'completed' | 'skipped' | 'failed';
  result?: FileResult;
}