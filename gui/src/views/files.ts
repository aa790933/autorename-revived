import { getState, subscribe, addFiles, clearFiles, setState, updateFileStatuses } from '../lib/state';
import { setupDragDrop } from '../lib/dnd';
import { pickFiles, pickFolder } from '../lib/filepicker';
import { renameFiles, undoRename, cancelRename } from '../lib/sidecar';
import { getConfigSync } from '../lib/config-store';
import { showToast } from '../lib/toast';
import { escapeHtml } from '../lib/utils';
import type { AppState, FileEntry } from '../lib/state';

let container: HTMLElement;
let cleanupDnd: (() => void) | undefined;
let unsubscribe: (() => void) | undefined;

// Module-level flag: tracks whether the current rename operation has been
// cancelled by the user. This prevents a stale `renameFiles` promise from
// overwriting the cancelled state set in `handleCancel`.
let cancelRequested = false;

// Guard against double-click: while a rename is in flight, further clicks are
// ignored until the backend call settles.
let renameInFlight = false;

export function renderFilesView(root: HTMLElement): void {
  container = root;

  cleanupDnd = setupDragDrop(
    (paths) => {
      if (paths.length > 0) addFiles(paths);
      else showToast('No supported files in drop', 'warning');
    },
    (hovering) => {
      const dropZone = container.querySelector('.drop-zone');
      const fileList = container.querySelector('#file-list-container');
      if (dropZone) {
        dropZone.classList.toggle('drop-zone-active', hovering);
      } else if (fileList) {
        fileList.classList.toggle('drag-hover', hovering);
      }
    },
  );

  unsubscribe = subscribe(render);
  render(getState());
}

export function destroyFilesView(): void {
  unsubscribe?.();
  unsubscribe = undefined;
  cleanupDnd?.();
}

function render(state: AppState): void {
  if (state.view !== 'files') return;

  if (state.files.length === 0) {
    renderEmpty();
  } else {
    renderFileList(state);
  }
}

function renderEmpty(): void {
  container.innerHTML = `
    <div class="flex flex-col items-center justify-center flex-1 p-8">
      <div class="drop-zone flex flex-col items-center justify-center gap-4 p-12 w-full max-w-lg
                  border-2 border-dashed rounded-xl border-[var(--border-secondary)]
                  hover:border-[var(--color-primary)] transition-colors cursor-pointer"
           id="drop-zone-area">
        <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor"
             stroke-width="1.5" class="text-[var(--text-tertiary)]">
          <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
          <polyline points="14 2 14 8 20 8"/>
          <line x1="12" y1="18" x2="12" y2="12"/>
          <line x1="9" y1="15" x2="12" y2="12"/>
          <line x1="15" y1="15" x2="12" y2="12"/>
        </svg>
        <p class="text-[var(--text-secondary)] text-center">
          Drop PDF, Word, Excel, PowerPoint, image, and text files here<br>
          <span class="text-sm text-[var(--text-tertiary)]">or click to browse</span>
        </p>
        <div class="flex gap-3 mt-2">
          <button class="btn btn-primary btn-sm" id="btn-browse-files">Browse Files</button>
          <button class="btn btn-secondary btn-sm" id="btn-browse-folder">Browse Folder</button>
        </div>
      </div>
    </div>
  `;

  document.getElementById('btn-browse-files')?.addEventListener('click', async () => {
    const files = await pickFiles();
    if (files.length > 0) addFiles(files);
  });

  document.getElementById('btn-browse-folder')?.addEventListener('click', async () => {
    const files = await pickFolder();
    if (files === null) return;
    if (files.length > 0) addFiles(files);
    else showToast('No supported files found in folder', 'warning');
  });

  document.getElementById('drop-zone-area')?.addEventListener('click', async (e) => {
    if ((e.target as HTMLElement).closest('button')) return;
    const files = await pickFiles();
    if (files.length > 0) addFiles(files);
  });
}

// ---------------------------------------------------------------------------
// File row rendering helpers
// ---------------------------------------------------------------------------

function fileVisualState(f: FileEntry): 'pending' | 'processing' | 'preview' | 'completed' | 'failed' | 'skipped' {
  if (f.status === 'pending' && f.result?.new_name) return 'preview';
  return f.status;
}

function renderFileRow(f: FileEntry): string {
  const vs = fileVisualState(f);
  const newName = f.result?.new_name;
  const error = f.result?.error;

  const dotClass = `fq-dot fq-dot-${vs}`;
  const badgeClass = `fq-badge fq-badge-${vs}`;
  const badgeLabel = vs === 'preview' ? 'preview' : f.status;

  let detail = '';
  if (newName && (vs === 'preview' || vs === 'completed')) {
    const nameClass = vs === 'preview' ? 'fq-new-name-preview' : 'fq-new-name-completed';
    let suggestionsHtml = '';
    const suggestionNames = f.result?.suggestion_names ?? [];
    const suggestionLanguages = f.result?.suggestion_languages ?? [];
    if (suggestionNames.length > 0 && vs === 'preview') {
      const suggestionItems = suggestionNames.map((s, i) => {
        const lang = suggestionLanguages[i] || '';
        const label = lang ? `${lang}` : `Option ${i + 1}`;
        return `<button type="button" class="fq-suggestion-btn" data-file="${escapeHtml(f.path)}" data-name="${escapeHtml(s)}" title="${escapeHtml(label)}">${escapeHtml(s)}</button>`;
      }).join('');
      suggestionsHtml = `
        <div class="fq-suggestions">
          <span class="fq-suggestion-label">Suggestions:</span>
          ${suggestionItems}
        </div>`;
    }
    detail = `
      <span class="fq-preview">
        <span class="fq-arrow">\u2192</span>
        <span class="fq-new-name ${nameClass}">${escapeHtml(newName)}</span>
      </span>
      ${suggestionsHtml}`;
  }
  if (error && vs === 'failed') {
    detail = `<span class="fq-error">${escapeHtml(error)}</span>`;
  }
  const warnings = f.result?.warnings ?? [];
  if (warnings.length > 0 && vs !== 'failed') {
    detail += `<span class="fq-warning">\u26a0 ${warnings.map(escapeHtml).join('; ')}</span>`;
  }

  return `
    <div class="fq-row">
      <span class="${dotClass}"></span>
      <div class="fq-info">
        <span class="fq-name">${escapeHtml(f.name)}</span>
        ${detail}
      </div>
      <span class="${badgeClass}">${badgeLabel}</span>
    </div>`;
}

// ---------------------------------------------------------------------------
// File list view
// ---------------------------------------------------------------------------

function renderFileList(state: AppState): void {
  const hasResults = state.lastResult !== null;
  const pendingCount = state.files.filter((f) => f.status === 'pending').length;
  const busy = state.processing;

  // Button bar: always visible, disabled during processing
  let actionsHtml: string;
  if (hasResults) {
    actionsHtml = `
      <div class="fq-actions-left">
        <button class="btn btn-secondary btn-sm" id="btn-undo" ${busy ? 'disabled' : ''}>Undo Last</button>
        <button class="btn btn-primary btn-sm" id="btn-add-more" ${busy ? 'disabled' : ''}>Add More Files</button>
      </div>`;
  } else {
    const noFiles = pendingCount === 0;
    const blocked = busy || noFiles || !!state.statusError;
    const cancelDisabled = !busy;
    actionsHtml = `
      <div class="fq-actions-left">
        <button class="btn btn-error btn-sm" id="btn-cancel" ${cancelDisabled ? 'disabled' : ''}>Cancel</button>
        <button class="btn btn-secondary btn-sm" id="btn-dry-run" ${blocked ? 'disabled' : ''}>Dry Run</button>
        <button class="btn btn-primary btn-sm" id="btn-rename" ${blocked ? 'disabled' : ''}>
          Rename ${pendingCount} File${pendingCount !== 1 ? 's' : ''}
        </button>
      </div>`;
  }

  container.innerHTML = `
    <div class="fq-container" id="file-list-container">
      <div class="fq-header">
        <span class="fq-count">${state.files.length} file${state.files.length !== 1 ? 's' : ''}</span>
        ${busy ? `<span class="fq-progress-text">${state.progress || 'Processing\u2026'}</span>` : ''}
      </div>
      ${busy ? '<div class="fq-progress-bar"></div>' : ''}
      <div class="fq-list">
        ${state.files.map(renderFileRow).join('')}
      </div>
      <div class="fq-actions">
        ${actionsHtml}
        <button class="btn btn-ghost btn-sm" id="btn-clear" ${busy ? 'disabled' : ''}>Clear</button>
      </div>
    </div>
  `;

  // Bind actions
  document.getElementById('btn-dry-run')?.addEventListener('click', () => runRename(true));
  document.getElementById('btn-rename')?.addEventListener('click', () => runRename(false));
  document.getElementById('btn-cancel')?.addEventListener('click', handleCancel);
  document.getElementById('btn-clear')?.addEventListener('click', () => clearFiles());
  document.getElementById('btn-undo')?.addEventListener('click', handleUndo);
  document.getElementById('btn-add-more')?.addEventListener('click', async () => {
    const files = await pickFiles();
    if (files.length > 0) addFiles(files);
  });

  // Bind suggestion buttons
  container.querySelectorAll('.fq-suggestion-btn').forEach((btn) => {
    btn.addEventListener('click', () => {
      const filePath = (btn as HTMLElement).dataset.file;
      const newName = (btn as HTMLElement).dataset.name;
      if (filePath && newName) {
        selectSuggestion(filePath, newName);
      }
    });
  });
}

// ---------------------------------------------------------------------------
// Suggestion selection
// ---------------------------------------------------------------------------

function selectSuggestion(filePath: string, newName: string): void {
  const currentState = getState();
  const updatedFiles = currentState.files.map((entry) => {
    if (entry.path !== filePath) return entry;
    // Compute new_path from parent_dir + new_name
    const lastSep = Math.max(filePath.lastIndexOf('/'), filePath.lastIndexOf('\\'));
    const parentDir = lastSep >= 0 ? filePath.slice(0, lastSep + 1) : '';
    const newPath = parentDir + newName;
    const updatedResult = entry.result
      ? { ...entry.result, new_name: newName, new_path: newPath }
      : undefined;
    return { ...entry, result: updatedResult };
  });
  setState({ files: updatedFiles });
  showToast(`Selected name: ${newName}`, 'success');
  render(getState());
}

// ---------------------------------------------------------------------------
// Rename
// ---------------------------------------------------------------------------

/**
 * Run the rename pipeline.
 *
 * `dryRun` asks the backend for the proposed names without touching disk; the
 * preview is stored on the file entries so the list can show them. A real run
 * always goes through the backend again rather than replaying a cached
 * preview, so the undo log, collision handling and vision fallbacks behave
 * exactly like a first run.
 */
async function runRename(dryRun: boolean): Promise<void> {
  if (renameInFlight) return;

  // Reset cancellation flag at the start of every new operation.
  cancelRequested = false;

  const state = getState();
  const targets = state.files.filter(
    (f) => f.status === 'pending' || f.status === 'skipped',
  );
  const paths = targets.map((f) => f.path);

  if (paths.length === 0) {
    showToast('No files to process', 'warning');
    return;
  }

  renameInFlight = true;

  // Mark only the pending/skipped files as 'processing', preserving
  // already-completed or failed entries instead of dropping them.
  const processing = new Set(paths);
  setState({
    processing: true,
    progress: dryRun ? 'Previewing\u2026' : 'Starting\u2026',
    statusError: '',
    files: state.files.map((f) =>
      processing.has(f.path) ? { ...f, status: 'processing' as const } : f,
    ),
  });

  try {
    const batch = await renameFiles(paths, {
      dryRun,
      provider: getConfigSync().ai.provider,
    });

    // The user cancelled while the request was in flight: `handleCancel`
    // already reset the UI state, so discard the result rather than
    // overwriting it.
    if (cancelRequested) return;

    updateFileStatuses(batch, dryRun);

    if (dryRun) {
      setState({ dryRunResult: batch });
      if (batch.completed === 0 && batch.skipped > 0) {
        showToast('Preview: all files already correctly named', 'info');
      } else {
        showToast(`Preview: ${batch.completed} to process, ${batch.skipped} to skip`, 'info');
      }
    } else {
      // Undo is only possible when something was actually renamed.
      setState({
        lastResult: batch,
        lastBatchId: batch.completed > 0 ? batch.batch_id : null,
      });

      if (batch.failed > 0) {
        showToast(`${batch.completed} completed, ${batch.failed} failed`, 'warning');
      } else if (batch.completed === 0 && batch.skipped > 0) {
        showToast('All files already correctly named', 'info');
      } else {
        showToast(`${batch.completed} files renamed successfully`, 'success');
      }

      // Surface per-file warnings (extraction failures, truncation, …) but cap
      // the number of toasts: a 500-file batch would otherwise bury the UI.
      const unique = [...new Set(batch.files.flatMap((f) => f.warnings ?? []))];
      const MAX_WARNING_TOASTS = 5;
      for (const w of unique.slice(0, MAX_WARNING_TOASTS)) {
        showToast(w, 'warning');
      }
      if (unique.length > MAX_WARNING_TOASTS) {
        showToast(`…and ${unique.length - MAX_WARNING_TOASTS} more warnings`, 'warning');
      }
    }

    setState({ processing: false, progress: '', statusError: '' });
  } catch (err) {
    if (cancelRequested) return;
    // Any file left in `processing` must not stay there forever.
    const message = String(err);
    const stuck = getState().files.map((f) =>
      f.status === 'processing' ? { ...f, status: 'failed' as const, result: undefined } : f,
    );
    setState({
      processing: false,
      progress: '',
      files: stuck,
      statusError: message,
    });
    showToast(`Rename failed: ${message}`, 'danger');
  } finally {
    renameInFlight = false;
  }
}

async function handleCancel(): Promise<void> {
  cancelRequested = true;
  try {
    await cancelRename();
  } catch {
    // The backend flag is best-effort; the local flag already stops the UI
    // from applying a stale result.
  }
  // `renameInFlight` deliberately stays true: the backend call is still
  // running, and letting a new batch start now would race the cancellation
  // and mix two batches in the undo log.
  const currentState = getState();
  setState({
    processing: false,
    progress: '',
    statusError: '',
    files: currentState.files.map((f) =>
      f.status === 'processing' ? { ...f, status: 'failed' as const } : f,
    ),
  });
  showToast('Rename operation cancelled', 'info');
}

async function handleUndo(): Promise<void> {
  const { lastBatchId } = getState();
  if (!lastBatchId) {
    showToast('Nothing to undo', 'info');
    return;
  }
  if (renameInFlight) return;

  renameInFlight = true;
  setState({ processing: true, progress: 'Undoing\u2026' });

  try {
    const result = await undoRename(lastBatchId);
    setState({ processing: false, progress: '' });

    if (result.success) {
      showToast(`${result.restored} files restored`, 'success');
      clearFiles();
    } else if (result.failed > 0) {
      showToast(`Undo: ${result.restored} restored, ${result.failed} failed`, 'warning');
    } else {
      showToast('Nothing to undo', 'info');
    }
  } catch (err) {
    setState({ processing: false, progress: '' });
    showToast(`Undo failed: ${err}`, 'danger');
  } finally {
    renameInFlight = false;
  }
}
