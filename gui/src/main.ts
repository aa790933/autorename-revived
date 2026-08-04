import { initTitlebar } from './lib/titlebar';
import { initTheme } from './lib/theme';
import { initRenderer, createStatusBar } from './renderer';
import { setState } from './lib/state';
import { loadConfig } from './lib/config-store';
import { validateConfig } from './lib/sidecar';

document.addEventListener('DOMContentLoaded', () => {
  initTitlebar();
  initTheme();

  const appEl = document.getElementById('app');
  if (!appEl) throw new Error('#app element not found');

  const content = document.createElement('div');
  content.className = 'flex flex-col flex-1 min-h-0';
  appEl.appendChild(content);

  createStatusBar(appEl);
  initRenderer(content);

  loadConfig().catch(() => {});

  validateConfig().then((validation) => {
    if (!validation.valid) {
      const errors = validation.issues.filter((i) => i.level === 'error');
      if (errors.length > 0) setState({ statusError: 'Config error' });
    }
  }).catch((err) => {
    const errStr = String(err);
    if (errStr.includes('sidecar') || errStr.includes('not found') || errStr.includes('binaries')) {
      setState({ statusError: 'CLI not found' });
    } else {
      setState({ statusError: 'Config error' });
    }
  });
});
