import { beforeEach, describe, expect, it, vi } from 'vitest';

const sidecarSpy = vi.fn();

vi.mock('@tauri-apps/api/path', () => ({
  appDataDir: vi.fn(async () => 'D:\\appdata'),
  join: vi.fn(async (...parts: string[]) => parts.join('\\')),
}));

vi.mock('@tauri-apps/plugin-shell', () => ({
  Command: {
    sidecar: (...args: unknown[]) => sidecarSpy(...args),
  },
}));

describe('sidecar config path resolution', () => {
  beforeEach(() => {
    sidecarSpy.mockReset();
    sidecarSpy.mockReturnValue({
      stderr: { on: vi.fn() },
      execute: vi.fn(async () => ({
        code: 0,
        stdout: JSON.stringify({ success: true, issues: [] }),
        stderr: '',
      })),
    });
  });

  it('passes the resolved config path to validation calls', async () => {
    const { validateConfig } = await import('./sidecar');

    await validateConfig();

    expect(sidecarSpy).toHaveBeenCalledWith('binaries/autorename-revived-cli', [
      'config',
      'validate',
      '--output',
      'json',
      '--config',
      'D:\\appdata\\config.yaml',
    ], {
      encoding: 'utf-8',
      env: {
        PYTHONUTF8: '1',
        PYTHONIOENCODING: 'utf-8',
      },
    });
  });

  it('passes provider, api-key, and model to test-connection', async () => {
    sidecarSpy.mockReturnValue({
      stderr: { on: vi.fn() },
      execute: vi.fn(async () => ({
        code: 0,
        stdout: JSON.stringify({ success: true, message: 'Connected', latency_ms: 100, provider: 'openai' }),
        stderr: '',
      })),
    });
    const { testApiConnection } = await import('./sidecar');

    await testApiConnection('openai', 'sk-test-key', 'gpt-4o');

    expect(sidecarSpy).toHaveBeenCalledWith('binaries/autorename-revived-cli', [
      'config',
      'test-connection',
      '--output',
      'json',
      '--provider',
      'openai',
      '--api-key',
      'sk-test-key',
      '--model',
      'gpt-4o',
      '--config',
      'D:\\appdata\\config.yaml',
    ], {
      encoding: 'utf-8',
      env: {
        PYTHONUTF8: '1',
        PYTHONIOENCODING: 'utf-8',
      },
    });
  });

  it('saveConfig sends key and value to CLI', async () => {
    sidecarSpy.mockReturnValue({
      stderr: { on: vi.fn() },
      execute: vi.fn(async () => ({
        code: 0,
        stdout: JSON.stringify({ success: true, saved_path: 'D:\\appdata\\config.yaml' }),
        stderr: '',
      })),
    });
    const { saveConfig } = await import('./sidecar');

    const result = await saveConfig('ai.api_key', 'sk-new-key');

    expect(result.success).toBe(true);
    expect(result.saved_path).toBe('D:\\appdata\\config.yaml');
    expect(sidecarSpy).toHaveBeenCalledWith('binaries/autorename-revived-cli', [
      'config',
      'save',
      '--key',
      'ai.api_key',
      '--value',
      'sk-new-key',
      '--output',
      'json',
      '--config',
      'D:\\appdata\\config.yaml',
    ], {
      encoding: 'utf-8',
      env: {
        PYTHONUTF8: '1',
        PYTHONIOENCODING: 'utf-8',
      },
    });
  });

  it('returns the resolved config path even before the CLI loads', async () => {
    const { getConfigPath } = await import('./sidecar');

    await expect(getConfigPath()).resolves.toBe('D:\\appdata\\config.yaml');
  });

  it('returns the resource directory for undo log', async () => {
    const { getUndoLogDir } = await import('./sidecar');

    await expect(getUndoLogDir()).resolves.toBe('D:\\appdata');
  });
});
