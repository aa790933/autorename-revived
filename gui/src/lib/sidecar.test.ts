import { beforeEach, describe, expect, it, vi } from 'vitest';

// Mock Tauri IPC invoke
const invokeSpy = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => invokeSpy(...args),
}));

vi.mock('@tauri-apps/api/path', () => ({
  appDataDir: vi.fn(async () => 'D:\\appdata'),
}));

describe('sidecar IPC wrappers', () => {
  beforeEach(() => {
    invokeSpy.mockReset();
  });

  it('renameFiles calls invoke with correct arguments', async () => {
    invokeSpy.mockResolvedValue({
      success: true,
      total: 2,
      completed: 2,
      skipped: 0,
      failed: 0,
      files: [],
      dry_run: false,
    });

    const { renameFiles } = await import('./sidecar');
    const result = await renameFiles(
      ['file1.pdf', 'file2.docx'],
      { dryRun: true, provider: 'gemini' },
    );

    expect(invokeSpy).toHaveBeenCalledWith('rename_files', {
      paths: ['file1.pdf', 'file2.docx'],
      options: { dryRun: true, provider: 'gemini' },
    });
    expect(result.success).toBe(true);
  });

  it('renameFiles returns ErrorResult on invoke failure', async () => {
    invokeSpy.mockRejectedValue(new Error('IPC failed'));

    const { renameFiles, isErrorResult } = await import('./sidecar');
    const result = await renameFiles(['file.pdf']);

    expect(isErrorResult(result)).toBe(true);
    if ('error_type' in result) {
      expect(result.error_type).toBe('sidecar_error');
    }
  });

  it('cancelRename invokes cancel_rename command', async () => {
    invokeSpy.mockResolvedValue(true);

    const { cancelRename } = await import('./sidecar');
    const result = await cancelRename();

    expect(invokeSpy).toHaveBeenCalledWith('cancel_rename');
    expect(result).toBe(true);
  });

  it('getUndoLogDir returns appDataDir', async () => {
    const { getUndoLogDir } = await import('./sidecar');
    const dir = await getUndoLogDir();

    expect(dir).toBe('D:\\appdata');
  });

  it('testApiConnection passes provider, apiKey, model to invoke', async () => {
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

  it('testApiConnection returns error result on failure', async () => {
    invokeSpy.mockRejectedValue(new Error('Connection refused'));

    const { testApiConnection } = await import('./sidecar');
    const result = await testApiConnection('openai', 'sk-key', 'gpt-4o');

    expect(result.success).toBe(false);
    expect(result.message).toContain('Connection refused');
  });

  it('isErrorResult correctly identifies ErrorResult', async () => {
    const { isErrorResult } = await import('./sidecar');
    const errorResult = { success: false as const, error_type: 'auth_error', message: 'bad key', suggestion: '' };
    const batchResult = { success: true as const, total: 1, completed: 1, skipped: 0, failed: 0, files: [], dry_run: false };

    expect(isErrorResult(errorResult)).toBe(true);
    expect(isErrorResult(batchResult as never)).toBe(false);
  });
});
