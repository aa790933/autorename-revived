import { describe, it, expect, beforeEach } from 'vitest';
import {
  getState,
  setState,
  addFiles,
  clearFiles,
  updateFileStatuses,
  subscribe,
} from './state';
import type { BatchResult } from './types';

beforeEach(() => {
  setState({
    view: 'files',
    files: [],
    processing: false,
    progress: '',
    lastResult: null,
    dryRunResult: null,
    statusError: '',
    lastBatchId: null,
  });
});

function makeBatchResult(
  files: Array<{ file: string; status: 'completed' | 'skipped' | 'failed'; new_name?: string }>,
  dryRun = false,
): BatchResult {
  return {
    success: files.every((f) => f.status !== 'failed'),
    total: files.length,
    completed: files.filter((f) => f.status === 'completed').length,
    skipped: files.filter((f) => f.status === 'skipped').length,
    failed: files.filter((f) => f.status === 'failed').length,
    dry_run: dryRun,
    batch_id: 'test-batch',
    files: files.map((f) => ({
      file: f.file,
      status: f.status,
      new_name: f.new_name ?? null,
      new_path: null,
      error: null,
      warnings: [],
      company: null,
      date: null,
      doc_type: null,
      provider: null,
      model: null,
      suggestion_names: [],
      suggestion_languages: [],
    })),
  };
}

// ---------------------------------------------------------------------------
// Path matching
// ---------------------------------------------------------------------------
describe('updateFileStatuses — path matching', () => {
  it('matches identical paths', () => {
    addFiles(['/home/user/invoice.pdf']);
    updateFileStatuses(
      makeBatchResult([
        { file: '/home/user/invoice.pdf', status: 'completed', new_name: '20250101 ACME Invoice.pdf' },
      ]),
      false,
    );

    const state = getState();
    expect(state.files[0].status).toBe('completed');
    expect(state.files[0].result?.new_name).toBe('20250101 ACME Invoice.pdf');
  });

  it('matches Windows backslash paths against forward-slash results', () => {
    // The backend may report either separator; matching must be
    // separator-insensitive, otherwise every file stays "processing".
    addFiles(['D:\\Documents\\invoice.pdf']);

    updateFileStatuses(
      makeBatchResult([
        { file: 'D:/Documents/invoice.pdf', status: 'completed', new_name: 'renamed.pdf' },
      ]),
      false,
    );

    expect(getState().files[0].status).toBe('completed');
  });

  it('matches mixed-case drive letters and path case', () => {
    addFiles(['d:\\Documents\\invoice.pdf']);

    updateFileStatuses(
      makeBatchResult([{ file: 'D:/DOCUMENTS/INVOICE.PDF', status: 'completed' }]),
      false,
    );

    expect(getState().files[0].status).toBe('completed');
  });

  it('marks a file failed when the batch returned no result for it', () => {
    addFiles(['/home/user/a.pdf']);
    setState({ files: [{ path: '/home/user/a.pdf', name: 'a.pdf', status: 'processing' }] });

    updateFileStatuses(makeBatchResult([]), false);

    expect(getState().files[0].status).toBe('failed');
  });

  it('leaves unrelated pending files untouched', () => {
    addFiles(['/home/user/a.pdf', '/home/user/b.pdf']);
    updateFileStatuses(makeBatchResult([{ file: '/home/user/a.pdf', status: 'completed' }]), false);

    const files = getState().files;
    expect(files[0].status).toBe('completed');
    expect(files[1].status).toBe('pending');
  });
});

// ---------------------------------------------------------------------------
// Dry run
// ---------------------------------------------------------------------------
describe('updateFileStatuses — dry run', () => {
  it('keeps completed files pending but stores the proposed name', () => {
    addFiles(['/home/user/invoice.pdf']);
    updateFileStatuses(
      makeBatchResult(
        [{ file: '/home/user/invoice.pdf', status: 'completed', new_name: '20250101 ACME Invoice.pdf' }],
        true,
      ),
      true,
    );

    const file = getState().files[0];
    expect(file.status).toBe('pending');
    expect(file.result?.new_name).toBe('20250101 ACME Invoice.pdf');
  });

  it('applies terminal skipped/failed statuses immediately during a dry run', () => {
    addFiles(['/home/user/ok.pdf', '/home/user/named.pdf', '/home/user/broken.pdf']);
    updateFileStatuses(
      makeBatchResult(
        [
          { file: '/home/user/ok.pdf', status: 'completed', new_name: 'x.pdf' },
          { file: '/home/user/named.pdf', status: 'skipped' },
          { file: '/home/user/broken.pdf', status: 'failed' },
        ],
        true,
      ),
      true,
    );

    const files = getState().files;
    expect(files[0].status).toBe('pending');
    expect(files[1].status).toBe('skipped');
    expect(files[2].status).toBe('failed');
  });

  it('a real rename after a dry run applies the completed status', () => {
    addFiles(['/home/user/invoice.pdf']);

    updateFileStatuses(
      makeBatchResult([{ file: '/home/user/invoice.pdf', status: 'completed', new_name: 'x.pdf' }], true),
      true,
    );
    expect(getState().files[0].status).toBe('pending');

    updateFileStatuses(
      makeBatchResult([{ file: '/home/user/invoice.pdf', status: 'completed', new_name: 'x.pdf' }]),
      false,
    );
    expect(getState().files[0].status).toBe('completed');
  });
});

// ---------------------------------------------------------------------------
// Subscription
// ---------------------------------------------------------------------------
describe('subscribe', () => {
  it('returns an unsubscribe function that removes the listener', () => {
    let callCount = 0;
    const unsub = subscribe(() => {
      callCount++;
    });

    setState({ progress: 'a' });
    expect(callCount).toBe(1);

    unsub();
    setState({ progress: 'b' });
    expect(callCount).toBe(1);
  });
});

// ---------------------------------------------------------------------------
// File list management
// ---------------------------------------------------------------------------
describe('addFiles', () => {
  it('deduplicates by path', () => {
    addFiles(['/a.pdf', '/b.pdf']);
    addFiles(['/b.pdf', '/c.pdf']);
    expect(getState().files.map((f) => f.path)).toEqual(['/a.pdf', '/b.pdf', '/c.pdf']);
  });

  it('derives the display name from the path', () => {
    addFiles(['C:\\docs\\invoice.pdf']);
    expect(getState().files[0].name).toBe('invoice.pdf');
  });

  it('clears stale results and errors so a retry starts clean', () => {
    setState({
      dryRunResult: makeBatchResult([]),
      lastResult: makeBatchResult([]),
      statusError: 'boom',
    });

    addFiles(['/new.pdf']);

    const s = getState();
    expect(s.dryRunResult).toBeNull();
    expect(s.lastResult).toBeNull();
    expect(s.statusError).toBe('');
  });
});

describe('clearFiles', () => {
  it('resets files, results, progress and batch id', () => {
    addFiles(['/a.pdf']);
    setState({
      progress: 'Processing…',
      dryRunResult: makeBatchResult([]),
      lastResult: makeBatchResult([]),
      lastBatchId: 'gui-1',
      statusError: 'boom',
    });

    clearFiles();

    const s = getState();
    expect(s.files).toEqual([]);
    expect(s.dryRunResult).toBeNull();
    expect(s.lastResult).toBeNull();
    expect(s.lastBatchId).toBeNull();
    expect(s.progress).toBe('');
    expect(s.statusError).toBe('');
  });
});
