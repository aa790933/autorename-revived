/**
 * In-memory cache of the persisted configuration.
 *
 * Writes always go through `saveConfigBatch` in `sidecar.ts`, which applies
 * `section.field` updates to the *raw* on-disk config so `${ENV_VAR}`
 * references survive a save. There is intentionally no "write the whole
 * object back" path: the config the frontend reads has its environment
 * placeholders already resolved, and persisting that would store the secret
 * in plaintext.
 */
import { invoke } from '@tauri-apps/api/core';

export interface ConfigData {
  ai: {
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

/**
 * Mirrors `AppConfig::default()` in `src-tauri/src/config.rs`.
 *
 * Only used when the backend is unreachable or the settings file is missing;
 * the backend remains the source of truth for every default.
 */
const DEFAULT_CONFIG: ConfigData = {
  ai: {
    provider: 'gemini',
    api_key: '',
    model: 'gpt-4o-mini',
    gemini_model: 'gemini-2.0-flash',
    base_url: '',
    gemini_base_url: '',
    custom_model: '',
    custom_base_url: '',
    ollama_base_url: '',
    temperature: 0,
    timeout: 30,
    system_prompt: '',
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
    log_path: '',
    max_entries: 100,
  },
  debug: false,
  max_workers: 4,
  harmonized_companies: [],
};

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

/**
 * Deep-merge a loaded config over the defaults.
 *
 * A shallow spread left whole sections undefined whenever the persisted file
 * predates a section (or a single key), which made the settings form render
 * `undefined`.
 */
function mergeConfig(base: ConfigData, loaded: unknown): ConfigData {
  if (!isPlainObject(loaded)) return structuredClone(base);

  const merge = (target: Record<string, unknown>, source: Record<string, unknown>) => {
    for (const [key, value] of Object.entries(source)) {
      if (value === undefined) continue;
      const current = target[key];
      if (isPlainObject(value) && isPlainObject(current)) {
        merge(current, value);
      } else {
        target[key] = value;
      }
    }
    return target;
  };

  return merge(
    structuredClone(base) as unknown as Record<string, unknown>,
    loaded,
  ) as unknown as ConfigData;
}

let _config: ConfigData = structuredClone(DEFAULT_CONFIG);
let _loaded = false;

export async function loadConfig(): Promise<ConfigData> {
  if (_loaded) return _config;

  try {
    const loaded = await invoke<unknown>('load_app_config');
    _config = mergeConfig(DEFAULT_CONFIG, loaded);
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
 * Force a re-read from disk. Used after a save so the form reflects exactly
 * what was persisted.
 */
export async function reloadConfig(): Promise<ConfigData> {
  _loaded = false;
  return loadConfig();
}
