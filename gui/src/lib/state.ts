export interface SettingsState {
  provider: 'gemini' | 'openai' | 'custom';
  apiKey: string;
  model: string;
  customBaseUrl: string;
  namingPattern: string;
  testStatus: 'idle' | 'testing' | 'success' | 'error';
  testMessage: string;
  saveStatus: 'idle' | 'saving' | 'success' | 'error';
  saveMessage: string;
}

export function defaultSettings(): SettingsState {
  return {
    provider: 'gemini',
    apiKey: '',
    model: 'gemini-3.1-flash-lite',
    customBaseUrl: '',
    namingPattern: '{date}_{company}_{doctype}',
    testStatus: 'idle',
    testMessage: '',
    saveStatus: 'idle',
    saveMessage: '',
  };
}

type Listener = () => void;

let state = defaultSettings();
const listeners = new Set<Listener>();

export function initState(): void {
  state = defaultSettings();
}

export function getState(): SettingsState {
  return state;
}

export function setState(partial: Partial<SettingsState>): void {
  state = { ...state, ...partial };
  listeners.forEach((l) => l());
}

export function subscribe(listener: Listener): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}