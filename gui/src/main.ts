import './css/tailwind.css';
import { initTitlebar } from './lib/titlebar';
import { initTheme } from './lib/theme';
import { initRenderer, createStatusBar } from './renderer';
import { loadConfig } from './lib/config-store';
import { validateConfig } from './lib/sidecar';
import { showToast } from './lib/toast';

function boot(): void {
  initTitlebar();
  initTheme();

  const appEl = document.getElementById('app');
  if (!appEl) throw new Error('#app element not found');

  const content = document.createElement('div');
  content.className = 'flex flex-col flex-1 min-h-0';
  appEl.appendChild(content);

  createStatusBar(appEl);
  initRenderer(content);

  // Load the persisted config first so views render with real values, then
  // surface configuration problems. Neither failure should block the UI.
  loadConfig().catch(() => {});

  validateConfig()
    .then((validation) => {
      if (!validation.valid) {
        const errors = validation.issues.filter((i) => i.level === 'error');
        if (errors.length > 0) {
          showToast('Config: ' + errors.map((e) => e.message).join(', '), 'warning');
        }
      }
    })
    .catch(() => {
      showToast('Could not validate config on startup', 'warning');
    });
}

if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', boot, { once: true });
} else {
  boot();
}
