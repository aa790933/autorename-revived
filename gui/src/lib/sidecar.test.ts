import { beforeEach, describe, expect, it, vi } from 'vitest';

// Mock Tauri IPC invoke. The mock must be declared before the module under
// test is imported, so each test imports `./sidecar` dynamically.
const invokeSpy = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => invokeSpy(...args),
}));

/**
 * These tests pin the *contract* between the frontend wrappers and the Rust
 * commands: the command name, and the argument keys Tauri will map onto the
 * snake_case Rust parameters. A mismatch here is a silent runtime failure —
 * exactly the class of bug this suite exists to catch.
 */
describe('sidecar IPC wrappers', () => {
  beforeEach(() => {
    invokeSpy.mockReset();
    vi.resetModules();
  });

  it('renameFiles passes paths and options under the keys the backend expects', async () => {
    invokeSpy.mockResolvedValue({
      success: true,
      total: 2,
      completed: 2,
      skipped: 0,
      failed: 0,
      files: [],
      dry_run: false,
      batch_id: 'gui-1',
    });

    const { renameFiles } = await import('./sidecar');
    const result = await renameFiles(['file1.pdf', 'file2.docx'], {
      dryRun: true,
      provider: 'gemini',
    });

    expect(invokeSpy).toHaveBeenCalledWith('rename_files', {
      paths: ['file1.pdf', 'file2.docx'],
      options: { dryRun: true, provider: 'gemini' },
    });
    expect(result.success).toBe(true);
  });

  it('renameFiles propagates a backend rejection instead of swallowing it', async () => {
    invokeSpy.mockRejectedValue(new Error('IPC failed'));
    const { renameFiles } = await import('./sidecar');
    await expect(renameFiles(['file.pdf'])).rejects.toThrow('IPC failed');
  });

  it('cancelRename invokes cancel_rename with no arguments', async () => {
    invokeSpy.mockResolvedValue(true);
    const { cancelRename } = await import('./sidecar');

    await expect(cancelRename()).resolves.toBe(true);
    expect(invokeSpy).toHaveBeenCalledWith('cancel_rename');
  });

  it('undoRename sends batchId as null when no batch is given', async () => {
    invokeSpy.mockResolvedValue({
      success: true,
      restored: 0,
      failed: 0,
      files: [],
      batch_id: null,
    });
    const { undoRename } = await import('./sidecar');

    await undoRename();
    expect(invokeSpy).toHaveBeenCalledWith('undo_rename', { batchId: null });

    invokeSpy.mockClear();
    await undoRename('gui-20240101T000000');
    expect(invokeSpy).toHaveBeenCalledWith('undo_rename', {
      batchId: 'gui-20240101T000000',
    });
  });

  it('getUndoLogDir asks the backend rather than guessing appDataDir', async () => {
    invokeSpy.mockResolvedValue('D:\\portable');
    const { getUndoLogDir } = await import('./sidecar');

    await expect(getUndoLogDir()).resolves.toBe('D:\\portable');
    expect(invokeSpy).toHaveBeenCalledWith('get_undo_log_path');
  });

  it('testApiConnection maps provider/apiKey/model onto the Rust parameters', async () => {
    invokeSpy.mockResolvedValue({
      success: true,
      message: 'Connected',
      latency_ms: 100,
      provider: 'openai',
    });
    const { testApiConnection } = await import('./sidecar');

    const result = await testApiConnection('openai', 'sk-test-key', 'gpt-4o');

    expect(invokeSpy).toHaveBeenCalledWith('test_connection', {
      provider: 'openai',
      apiKey: 'sk-test-key',
      model: 'gpt-4o',
    });
    expect(result.success).toBe(true);
    expect(result.provider).toBe('openai');
  });

  it('saveConfigBatch sends the updates array under the "updates" key', async () => {
    invokeSpy.mockResolvedValue({ success: true, saved: 1, failed: 0, errors: [] });
    const { saveConfigBatch } = await import('./sidecar');

    const result = await saveConfigBatch([{ key: 'ai.provider', value: 'ollama' }]);

    // The Rust command takes `updates: Vec<ConfigUpdate>`; sending `pairs`
    // would make every settings save fail with a deserialization error.
    expect(invokeSpy).toHaveBeenCalledWith('save_app_config_batch', {
      updates: [{ key: 'ai.provider', value: 'ollama' }],
    });
    expect(result.saved).toBe(1);
  });

  it('getSupportedExtensions reads the backend list', async () => {
    invokeSpy.mockResolvedValue(['pdf', 'docx']);
    const { getSupportedExtensions } = await import('./sidecar');

    await expect(getSupportedExtensions()).resolves.toEqual(['pdf', 'docx']);
    expect(invokeSpy).toHaveBeenCalledWith('get_supported_extensions_list');
  });
});
