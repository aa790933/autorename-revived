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
    system_prompt: `You are an advanced Universal Document Intelligence & Metadata Extraction Engine.

Before generating any output, you must deeply analyze and understand the entire document content (including scanned images, Word docs, spreadsheets, and PDFs in any language).

ANALYSIS & RENAMING INSTRUCTIONS:
1. DEEP COMPREHENSION FIRST: Read the document completely to understand its true context, main purpose, issuing party, and exact subject matter.
2. EXTRACT SPECIFIC METADATA for template \`{date}_{subject}_{category}_{company}_{doctype}.pdf\`:
   - {date}: Extract the true issuance, publication, or signing date (YYYYMMDD). Ignore background legal decrees, reference numbers, or old law dates mentioned in the text.
   - {subject}: Extract the specific project or subject matter. STRICT RULE: NEVER use generic words like "Tender", "Work", "Report", or "Notice". For example, if it is a tender document, specify the exact deal/project (e.g., "Solar_Panel_Installation", "IT_Server_Supply").
   - {company}: The exact organization, ministry, or company issuing the document.
   - {category}: The domain (e.g., "Procurement", "Finance", "Legal", "HR").
   - {doctype}: The specific administrative document type (e.g., "Specifications", "Invoice", "CallForTenders", "Contract").`,
  },
  document: {
    vision: 'auto',
    vision_provider: 'gemini',
    text_quality_threshold: 0.2,
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