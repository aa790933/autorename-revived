import { beforeEach, describe, expect, it, vi } from 'vitest';

const invokeSpy = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => invokeSpy(...args),
}));

describe('loadConfig', () => {
  beforeEach(() => {
    invokeSpy.mockReset();
    vi.resetModules();
  });

  it('deep-merges a partial persisted config over the defaults', async () => {
    // A settings file written by an older version may be missing whole
    // sections; a shallow spread would leave them undefined.
    invokeSpy.mockResolvedValue({
      ai: { provider: 'ollama', custom_model: 'llama3.2' },
      naming: { template: '{date}_{company}' },
    });

    const { loadConfig } = await import('./config-store');
    const config = await loadConfig();

    expect(config.ai.provider).toBe('ollama');
    expect(config.ai.custom_model).toBe('llama3.2');
    // Not present in the persisted blob -> default kept.
    expect(config.ai.timeout).toBe(30);
    expect(config.naming.template).toBe('{date}_{company}');
    expect(config.naming.max_length).toBe(128);
    expect(config.document.vision).toBe('auto');
    expect(config.undo.max_entries).toBe(100);
    expect(config.naming.suggestion_languages).toEqual([]);
  });

  it('falls back to defaults when the backend call fails', async () => {
    invokeSpy.mockRejectedValue(new Error('no backend'));

    const { loadConfig, getConfigSync } = await import('./config-store');
    const config = await loadConfig();

    expect(config.ai.provider).toBe('gemini');
    expect(config.max_workers).toBe(4);
    expect(getConfigSync()).toBe(config);
  });

  it('caches after the first load and reloadConfig forces a re-read', async () => {
    invokeSpy.mockResolvedValue({ ai: { provider: 'openai' } });

    const { loadConfig, reloadConfig } = await import('./config-store');
    await loadConfig();
    await loadConfig();
    expect(invokeSpy).toHaveBeenCalledTimes(1);

    invokeSpy.mockResolvedValue({ ai: { provider: 'anthropic' } });
    const reloaded = await reloadConfig();
    expect(invokeSpy).toHaveBeenCalledTimes(2);
    expect(reloaded.ai.provider).toBe('anthropic');
  });

  it('keeps array values from the persisted config rather than merging them', async () => {
    invokeSpy.mockResolvedValue({
      naming: { suggestion_languages: ['French', 'Arabic'] },
    });

    const { loadConfig } = await import('./config-store');
    const config = await loadConfig();
    expect(config.naming.suggestion_languages).toEqual(['French', 'Arabic']);
  });
});
