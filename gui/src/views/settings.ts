import { setState } from '../lib/state';
import { reloadConfig, type ConfigData } from '../lib/config-store';
import { saveConfigBatch, testApiConnection } from '../lib/sidecar';
import { showToast } from '../lib/toast';
import { escapeHtml } from '../lib/utils';

type FieldType = 'string' | 'secret' | 'number' | 'toggle' | 'auto-or-bool' | 'textarea';

interface FieldDef {
  key: string;
  label: string;
  type: FieldType;
  hint?: string;
  configKey: string;
}

interface ProviderDef {
  label: string;
  icon: string;
  /**
   * Which `ai.*` field holds this provider's model name. `get_model_name` in
   * the backend reads `gemini_model` for Gemini, `model` for OpenAI/Anthropic
   * and `custom_model` for Ollama/xAI/custom, so the form must match.
   */
  modelKey: 'gemini_model' | 'model' | 'custom_model';
  /** `ai.*` fields shown for this provider, besides the common ones. */
  fields: FieldDef[];
}

const PROVIDER_DEFS: Record<string, ProviderDef> = {
  gemini: {
    label: 'Google Gemini',
    icon: '<svg viewBox="0 0 24 24" fill="currentColor" class="w-4 h-4"><path d="M12 2L2 19.5h20L12 2zm0 4l6.5 11.5h-13L12 6z"/></svg>',
    modelKey: 'gemini_model',
    fields: [
      { key: 'api_key', label: 'API Key', type: 'secret', configKey: 'ai' },
      { key: 'gemini_model', label: 'Text Model', type: 'string', hint: 'e.g. gemini-2.0-flash', configKey: 'ai' },
      { key: 'gemini_base_url', label: 'Base URL (optional)', type: 'string', configKey: 'ai' },
    ],
  },
  openai: {
    label: 'OpenAI',
    icon: '<svg viewBox="0 0 24 24" fill="currentColor" class="w-4 h-4"><circle cx="12" cy="12" r="10"/></svg>',
    modelKey: 'model',
    fields: [
      { key: 'api_key', label: 'API Key', type: 'secret', configKey: 'ai' },
      { key: 'model', label: 'Model', type: 'string', hint: 'e.g. gpt-4o-mini', configKey: 'ai' },
      { key: 'base_url', label: 'Base URL (optional)', type: 'string', configKey: 'ai' },
    ],
  },
  anthropic: {
    label: 'Anthropic',
    icon: '<svg viewBox="0 0 24 24" fill="currentColor" class="w-4 h-4"><path d="M12 3l9 18h-4.5l-1.8-3.9H9.3L7.5 21H3L12 3zm0 6.4l-2 4.3h4l-2-4.3z"/></svg>',
    modelKey: 'model',
    fields: [
      { key: 'api_key', label: 'API Key', type: 'secret', configKey: 'ai' },
      { key: 'model', label: 'Model', type: 'string', hint: 'e.g. claude-3-5-haiku-latest', configKey: 'ai' },
      { key: 'base_url', label: 'Base URL (optional)', type: 'string', configKey: 'ai' },
    ],
  },
  ollama: {
    label: 'Ollama (local)',
    icon: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="w-4 h-4"><rect x="3" y="4" width="18" height="16" rx="3"/><circle cx="12" cy="12" r="3"/></svg>',
    modelKey: 'custom_model',
    fields: [
      { key: 'custom_model', label: 'Model', type: 'string', hint: 'e.g. llama3.2, qwen2.5', configKey: 'ai' },
      { key: 'ollama_base_url', label: 'Ollama Base URL', type: 'string', hint: 'default http://localhost:11434', configKey: 'ai' },
      { key: 'api_key', label: 'API Key (optional)', type: 'secret', configKey: 'ai' },
    ],
  },
  xai: {
    label: 'xAI',
    icon: '<svg viewBox="0 0 24 24" fill="currentColor" class="w-4 h-4"><path d="M3 3h4.2l4.3 6 4.4-6H21l-6.6 8.9L21 21h-4.2l-4.4-6.1L8 21H3l6.8-9.1L3 3z"/></svg>',
    modelKey: 'custom_model',
    fields: [
      { key: 'api_key', label: 'API Key', type: 'secret', configKey: 'ai' },
      { key: 'custom_model', label: 'Model', type: 'string', hint: 'e.g. grok-3-beta', configKey: 'ai' },
      { key: 'base_url', label: 'Base URL (optional)', type: 'string', configKey: 'ai' },
    ],
  },
  custom: {
    label: 'Custom / vLLM',
    icon: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="w-4 h-4"><path d="M12 2v20M2 12h20"/></svg>',
    modelKey: 'custom_model',
    fields: [
      { key: 'api_key', label: 'API Key (optional)', type: 'secret', configKey: 'ai' },
      { key: 'custom_model', label: 'Model', type: 'string', hint: 'e.g. qwen-72b', configKey: 'ai' },
      { key: 'custom_base_url', label: 'Base URL', type: 'string', hint: 'e.g. http://localhost:8000/v1', configKey: 'ai' },
    ],
  },
};

/** Order the provider buttons appear in. */
const PROVIDER_ORDER = ['gemini', 'openai', 'anthropic', 'ollama', 'xai', 'custom'];

const COMMON_AI_FIELDS: FieldDef[] = [
  { key: 'temperature', label: 'Temperature', type: 'number', hint: '0.0 = deterministic, 1.0 = creative', configKey: 'ai' },
  { key: 'timeout', label: 'Timeout (seconds)', type: 'number', hint: '5–600; vision requests use at least 60', configKey: 'ai' },
  { key: 'system_prompt', label: 'AI System Prompt', type: 'textarea', hint: 'Leave empty to use the built-in default prompt.', configKey: 'ai' },
];

const DOCUMENT_FIELDS: FieldDef[] = [
  { key: 'vision', label: 'Vision (scanned docs)', type: 'auto-or-bool', hint: 'auto = use vision AI when local text extraction is poor', configKey: 'document' },
  { key: 'vision_provider', label: 'Vision Provider', type: 'string', hint: 'gemini, openai, anthropic, ollama, xai or custom', configKey: 'document' },
  { key: 'text_quality_threshold', label: 'Text Quality Threshold', type: 'number', hint: 'Minimum text quality (0.0–1.0) for local extraction; below this, vision AI is used', configKey: 'document' },
];

const NAMING_FIELDS: FieldDef[] = [
  { key: 'template', label: 'Filename Template', type: 'string', hint: '{date}, {company}, {doctype}, {subject}, {original}, {sequence}, {separator}', configKey: 'naming' },
  { key: 'fallback', label: 'Fallback Template', type: 'string', hint: 'Used when the template yields nothing usable', configKey: 'naming' },
  { key: 'date_format', label: 'Date Format', type: 'string', hint: 'strftime format, e.g. %Y%m%d or %Y-%m-%d', configKey: 'naming' },
  { key: 'separator', label: 'Separator', type: 'string', hint: 'Joins fields and replaces {separator}; empty means _', configKey: 'naming' },
  { key: 'max_length', label: 'Max Filename Length', type: 'number', hint: '16–255 characters, extension included', configKey: 'naming' },
  { key: 'sequence_zerofill', label: 'Sequence Zero-Fill', type: 'number', hint: '1–9 digits for the {sequence} counter', configKey: 'naming' },
];

const LANGUAGES_FIELDS: FieldDef[] = [
  { key: 'primary_language', label: 'Primary Language', type: 'string', configKey: 'naming' },
  { key: 'suggestion_languages', label: 'Suggestion Languages (comma-separated)', type: 'string', hint: 'e.g. French,Arabic — extra name suggestions in these languages', configKey: 'naming' },
];

const UNDO_FIELDS: FieldDef[] = [
  { key: 'enabled', label: 'Enable Undo', type: 'toggle', configKey: 'undo' },
  { key: 'log_path', label: 'Log Path', type: 'string', hint: 'Leave empty to keep the history next to the settings file', configKey: 'undo' },
  { key: 'max_entries', label: 'Max Batches Kept', type: 'number', hint: '1–100000', configKey: 'undo' },
];

const GENERAL_FIELDS: FieldDef[] = [
  { key: 'debug', label: 'Debug Mode', type: 'toggle', configKey: '_general' },
  { key: 'max_workers', label: 'Max Workers', type: 'number', hint: 'Parallel AI requests, 1–32', configKey: '_general' },
];

function getNested(obj: Record<string, unknown>, dottedKey: string): unknown {
  return dottedKey.split('.').reduce<unknown>((acc, part) => {
    if (acc && typeof acc === 'object') return (acc as Record<string, unknown>)[part];
    return undefined;
  }, obj);
}

function autoOrBoolToString(val: unknown): string {
  if (val === 'auto') return 'auto';
  if (val === true) return 'true';
  if (val === false) return 'false';
  return String(val ?? 'false');
}

function renderInput(def: FieldDef, value: unknown, fullKey: string): string {
  const strVal = String(value ?? '');
  const id = `field-${fullKey.replace(/\./g, '-')}`;

  switch (def.type) {
    case 'secret':
      return `
        <div class="input-group">
          <input id="${id}" type="password" class="input input-sm" value="${escapeHtml(strVal)}" autocomplete="off">
          <button type="button" class="btn-toggle-password btn btn-ghost btn-sm" aria-label="Toggle visibility">
            <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path class="toggle-eye-closed" d="M10.585 10.585a2 2 0 102.83 2.83 2 2 0 00-2.83-2.83z"/>
              <path class="toggle-eye-open" style="display:none" d="M1 12s8 6 11 6 11-6 11-6-8-6-11-6S1 12 1 12z"/>
            </svg>
          </button>
        </div>`;
    case 'auto-or-bool': {
      const current = autoOrBoolToString(value);
      const opts = ['auto', 'true', 'false']
        .map((v) => `<option value="${v}" ${v === current ? 'selected' : ''}>${v}</option>`)
        .join('');
      return `<select id="${id}" class="input input-sm">${opts}</select>`;
    }
    case 'number':
      return `<input id="${id}" type="number" class="input input-sm" value="${escapeHtml(strVal)}">`;
    case 'toggle':
      return `<input id="${id}" type="checkbox" class="checkbox toggle-checkbox" ${value ? 'checked' : ''}>`;
    case 'textarea':
      return `<textarea id="${id}" class="input input-sm" rows="6" autocomplete="off">${escapeHtml(strVal)}</textarea>`;
    case 'string':
    default:
      return `<input id="${id}" type="text" class="input input-sm" value="${escapeHtml(strVal)}" autocomplete="off">`;
  }
}

function renderFieldRow(def: FieldDef, value: unknown, fullKey: string): string {
  const hintHtml = def.hint ? `<div class="form-hint">${escapeHtml(def.hint)}</div>` : '';
  return `
    <div class="settings-field">
      <label class="label" for="field-${fullKey.replace(/\./g, '-')}">${escapeHtml(def.label)}</label>
      ${renderInput(def, value, fullKey)}
      ${hintHtml}
    </div>`;
}

function valueForField(config: ConfigData, def: FieldDef): unknown {
  if (def.configKey === '_general') {
    return (config as unknown as Record<string, unknown>)[def.key];
  }
  const section = config[def.configKey as keyof ConfigData];
  return getNested((section ?? {}) as Record<string, unknown>, def.key);
}

/** Fields rendered for the active provider, in display order. */
function activeProviderFields(config: ConfigData): FieldDef[] {
  const def = PROVIDER_DEFS[config.ai.provider] ?? PROVIDER_DEFS.gemini;
  return [...def.fields, ...COMMON_AI_FIELDS];
}

let currentConfig: ConfigData | null = null;

export async function renderSettingsView(root: HTMLElement): Promise<void> {
  root.innerHTML = `
    <div class="flex flex-col flex-1 min-h-0 p-6 pt-8">
      <div class="flex items-center justify-between mb-4">
        <h2 class="text-lg font-semibold">Settings</h2>
        <button class="btn btn-secondary btn-sm" id="btn-back-from-settings">
          <svg class="w-3.5 h-3.5 inline-block mr-1 -mt-px" viewBox="0 0 24 24" fill="none" stroke="currentColor"
               stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="15 18 9 12 15 6"/>
          </svg>Back
        </button>
      </div>
      <div id="provider-bar" class="provider-bar mb-4"></div>
      <div id="settings-content" class="flex-1 overflow-y-auto" style="overflow-x:hidden;padding-right:0.5rem">
      </div>
      <div class="flex items-center justify-center gap-2 mt-4" id="settings-footer">
        <button class="btn btn-secondary btn-sm" id="btn-save-settings">Save All</button>
        <button class="btn btn-secondary btn-sm" id="btn-test-connection">Test Connection</button>
      </div>
    </div>
  `;

  document.getElementById('btn-back-from-settings')?.addEventListener('click', () => {
    setState({ view: 'files' });
  });

  document.getElementById('btn-test-connection')?.addEventListener('click', handleTestConnection);
  document.getElementById('btn-save-settings')?.addEventListener('click', () => {
    void saveAllSettings();
  });

  const config = await reloadConfig();
  setState({ statusError: '' });
  currentConfig = config;

  renderProviderBar(config);
  renderContent(config);
  bindPasswordToggles();
}

function renderProviderBar(config: ConfigData): void {
  const bar = document.getElementById('provider-bar');
  if (!bar) return;
  const current = config.ai.provider || 'gemini';

  bar.innerHTML = `<div class="provider-bar-inner">
    ${PROVIDER_ORDER.filter((key) => key in PROVIDER_DEFS)
      .map((key) => {
        const def = PROVIDER_DEFS[key];
        const active = key === current ? ' provider-btn-active' : '';
        return `<button type="button" class="provider-btn${active}" data-provider="${key}">
          <span class="provider-btn-icon">${def.icon}</span>
          <span class="provider-btn-label">${escapeHtml(def.label)}</span>
        </button>`;
      })
      .join('')}
  </div>`;

  bar.querySelectorAll('.provider-btn').forEach((btn) => {
    btn.addEventListener('click', () => {
      const provider = (btn as HTMLElement).dataset.provider;
      if (!provider || !currentConfig) return;
      // Provider switches are local until "Save All" is pressed; unsaved edits
      // in the currently rendered fields are read back first so they are not
      // lost when the form is re-rendered.
      collectVisibleEdits(currentConfig);
      currentConfig.ai.provider = provider;
      renderProviderBar(currentConfig);
      renderContent(currentConfig);
      bindPasswordToggles();
    });
  });
}

function renderContent(config: ConfigData): void {
  const contentEl = document.getElementById('settings-content');
  if (!contentEl) return;

  const provider = config.ai.provider || 'gemini';
  const def = PROVIDER_DEFS[provider] ?? PROVIDER_DEFS.gemini;

  const providerFieldsHtml = activeProviderFields(config)
    .map((f) => renderFieldRow(f, valueForField(config, f), `${f.configKey}.${f.key}`))
    .join('');

  const section = (title: string, fields: FieldDef[]): string => {
    const rows = fields
      .map((f) => renderFieldRow(f, valueForField(config, f), `${f.configKey}.${f.key}`))
      .join('');
    if (!rows) return '';
    return `
      <div class="settings-section">
        <h3>${escapeHtml(title)}</h3>
        <div class="card card-bordered">${rows}</div>
      </div>`;
  };

  contentEl.innerHTML = `
    <div class="space-y-4">
      <div class="settings-section">
        <h3>${escapeHtml(def.label)}</h3>
        <div class="card card-bordered">${providerFieldsHtml}</div>
      </div>
      ${section('Document Processing', DOCUMENT_FIELDS)}
      ${section('File Naming', NAMING_FIELDS)}
      ${section('AI Languages', LANGUAGES_FIELDS)}
      ${section('Undo History', UNDO_FIELDS)}
      ${section('General', GENERAL_FIELDS)}
    </div>
  `;
}

function bindPasswordToggles(): void {
  document.querySelectorAll('.btn-toggle-password').forEach((btn) => {
    btn.addEventListener('click', () => {
      const input = btn.closest('.input-group')?.querySelector('input') as HTMLInputElement | null;
      if (!input) return;
      input.type = input.type === 'password' ? 'text' : 'password';
    });
  });
}

/** Read the current DOM value of one field, or `undefined` if not rendered. */
function readFieldValue(def: FieldDef): string | undefined {
  const id = `field-${def.configKey}.${def.key}`.replace(/\./g, '-');
  const el = document.getElementById(id);
  if (!el) return undefined;

  if (el instanceof HTMLInputElement && el.type === 'checkbox') {
    return el.checked ? 'true' : 'false';
  }
  if (
    el instanceof HTMLInputElement ||
    el instanceof HTMLSelectElement ||
    el instanceof HTMLTextAreaElement
  ) {
    return el.value;
  }
  return undefined;
}

/** Current value of a field inside the in-memory config, as a string. */
function configValueString(config: ConfigData, def: FieldDef): string {
  const raw = valueForField(config, def);
  if (def.type === 'auto-or-bool') return autoOrBoolToString(raw);
  if (def.type === 'toggle') return raw ? 'true' : 'false';
  if (Array.isArray(raw)) return raw.join(',');
  return String(raw ?? '');
}

/**
 * Copy the values currently in the form into `config`.
 *
 * Only the *rendered* fields are read, which is exactly the set the user can
 * have edited. Returns the `section.field` updates that differ from what was
 * loaded, ready to hand to `save_app_config_batch`.
 */
function collectVisibleEdits(config: ConfigData): Array<{ key: string; value: string }> {
  const updates: Array<{ key: string; value: string }> = [];
  const seen = new Set<string>();

  for (const field of activeProviderFields(config)) {
    const fullKey = `${field.configKey}.${field.key}`;
    if (seen.has(fullKey)) continue;
    seen.add(fullKey);

    const rawValue = readFieldValue(field);
    if (rawValue === undefined) continue;

    // `auto-or-bool` selects already emit 'auto' | 'true' | 'false'.
    if (rawValue !== configValueString(config, field)) {
      updates.push({ key: fullKey, value: rawValue });
    }
  }

  for (const field of [
    ...DOCUMENT_FIELDS,
    ...NAMING_FIELDS,
    ...LANGUAGES_FIELDS,
    ...UNDO_FIELDS,
    ...GENERAL_FIELDS,
  ]) {
    const fullKey = `${field.configKey}.${field.key}`;
    if (seen.has(fullKey)) continue;
    seen.add(fullKey);

    const rawValue = readFieldValue(field);
    if (rawValue === undefined) continue;
    if (rawValue !== configValueString(config, field)) {
      updates.push({ key: fullKey, value: rawValue });
    }
  }

  return updates;
}

async function saveAllSettings(): Promise<void> {
  if (!currentConfig) {
    showToast('No configuration loaded', 'warning');
    return;
  }

  const updates = collectVisibleEdits(currentConfig);
  // The provider itself is not a form field, so it is always written.
  updates.push({ key: 'ai.provider', value: currentConfig.ai.provider });

  const saveBtn = document.getElementById('btn-save-settings') as HTMLButtonElement | null;
  if (saveBtn) {
    saveBtn.disabled = true;
    saveBtn.textContent = 'Saving\u2026';
  }

  try {
    const result = await saveConfigBatch(updates);

    if (result.failed > 0 && result.saved > 0) {
      showToast(`${result.saved} saved, ${result.failed} failed: ${result.errors[0] ?? ''}`, 'warning');
    } else if (result.failed > 0) {
      showToast(`Save failed: ${result.errors[0] || 'Unknown error'}`, 'danger');
    } else {
      showToast(`${result.saved} settings saved`, 'success');
    }

    // Re-read from disk so the form shows exactly what was persisted
    // (including any value the backend normalised or rejected).
    const refreshed = await reloadConfig();
    currentConfig = refreshed;
    renderProviderBar(refreshed);
    renderContent(refreshed);
    bindPasswordToggles();
  } catch (e) {
    showToast(`Save failed: ${e}`, 'danger');
  } finally {
    if (saveBtn) {
      saveBtn.disabled = false;
      saveBtn.textContent = 'Save All';
    }
  }
}

async function handleTestConnection(): Promise<void> {
  const btn = document.getElementById('btn-test-connection') as HTMLButtonElement | null;
  if (btn) {
    btn.disabled = true;
    btn.textContent = 'Testing\u2026';
  }

  try {
    const config = currentConfig;
    const provider = config?.ai.provider ?? 'gemini';
    const apiKey = readFieldValue({ key: 'api_key', label: '', type: 'secret', configKey: 'ai' })
      ?? config?.ai.api_key
      ?? '';
    const modelKey = (PROVIDER_DEFS[provider] ?? PROVIDER_DEFS.gemini).modelKey;
    const model = readFieldValue({ key: modelKey, label: '', type: 'string', configKey: 'ai' })
      ?? config?.ai[modelKey]
      ?? '';

    const result = await testApiConnection(provider, apiKey, model);
    showToast(`${result.provider}: ${result.message}`, result.success ? 'success' : 'danger');
  } catch (err) {
    showToast(`Connection test failed: ${err}`, 'danger');
  } finally {
    if (btn) {
      btn.disabled = false;
      btn.textContent = 'Test Connection';
    }
  }
}

export function destroySettingsView(): void {
  currentConfig = null;
}
