import { invoke } from '@tauri-apps/api/core';

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
  document: {
    vision: string;
    vision_provider: string;
    text_quality_threshold: number;
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
    system_prompt: `Analyze this document thoroughly. Extract the following metadata fields as JSON:

- date_YYYY_MM_DD: Primary issuance/effective date in YYYY-MM-DD format. If no date exists, use empty string.
- issuer_entity_short: Short functional entity name (strip legal suffixes like 'LLC', 'Inc.'). If no issuer, use empty string.
- document_nature: Administrative/structural type (e.g., Invoice, Contract, ID_Card, Photo, Receipt, Letter, Report).
- specific_subject: 2-4 word description of the UNIQUE topic. Must NOT repeat words from document_nature or issuer_entity_short.
- is_unreadable_or_error: Set to true if the document cannot be read or understood.

Do NOT output anything except the JSON object. If you CANNOT read the document, set is_unreadable_or_error to true and provide best-effort values for the other fields.`,
  },
  document: {
    vision: 'auto',
    vision_provider: 'gemini',
    text_quality_threshold: 0.2,
  },
  naming: {
    template: '{date}_{doctype}_{company}_{subject}',
    fallback: '{date}_{doctype}_{company}_Unknown',
    date_format: '%Y-%m-%d',
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

/**
 * Force re-read config from disk. Useful when the config file may have been
 * modified externally (e.g. portable mode settings change).
 */
export async function reloadConfig(): Promise<ConfigData> {
  _loaded = false;
  return loadConfig();
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