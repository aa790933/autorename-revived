import { invoke } from '@tauri-apps/api/core';
import type { SettingsState } from './state';

export async function saveSettings(settings: SettingsState): Promise<void> {
  await invoke('save_settings', {
    settings: {
      provider: settings.provider,
      apiKey: settings.apiKey,
      model: settings.model,
      customBaseUrl: settings.customBaseUrl,
      namingPattern: settings.namingPattern,
    },
  });
}

export async function getSettings(): Promise<SettingsState> {
  return await invoke('get_settings');
}

export async function testConnection(): Promise<boolean> {
  return await invoke('test_connection');
}