import { setState } from '../lib/state';
import { loadConfig, saveConfig, type ConfigData } from '../lib/config-store';
import { testApiConnection, saveConfigBatch } from '../lib/sidecar';
import { showToast } from '../lib/toast';
import { escapeHtml } from '../lib/utils';

type FieldType = 'string' | 'secret' | 'number' | 'toggle' | 'auto-or-bool';

interface FieldDef {
  key: string;
  label: string;
  type: FieldType;
  hint?: string;
  configKey: string;
}

const PROVIDER_DEFS: Record<string, { label: string; icon: string; fields: FieldDef[] }> = {
  gemini: {
    label: 'Google Gemini',
    icon: '<svg viewBox="0 0 24 24" fill="currentColor" class="w-4 h-4"><path d="M12 2L2 19.5h20L12 2zm0 4l6.5 11.5h-13L12 6z"/></svg>',
    fields: [
      { key: 'api_key', label: 'API Key', type: 'secret', configKey: 'ai' },
      { key: 'gemini_model', label: 'Text Model', type: 'string', hint: 'e.g. gemini-2.0-flash', configKey: 'ai' },
      { key: 'gemini_base_url', label: 'Base URL (optional)', type: 'string', configKey: 'ai' },
    ],
  },
  openai: {
    label: 'OpenAI',
    icon: '<svg viewBox="0 0 24 24" fill="currentColor" class="w-4 h-4"><circle cx="12" cy="12" r="10"/></svg>',
    fields: [
      { key: 'api_key', label: 'API Key', type: 'secret', configKey: 'ai' },
      { key: 'model', label: 'Model', type: 'string', hint: 'e.g. gpt-4o-mini', configKey: 'ai' },
    ],
  },
  custom: {
    label: 'Custom / Ollama / vLLM',
    icon: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="w-4 h-4"><path d="M12 2v20M2 12h20"/></svg>',
    fields: [
      { key: 'api_key', label: 'API Key', type: 'secret', configKey: 'ai' },
      { key: 'custom_model', label: 'Model', type: 'string', hint: 'e.g. llama3.2, qwen-72b', configKey: 'ai' },
      { key: 'custom_base_url', label: 'Base URL', type: 'string', hint: 'e.g. http://localhost:11434/v1', configKey: 'ai' },
    ],
  },
};

const COMMON_AI_FIELDS: FieldDef[] = [
  { key: 'temperature', label: 'Temperature', type: 'number', hint: '0.0 = deterministic, 1.0 = creative', configKey: 'ai' },
  { key: 'timeout', label: 'Timeout (seconds)', type: 'number', configKey: 'ai' },
];

const DOCUMENT_FIELDS: FieldDef[] = [
  { key: 'vision', label: 'Vision (scanned docs)', type: 'auto-or-bool', hint: 'auto = use AI vision for scanned pages', configKey: 'pdf' },
];

const NAMING_FIELDS: FieldDef[] = [
  { key: 'template', label: 'Filename Template', type: 'string', hint: '{date}, {company}, {doctype}, {category}, {subject}, {original}, {sequence}', configKey: 'naming' },
  { key: 'fallback', label: 'Fallback Template', type: 'string', hint: 'Used when template yields empty name', configKey: 'naming' },
  { key: 'date_format', label: 'Date Format', type: 'string', hint: 'strftime format, e.g. %Y%m%d', configKey: 'naming' },
  { key: 'separator', label: 'Separator', type: 'string', configKey: 'naming' },
  { key: 'max_length', label: 'Max Filename Length', type: 'number', configKey: 'naming' },
  { key: 'sequence_zerofill', label: 'Sequence Zero-Fill', type: 'number', hint: 'Pad sequence numbers to this width', configKey: 'naming' },
];

const UNDO_FIELDS: FieldDef[] = [
  { key: 'enabled', label: 'Enable Undo', type: 'toggle', configKey: 'undo' },
  { key: 'log_path', label: 'Log Path', type: 'string', hint: 'Path to rename history log', configKey: 'undo' },
  { key: 'max_entries', label: 'Max Entries', type: 'number', configKey: 'undo' },
];

const GENERAL_FIELDS: FieldDef[] = [
  { key: 'debug', label: 'Debug Mode', type: 'toggle', configKey: '_general' },
  { key: 'max_workers', label: 'Max Workers', type: 'number', hint: 'Parallel rename threads', configKey: '_general' },
];

function getNested(obj: Record<string, unknown>, dottedKey: string): unknown {
  return dottedKey.split('.').reduce<unknown>((acc, part) => {
    if (acc && typeof acc === 'object') return (acc as Record<string, unknown>)[part];
    return undefined;
  }, obj);
}

function autoOrBoolToString(val: unknown): string {
  if (val === 'auto') return 'auto';
  return Boolean(val) ? 'true' : 'false';
}

function stringToAutoOrBool(val: string): string {
  if (val === 'auto' || val === '' || val === undefined) return val;
  return val.toLowerCase();
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
      const opts = ['auto', 'true', 'false']
        .map((v) => `<option value="${v}" ${v === autoOrBoolToString(value) ? 'selected' : ''}>${v}</option>`)
        .join('');
      return `<select id="${id}" class="input input-sm">${opts}</select>`;
    }
    case 'number':
      return `<input id="${id}" type="number" class="input input-sm" value="${escapeHtml(strVal)}">`;
    case 'toggle': {
      const checked = Boolean(value);
      return `<input id="${id}" type="checkbox" class="checkbox toggle-checkbox" ${checked ? 'checked' : ''}>`;
    }
    case 'string':
    default:
      return `<input id="${id}" type="text" class="input input-sm" value="${escapeHtml(strVal)}" autocomplete="off">`;
  }
}

function renderFieldRow(def: FieldDef, value: unknown, fullKey: string): string {
  const hintHtml = def.hint ? `<div class="form-hint">${escapeHtml(def.hint)}</div>` : '';
  return `
    <div class="settings-field">
      <label class="label">${escapeHtml(def.label)}</label>
      ${renderInput(def, value, fullKey)}
      ${hintHtml}
    </div>`;
}

function renderProviderFields(config: ConfigData): string {
  const provider = config.ai.provider || 'openai';
  const def = PROVIDER_DEFS[provider] || PROVIDER_DEFS.openai;
  const fields = [...def.fields, ...COMMON_AI_FIELDS];
  return fields.map((f) => {
    const val = f.configKey === '_general'
      ? (config as unknown as Record<string, unknown>)[f.key]
      : getNested((config[f.configKey as keyof ConfigData] || {}) as Record<string, unknown>, f.key);
    return renderFieldRow(f, val, `${f.configKey}.${f.key}`);
  }).join('');
}

function renderSection(title: string, fields: FieldDef[], config: ConfigData): string {
  const rows = fields.map((f) => {
    const val = f.configKey === '_general'
      ? (config as unknown as Record<string, unknown>)[f.key]
      : getNested((config[f.configKey as keyof ConfigData] || {}) as Record<string, unknown>, f.key);
    return renderFieldRow(f, val, `${f.configKey}.${f.key}`);
  }).join('');
  if (!rows) return '';
  return `
    <div class="settings-section">
      <h3>${escapeHtml(title)}</h3>
      <div class="card card-bordered">${rows}</div>
    </div>`;
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

  document.getElementById('btn-test-connection')?.addEventListener('click', async () => {
    const btn = document.getElementById('btn-test-connection') as HTMLButtonElement | null;
    if (btn) { btn.disabled = true; btn.textContent = 'Testing...'; }
    try {
      const providerEl = document.getElementById('field-ai-provider') as HTMLSelectElement | null;
      const apiKeyEl = document.getElementById('field-ai-api_key') as HTMLInputElement | null;
      const modelEl = document.getElementById('field-ai-model') as HTMLInputElement | null;
      const provider = providerEl?.value || '';
      const apiKey = apiKeyEl?.value || '';
      const model = modelEl?.value || '';
      const result = await testApiConnection(provider || undefined, apiKey || undefined, model || undefined);
      showToast(`${result.provider}: ${result.message}`, result.success ? 'success' : 'danger');
    } catch (err) {
      showToast(`Connection test failed: ${err}`, 'danger');
    } finally {
      if (btn) { btn.disabled = false; btn.textContent = 'Test Connection'; }
    }
  });

  document.getElementById('btn-save-settings')?.addEventListener('click', async () => {
    await saveAllSettings();
  });

  const config = await loadConfig();
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
    ${Object.entries(PROVIDER_DEFS).map(([key, def]) => {
      const active = key === current ? ' provider-btn-active' : '';
      return `<button class="provider-btn${active}" data-provider="${key}">
        <span class="provider-btn-icon">${def.icon}</span>
        <span class="provider-btn-label">${def.label}</span>
      </button>`;
    }).join('')}
  </div>`;

  bar.querySelectorAll('.provider-btn').forEach((btn) => {
    btn.addEventListener('click', () => {
      const provider = (btn as HTMLElement).dataset.provider;
      if (!provider || !currentConfig) return;
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

  const providerFieldsHtml = renderProviderFields(config);
  const docHtml = renderSection('Document Processing', DOCUMENT_FIELDS, config);
  const namingHtml = renderSection('File Naming', NAMING_FIELDS, config);
  const undoHtml = renderSection('Undo History', UNDO_FIELDS, config);
  const generalHtml = renderSection('General', GENERAL_FIELDS, config);

  contentEl.innerHTML = `
    <div class="space-y-4">
      <div class="settings-section">
        <h3>${escapeHtml(PROVIDER_DEFS[provider]?.label || provider)}</h3>
        <div class="card card-bordered">${providerFieldsHtml}</div>
      </div>
      ${docHtml}
      ${namingHtml}
      ${undoHtml}
      ${generalHtml}
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

async function saveAllSettings(): Promise<void> {
  if (!currentConfig) {
    showToast('No configuration loaded', 'warning');
    return;
  }

  const updates: Array<{ key: string; value: string }> = [];

  const allFields: FieldDef[] = [
    ...Object.values(PROVIDER_DEFS).flatMap((d) => d.fields),
    ...COMMON_AI_FIELDS,
    ...DOCUMENT_FIELDS,
    ...NAMING_FIELDS,
    ...UNDO_FIELDS,
    ...GENERAL_FIELDS,
  ];

  const seen = new Set<string>();
  for (const field of allFields) {
    const fullKey = `${field.configKey}.${field.key}`;
    if (seen.has(fullKey)) continue;
    seen.add(fullKey);

    const id = `field-${fullKey.replace(/\./g, '-')}`;
    const el = document.getElementById(id);
    if (!el) continue;

    let rawValue: string;
    if (el instanceof HTMLInputElement && el.type === 'checkbox') {
      rawValue = el.checked ? 'true' : 'false';
    } else if (el instanceof HTMLInputElement || el instanceof HTMLSelectElement) {
      rawValue = el.value;
    } else {
      continue;
    }

    let oldValue: string;
    if (field.configKey === '_general') {
      oldValue = String((currentConfig as unknown as Record<string, unknown>)[field.key] ?? '');
    } else {
      const sectionData = (currentConfig[field.configKey as keyof ConfigData] || {}) as Record<string, unknown>;
      oldValue = String(getNested(sectionData, field.key) ?? '');
    }

    const normalizedOld = field.type === 'auto-or-bool' ? autoOrBoolToString(oldValue) : oldValue;
    const normalizedNew = field.type === 'auto-or-bool' ? stringToAutoOrBool(rawValue) : rawValue;

    if (normalizedOld !== normalizedNew) {
      updates.push({ key: fullKey, value: normalizedNew });

      if (field.configKey === '_general') {
        (currentConfig as unknown as Record<string, unknown>)[field.key] =
          field.type === 'toggle' ? normalizedNew === 'true' :
          field.type === 'number' ? Number(normalizedNew) : normalizedNew;
      } else {
        const section = currentConfig[field.configKey as keyof ConfigData] as Record<string, unknown>;
        if (section) {
          section[field.key] =
            field.type === 'toggle' ? normalizedNew === 'true' :
            field.type === 'number' ? Number(normalizedNew) : normalizedNew;
        }
      }
    }
  }

  updates.push({ key: 'ai.provider', value: currentConfig.ai.provider });

  if (!updates.length) {
    showToast('No changes to save', 'info');
    return;
  }

  try {
    await saveConfig(currentConfig);

    const result = await saveConfigBatch(updates);
    if (result.failed > 0 && result.saved > 0) {
      showToast(`${result.saved} saved, ${result.failed} failed`, 'warning');
    } else if (result.failed > 0) {
      showToast(`Save failed: ${result.errors?.[0] || 'Unknown error'}`, 'danger');
    } else {
      showToast(`${result.saved} settings saved`, 'success');
    }

    renderProviderBar(currentConfig);
    renderContent(currentConfig);
    bindPasswordToggles();
  } catch (e) {
    showToast(`Save failed: ${e}`, 'danger');
  }
}

export function destroySettingsView(): void {
  currentConfig = null;
}
