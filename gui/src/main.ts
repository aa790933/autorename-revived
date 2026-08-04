import { initState } from './lib/state';
import { renderSettings } from './views/settings';
import './css/index.css';

function mountApp(): void {
  const app = document.getElementById('app');
  if (!app) return;
  initState();
  renderSettings(app);
}

document.addEventListener('DOMContentLoaded', mountApp);