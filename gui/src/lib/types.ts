/**
 * Shared type definitions for the AutoRename-Revived frontend.
 *
 * These types mirror the Rust structs in `src-tauri/src/document.rs` and are
 * the single source of truth for frontend type definitions. Import from here
 * instead of duplicating in `sidecar.ts` or `config-store.ts`.
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
