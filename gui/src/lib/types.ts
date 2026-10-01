/**
 * Shared type definitions for the AutoRename-Revived frontend.
 *
 * These mirror the Rust structs in `src-tauri/src/document.rs` and
 * `src-tauri/src/config.rs`, and are the single source of truth for frontend
 * types. Import from here instead of re-declaring shapes in `sidecar.ts` or
 * `config-store.ts`.
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

/** A single `section.field` config write. */
export interface ConfigUpdate {
  key: string;
  value: string;
}
