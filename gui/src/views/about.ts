import { setState } from '../lib/state';
import { getVersion } from '../lib/sidecar';
import { openUrl } from '@tauri-apps/plugin-opener';

const REPO_URL = 'https://github.com/aa790933/autorename-revived';
const LICENSE_URL = 'https://github.com/aa790933/autorename-revived/blob/main/LICENSE';
const PHRASEVAULT_URL = 'https://phrasevault.app';

const externalLinkIcon = `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/><polyline points="15 3 21 3 21 9"/><line x1="10" y1="14" x2="21" y2="3"/></svg>`;

/** Version injected by Vite from `package.json`; replaced by the backend value. */
const BUILD_VERSION = typeof __APP_VERSION__ !== 'undefined' ? __APP_VERSION__ : '0.0.0';

export function renderAboutView(root: HTMLElement): void {
  root.innerHTML = `
    <div class="about-scroll">
    <div class="about-view">

      <div class="about-header">
        <h1>AutoRename-Revived</h1>
        <p class="about-tagline">
          AI-powered document auto-renamer.<br>
          Extracts company name, date, document type and subject from PDFs, Word docs, spreadsheets, presentations, images, and text files.
        </p>
        <p class="about-version">v<span id="about-version">${BUILD_VERSION}</span></p>
      </div>

      <div class="about-section">
        <p class="about-section-title">Open Source</p>
        <p>
          Licensed under the
          <button class="about-link" data-href="${LICENSE_URL}">MIT License ${externalLinkIcon}</button>
        </p>
        <p style="margin-top: 0.5rem;">
          <button class="about-link" data-href="${REPO_URL}">View on GitHub ${externalLinkIcon}</button>
        </p>
      </div>

      <div class="about-section">
        <p class="about-section-title">Technology</p>
        <p style="font-size: var(--font-size-xs, 0.75rem); color: var(--text-secondary); line-height: 1.6;">
          Tauri v2 desktop app with a TypeScript frontend and a Rust backend.<br>
          Local text extraction (DOCX, XLSX, PPTX, PDF, plain text) with multi-provider
          AI metadata extraction, including a vision fallback for scanned documents.
        </p>
      </div>

      <div class="about-section">
        <p class="about-section-title">Disclaimer</p>
        <p>
          This software is provided "as is", without warranty of any kind, express or implied.
          The authors are not liable for any claim, damages, or other liability arising from its use.
        </p>
      </div>

      <div class="about-support-card">
        <p>
          If you find this project useful, please consider supporting its development by checking out
        </p>
        <p>
          <button class="about-link" data-href="${PHRASEVAULT_URL}" style="font-size:var(--font-size-sm);">PhraseVault ${externalLinkIcon}</button>
        </p>
        <p>
          A text expander and snippet manager by the same developer. Your support helps keep this project free and maintained.
        </p>
      </div>

      <button class="btn btn-ghost btn-sm" id="btn-back-from-about">&larr; Back</button>
    </div>
    </div>
  `;

  // External link handler
  root.querySelectorAll<HTMLElement>('[data-href]').forEach((el) => {
    el.addEventListener('click', () => {
      const url = el.dataset.href;
      if (url) {
        openUrl(url).catch(() => {
          /* no browser handler available */
        });
      }
    });
  });

  document.getElementById('btn-back-from-about')?.addEventListener('click', () => {
    setState({ view: 'files' });
  });

  // Prefer the backend's own version: it is compiled into the binary and is
  // what the user actually installed.
  getVersion()
    .then((version) => {
      const el = document.getElementById('about-version');
      if (el && version) el.textContent = version;
    })
    .catch(() => {
      /* keep the build-time version */
    });
}
