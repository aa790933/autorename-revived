import { invoke } from '@tauri-apps/api/core';

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
  suggestion_names: string[];
  suggestion_languages: string[];
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

export interface ConfigData {
  ai: {
    provider: string;
    api_key: string;
    model: string;
    gemini_model: string;
    gemini_api_key: string;
    gemini_base_url: string;
    custom_model: string;
    custom_base_url: string;
     temperature: number;
    timeout: number;
    system_prompt: string;
  };
  pdf: {
    vision: string;
    vision_provider: string;
  };
  naming: {
    template: string;
    fallback: string;
    date_format: string;
    separator: string;
    max_length: number;
    sequence_zerofill: number;
    primary_language: string;
    suggestion_languages: string[];
  };
  undo: {
    enabled: boolean;
    log_path: string;
    max_entries: number;
  };
  debug: boolean;
  max_workers: number;
  harmonized_companies: unknown[];
}

const DEFAULT_CONFIG: ConfigData = {
  ai: {
    provider: 'gemini',
    api_key: '',
    model: 'gpt-4o-mini',
    gemini_model: 'gemini-3.5-flash-lite',
    gemini_api_key: '',
    gemini_base_url: '',
    custom_model: '',
    custom_base_url: '',
     temperature: 0.0,
    timeout: 30,
    system_prompt: '',
  },
  pdf: {
    vision: 'auto',
    vision_provider: 'gemini',
  },
  naming: {
    template: '{date}_{company}_{doctype}',
    fallback: '{date}_Unknown_{doctype}',
    date_format: '%Y%m%d',
    separator: '_',
    max_length: 128,
    sequence_zerofill: 2,
    primary_language: 'English',
    suggestion_languages: [],
  },
  undo: {
    enabled: true,
    log_path: '~/.autorename-revived/rename_history.json',
    max_entries: 100,
  },
  debug: false,
  max_workers: 4,
  harmonized_companies: [],
};

let _config: ConfigData = structuredClone(DEFAULT_CONFIG);
let _loaded = false;

export async function loadConfig(): Promise<ConfigData> {
  if (_loaded) return _config;

  try {
    const loaded = await invoke<ConfigData>('load_app_config');
    _config = { ...structuredClone(DEFAULT_CONFIG), ...loaded };
  } catch {
    _config = structuredClone(DEFAULT_CONFIG);
  }

  _loaded = true;
  return _config;
}

export function getConfigSync(): ConfigData {
  return _config;
}

export async function saveConfig(config: ConfigData): Promise<void> {
  _config = structuredClone(config);
  _loaded = true;

  try {
    await invoke('save_app_config', { config: _config });
  } catch (e) {
    console.warn('Failed to persist config via Rust backend:', e);
  }
}

export function resetConfig(): void {
  _config = structuredClone(DEFAULT_CONFIG);
  _loaded = true;
}