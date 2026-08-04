import { readTextFile, writeTextFile } from '@tauri-apps/plugin-fs';
import { appDataDir } from '@tauri-apps/api/path';

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
    gemini_model: 'gemini-3.1-flash-lite',
    gemini_api_key: '',
    gemini_base_url: '',
    custom_model: '',
    custom_base_url: '',
    temperature: 0.0,
    timeout: 30,
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
let _configPath: string | null = null;
let _loaded = false;

async function getConfigPath(): Promise<string> {
  if (!_configPath) {
    const dir = await appDataDir();
    const sep = dir.includes('\\') ? '\\' : '/';
    _configPath = dir + sep + 'config.json';
  }
  return _configPath;
}

function deepMerge(base: Record<string, unknown>, override: Record<string, unknown>): void {
  for (const key of Object.keys(override)) {
    if (
      key in base &&
      typeof base[key] === 'object' && base[key] !== null &&
      typeof override[key] === 'object' && override[key] !== null &&
      !Array.isArray(base[key])
    ) {
      deepMerge(base[key] as Record<string, unknown>, override[key] as Record<string, unknown>);
    } else {
      base[key] = override[key];
    }
  }
}

export async function loadConfig(): Promise<ConfigData> {
  if (_loaded) return _config;

  try {
    const path = await getConfigPath();
    const raw = await readTextFile(path);
    const parsed = JSON.parse(raw) as Partial<ConfigData>;
    _config = structuredClone(DEFAULT_CONFIG);
    deepMerge(_config as unknown as Record<string, unknown>, parsed as Record<string, unknown>);
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
    const path = await getConfigPath();
    await writeTextFile(path, JSON.stringify(_config, null, 2));
  } catch (e) {
    console.warn('Failed to persist config to disk:', e);
  }
}

export function resetConfig(): void {
  _config = structuredClone(DEFAULT_CONFIG);
  _loaded = true;
}
