import { getState, setState, SettingsState, subscribe } from '../lib/state';
import { saveSettings, getSettings, testConnection } from '../lib/invoke';
import { SUPPORTED_EXTENSIONS } from '../lib/utils';

export function renderSettingsView(container: HTMLElement, state: SettingsState): void {
  container.innerHTML = `
    <div class="settings-container">
      <header class="settings-header">
        <h1>AutoRename Revived</h1>
        <div class="header-actions">
          <button id="btn-save" class="btn btn-primary" title="Save settings">Save</button>
          <button id="btn-test" class="btn btn-secondary" title="Test connection">Test Connection</button>
        </div>
      </header>

      <main class="settings-body">
        <section class="card">
          <h2>AI Provider</h2>
          <div class="field">
            <label for="provider">Provider</label>
            <select id="provider">
              <option value="gemini">Google Gemini</option>
              <option value="openai">OpenAI</option>
              <option value="custom">Custom Endpoints (Ollama/vLLM)</option>
            </select>
          </div>

          <div class="field" id="field-model">
            <label for="model">Model</label>
            <input type="text" id="model" placeholder="gemini-3.1-flash-lite" />
          </div>

          <div class="field" id="field-api-key">
            <label for="api-key">API Key</label>
            <input type="password" id="api-key" placeholder="Enter your API key" />
          </div>

          <div class="field" id="field-custom-url" style="display:none;">
            <label for="custom-base-url">Base URL</label>
            <input type="url" id="custom-base-url" placeholder="http://localhost:11434" />
          </div>
        </section>

        <section class="card">
          <h2>Naming Pattern</h2>
          <div class="field">
            <label for="naming-pattern">Pattern</label>
            <input type="text" id="naming-pattern" placeholder="{date}_{company}_{doctype}" />
          </div>
          <p class="hint">Available variables: {date}, {company}, {doctype}</p>
        </section>

        <section class="card" id="connection-status">
          <h2>Connection Status</h2>
          <div class="status-row">
            <span class="status-dot" id="status-dot"></span>
            <span id="status-text">Not tested</span>
          </div>
        </section>

        <section class="card">
          <h2>Supported File Types</h2>
          <div class="file-types">${Array.from(SUPPORTED_EXTENSIONS)
            .sort()
            .map((ext) => `<span class="tag">${ext}</span>`)
            .join('')}</div>
        </section>
      </main>
    </div>
  `;

  bindEvents(container, state);
  syncFormFromState(container, state);
}

function bindEvents(container: HTMLElement, state: SettingsState): void {
  const providerSelect = container.querySelector<HTMLSelectElement>('#provider');
  const modelInput = container.querySelector<HTMLInputElement>('#model');
  const apiKeyInput = container.querySelector<HTMLInputElement>('#api-key');
  const customUrlInput = container.querySelector<HTMLInputElement>('#custom-base-url');
  const namingPatternInput = container.querySelector<HTMLInputElement>('#naming-pattern');
  const btnSave = container.querySelector<HTMLButtonElement>('#btn-save');
  const btnTest = container.querySelector<HTMLButtonElement>('#btn-test');

  providerSelect?.addEventListener('change', () => {
    const provider = providerSelect.value;
    setState({ provider: provider as SettingsState['provider'] });
    updateProviderFields(provider);
  });

  modelInput?.addEventListener('input', () => {
    setState({ model: modelInput.value });
  });

  apiKeyInput?.addEventListener('input', () => {
    setState({ apiKey: apiKeyInput.value });
  });

  customUrlInput?.addEventListener('input', () => {
    setState({ customBaseUrl: customUrlInput.value });
  });

  namingPatternInput?.addEventListener('input', () => {
    setState({ namingPattern: namingPatternInput.value });
  });

  btnSave?.addEventListener('click', async () => {
    setState({ saveStatus: 'saving', saveMessage: 'Saving...' });
    try {
      await saveSettings(getState());
      setState({ saveStatus: 'success', saveMessage: 'Settings saved' });
    } catch (e) {
      setState({ saveStatus: 'error', saveMessage: (e as Error).message });
    }
    setTimeout(() => setState({ saveStatus: 'idle', saveMessage: '' }), 3000);
  });

  btnTest?.addEventListener('click', async () => {
    setState({ testStatus: 'testing', testMessage: 'Testing connection...' });
    try {
      const result = await testConnection();
      setState({
        testStatus: result ? 'success' : 'error',
        testMessage: result ? 'Connection successful' : 'Connection failed',
      });
    } catch (e) {
      setState({ testStatus: 'error', testMessage: (e as Error).message });
    }
    setTimeout(() => setState({ testStatus: 'idle', testMessage: '' }), 5000);
  });
}

function updateProviderFields(provider: string): void {
  const customUrlField = document.getElementById('field-custom-url');
  if (customUrlField) {
    customUrlField.style.display = provider === 'custom' ? 'block' : 'none';
  }
}

function syncFormFromState(container: HTMLElement, state: SettingsState): void {
  const providerSelect = container.querySelector<HTMLSelectElement>('#provider');
  const modelInput = container.querySelector<HTMLInputElement>('#model');
  const apiKeyInput = container.querySelector<HTMLInputElement>('#api-key');
  const customUrlInput = container.querySelector<HTMLInputElement>('#custom-base-url');
  const namingPatternInput = container.querySelector<HTMLInputElement>('#naming-pattern');
  const statusDot = container.querySelector<HTMLSpanElement>('#status-dot');
  const statusText = container.querySelector<HTMLSpanElement>('#status-text');

  providerSelect && (providerSelect.value = state.provider);
  modelInput && (modelInput.value = state.model);
  apiKeyInput && (apiKeyInput.value = state.apiKey);
  customUrlInput && (customUrlInput.value = state.customBaseUrl);
  namingPatternInput && (namingPatternInput.value = state.namingPattern);

  updateProviderFields(state.provider);

  if (statusDot && statusText) {
    statusDot.className = 'status-dot';
    switch (state.testStatus) {
      case 'success':
        statusDot.classList.add('status-success');
        statusText.textContent = state.testMessage || 'Connected';
        break;
      case 'error':
        statusDot.classList.add('status-error');
        statusText.textContent = state.testMessage || 'Connection failed';
        break;
      case 'testing':
        statusDot.classList.add('status-testing');
        statusText.textContent = 'Testing...';
        break;
      default:
        statusText.textContent = 'Not tested';
    }
  }
}