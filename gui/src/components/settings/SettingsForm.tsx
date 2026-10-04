/**
 * Settings Components - Settings Form
 */
import { useTranslation } from 'react-i18next';
import { useState, useEffect } from 'react';
import { useAppStore } from '@/store';
import { saveConfigBatch, testApiConnection, reloadConfig } from '@/services/api';
import { ProviderSelector, PROVIDERS } from './ProviderSelector';
import { Button, Input, Textarea, Select, Toggle } from '@/components/ui';
import { showToast } from '@/hooks/useToast';
import type { AppConfig, AiConfig, DocumentConfig, NamingConfig, UndoConfig } from '@/types';

type NestedSection = 'ai' | 'document' | 'naming' | 'undo';
type NestedConfig = AiConfig | DocumentConfig | NamingConfig | UndoConfig;

const PROVIDER_MODEL_KEYS: Record<string, keyof AiConfig> = {
  gemini: 'gemini_model',
  openai: 'model',
  anthropic: 'model',
  ollama: 'custom_model',
  xai: 'custom_model',
  custom: 'custom_model',
};

const PROVIDER_FIELDS: Record<string, { key: keyof AiConfig; label: string; hint?: string }[]> = {
  gemini: [
    { key: 'gemini_model', label: 'settings.aiProvider.model', hint: 'settings.aiProvider.modelHint' },
    { key: 'gemini_base_url', label: 'settings.aiProvider.baseUrl', hint: 'settings.aiProvider.baseUrlHint' },
  ],
  openai: [
    { key: 'model', label: 'settings.aiProvider.model', hint: 'settings.aiProvider.modelHint' },
    { key: 'base_url', label: 'settings.aiProvider.baseUrl', hint: 'settings.aiProvider.baseUrlHint' },
  ],
  anthropic: [
    { key: 'model', label: 'settings.aiProvider.model', hint: 'settings.aiProvider.modelHint' },
    { key: 'base_url', label: 'settings.aiProvider.baseUrl', hint: 'settings.aiProvider.baseUrlHint' },
  ],
  ollama: [
    { key: 'custom_model', label: 'settings.aiProvider.model', hint: 'settings.aiProvider.modelHint' },
    { key: 'ollama_base_url', label: 'settings.aiProvider.ollamaBaseUrl', hint: 'settings.aiProvider.ollamaBaseUrlHint' },
  ],
  xai: [
    { key: 'custom_model', label: 'settings.aiProvider.model', hint: 'settings.aiProvider.modelHint' },
    { key: 'base_url', label: 'settings.aiProvider.baseUrl', hint: 'settings.aiProvider.baseUrlHint' },
  ],
  custom: [
    { key: 'custom_model', label: 'settings.aiProvider.model', hint: 'settings.aiProvider.modelHint' },
    { key: 'custom_base_url', label: 'settings.aiProvider.customBaseUrl', hint: 'settings.aiProvider.customBaseUrlHint' },
  ],
};

const VISION_OPTIONS = [
  { value: 'auto', label: 'settings.documentProcessing.visionOptions.auto' },
  { value: 'true', label: 'settings.documentProcessing.visionOptions.true' },
  { value: 'false', label: 'settings.documentProcessing.visionOptions.false' },
];

interface SettingsFormProps {
  onBack: () => void;
}

function updateNestedSection(
  prev: AppConfig,
  section: NestedSection,
  key: string,
  value: unknown
): AppConfig {
  const sectionData = prev[section] as NestedConfig;
  return {
    ...prev,
    [section]: {
      ...sectionData,
      [key]: value,
    },
  };
}

export function SettingsForm({ onBack }: SettingsFormProps) {
  const { t } = useTranslation();
  const { config, setConfig } = useAppStore();
  const [localConfig, setLocalConfig] = useState<typeof config>(config);
  const [saving, setSaving] = useState(false);
  const [testing, setTesting] = useState(false);
  const [errors, setErrors] = useState<Record<string, string>>({});

  // Sync local config with store
  useEffect(() => {
    if (config) {
      setLocalConfig(config);
    }
  }, [config]);

  const handleChange = (section: NestedSection, key: string, value: string) => {
    setLocalConfig(prev => {
      if (!prev) return prev;
      return updateNestedSection(prev, section, key, value);
    });
    setErrors(prev => {
      const next = { ...prev };
      delete next[`${section}.${key}`];
      return next;
    });
  };

  const handleChangeNumber = (section: NestedSection | '', key: string, value: number) => {
    setLocalConfig(prev => {
      if (!prev) return prev;
      if (section === '') {
        return { ...prev, [key]: value };
      }
      return updateNestedSection(prev, section, key, value);
    });
  };

  const handleChangeBoolean = (section: NestedSection | '', key: string, value: boolean) => {
    setLocalConfig(prev => {
      if (!prev) return prev;
      if (section === '') {
        return { ...prev, [key]: value };
      }
      return updateNestedSection(prev, section, key, value);
    });
  };

  const handleChangeArray = (section: NestedSection, key: string, value: string[]) => {
    setLocalConfig(prev => {
      if (!prev) return prev;
      return updateNestedSection(prev, section, key, value);
    });
  };

  const collectUpdates = () => {
    const updates: { key: string; value: string }[] = [];
    if (!localConfig) return updates;

    // Compare with original config
    const original = config;
    if (!original) return updates;

    const checkNestedField = (section: NestedSection, key: string) => {
      const originalSection = original[section] as Record<string, unknown>;
      const localSection = localConfig[section] as Record<string, unknown>;
      const originalVal = originalSection[key];
      const localVal = localSection[key];
      if (originalVal !== localVal) {
        updates.push({
          key: `${section}.${key}`,
          value: Array.isArray(localVal) ? localVal.join(',') : String(localVal ?? ''),
        });
      }
    };

    const checkTopLevelField = (key: keyof AppConfig) => {
      const originalVal = original[key];
      const localVal = localConfig[key];
      if (originalVal !== localVal) {
        updates.push({
          key,
          value: String(localVal ?? ''),
        });
      }
    };

    // AI fields
    checkNestedField('ai', 'provider');
    checkNestedField('ai', 'api_key');
    checkNestedField('ai', 'model');
    checkNestedField('ai', 'gemini_model');
    checkNestedField('ai', 'base_url');
    checkNestedField('ai', 'gemini_base_url');
    checkNestedField('ai', 'custom_model');
    checkNestedField('ai', 'custom_base_url');
    checkNestedField('ai', 'ollama_base_url');
    checkNestedField('ai', 'temperature');
    checkNestedField('ai', 'timeout');
    checkNestedField('ai', 'system_prompt');

    // Document fields
    checkNestedField('document', 'vision');
    checkNestedField('document', 'vision_provider');
    checkNestedField('document', 'text_quality_threshold');

    // Naming fields
    checkNestedField('naming', 'template');
    checkNestedField('naming', 'fallback');
    checkNestedField('naming', 'date_format');
    checkNestedField('naming', 'separator');
    checkNestedField('naming', 'max_length');
    checkNestedField('naming', 'sequence_zerofill');
    checkNestedField('naming', 'primary_language');
    checkNestedField('naming', 'suggestion_languages');

    // Undo fields
    checkNestedField('undo', 'enabled');
    checkNestedField('undo', 'log_path');
    checkNestedField('undo', 'max_entries');

    // General fields
    checkTopLevelField('debug');
    checkTopLevelField('max_workers');

    return updates;
  };

  const handleSave = async () => {
    if (!localConfig) return;
    setSaving(true);
    setErrors({});

    const updates = collectUpdates();
    if (updates.length === 0) {
      showToast(t('toasts.configSaved', { count: 0 }), 'info');
      setSaving(false);
      return;
    }

    try {
      const result = await saveConfigBatch(updates);
      if (result.failed > 0 && result.saved > 0) {
        showToast(t('toasts.configSavePartial', { saved: result.saved, failed: result.failed }), 'warning');
        if (result.errors.length > 0) {
          setErrors({ general: result.errors[0] });
        }
      } else if (result.failed > 0) {
        showToast(t('toasts.configSaveFailed', { error: result.errors[0] || 'Unknown error' }), 'danger');
        setErrors({ general: result.errors[0] || 'Unknown error' });
      } else {
        showToast(t('toasts.configSaved', { count: result.saved }), 'success');
        // Reload config from backend
        const refreshed = await reloadConfig();
        setConfig(refreshed);
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      showToast(t('toasts.configSaveFailed', { error: message }), 'danger');
      setErrors({ general: message });
    } finally {
      setSaving(false);
    }
  };

  const handleTestConnection = async () => {
    if (!localConfig) return;
    setTesting(true);
    try {
      const currentProvider = localConfig.ai.provider;
      const apiKey = localConfig.ai.api_key;
      const modelKey = PROVIDER_MODEL_KEYS[currentProvider] || 'model';
      const model = String(localConfig.ai[modelKey] ?? '');

      const result = await testApiConnection(currentProvider, apiKey, model);
      if (result.success) {
        showToast(t('toasts.connectionTestSuccess', { provider: result.provider, latency: result.latency_ms }), 'success');
      } else {
        showToast(t('toasts.connectionTestFailed', { provider: result.provider, error: result.message }), 'danger');
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      showToast(t('toasts.connectionTestFailed', { provider: localConfig.ai.provider, error: message }), 'danger');
    } finally {
      setTesting(false);
    }
  };

  if (!localConfig) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" aria-label="Loading..." />
      </div>
    );
  }

  const provider = localConfig.ai.provider;
  const modelKey = PROVIDER_MODEL_KEYS[provider] || 'model';
  const providerSpecificFields = PROVIDER_FIELDS[provider] || [];

  return (
    <div className="max-w-4xl mx-auto space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold text-gray-900 dark:text-white">
          {t('settings.title')}
        </h2>
        <Button variant="secondary" onClick={onBack}>
          ← {t('common.back')}
        </Button>
      </div>

      {/* Provider Selector */}
      <section className="space-y-4" aria-labelledby="provider-heading">
        <h3 id="provider-heading" className="text-lg font-semibold text-gray-900 dark:text-white">
          {t('settings.sections.aiProvider')}
        </h3>
        <ProviderSelector
          currentProvider={localConfig.ai.provider}
          onChange={(nextProvider) => handleChange('ai', 'provider', nextProvider)}
        />
      </section>

      {/* Provider Specific Fields + Common AI Fields */}
      <section className="space-y-4" aria-labelledby="ai-fields-heading">
        <h3 id="ai-fields-heading" className="text-lg font-semibold text-gray-900 dark:text-white">
          {PROVIDERS.find(p => p.id === provider)?.label || t('settings.sections.aiProvider')}
        </h3>
        <div className="grid gap-4 md:grid-cols-2">
          {/* API Key */}
          <Input
            label={t('settings.aiProvider.apiKey')}
            type="password"
            value={localConfig.ai.api_key ?? ''}
            onChange={(e) => handleChange('ai', 'api_key', e.target.value)}
            placeholder="••••••••"
            error={errors['ai.api_key']}
            autoComplete="off"
          />

          {/* Model */}
          <Input
            label={t('settings.aiProvider.model')}
            value={String(localConfig.ai[modelKey] ?? '')}
            onChange={(e) => handleChange('ai', modelKey, e.target.value)}
            placeholder={provider === 'gemini' ? 'gemini-2.0-flash' : provider === 'openai' ? 'gpt-4o-mini' : 'llama3.2'}
            error={errors[`ai.${modelKey}`]}
          />

          {/* Provider-specific fields */}
          {providerSpecificFields.map((field) => (
            <Input
              key={field.key}
              label={t(field.label)}
              value={String(localConfig.ai[field.key] ?? '')}
              onChange={(e) => handleChange('ai', field.key, e.target.value)}
              placeholder={field.hint ? t(field.hint) : undefined}
              error={errors[`ai.${field.key}`]}
            />
          ))}

          {/* Temperature */}
          <Input
            label={t('settings.aiProvider.temperature')}
            type="number"
            min={0}
            max={2}
            step={0.1}
            value={String(localConfig.ai.temperature)}
            onChange={(e) => handleChangeNumber('ai', 'temperature', parseFloat(e.target.value) || 0)}
            error={errors['ai.temperature']}
          />

          {/* Timeout */}
          <Input
            label={t('settings.aiProvider.timeout')}
            type="number"
            min={5}
            max={600}
            value={String(localConfig.ai.timeout)}
            onChange={(e) => handleChangeNumber('ai', 'timeout', parseInt(e.target.value) || 30)}
            error={errors['ai.timeout']}
          />

          {/* System Prompt */}
          <Textarea
            label={t('settings.aiProvider.systemPrompt')}
            value={localConfig.ai.system_prompt ?? ''}
            onChange={(e) => handleChange('ai', 'system_prompt', e.target.value)}
            placeholder={t('settings.aiProvider.systemPromptHint')}
            rows={6}
            error={errors['ai.system_prompt']}
            className="md:col-span-2"
          />
        </div>
      </section>

      {/* Document Processing */}
      <section className="space-y-4" aria-labelledby="doc-heading">
        <h3 id="doc-heading" className="text-lg font-semibold text-gray-900 dark:text-white">
          {t('settings.sections.documentProcessing')}
        </h3>
        <div className="grid gap-4 md:grid-cols-2">
          <Select
            label={t('settings.documentProcessing.vision')}
            value={localConfig.document.vision}
            onChange={(e) => handleChange('document', 'vision', e.target.value)}
            options={VISION_OPTIONS.map(o => ({ value: o.value, label: t(o.label) }))}
            error={errors['document.vision']}
          />

          <Input
            label={t('settings.documentProcessing.visionProvider')}
            value={localConfig.document.vision_provider ?? ''}
            onChange={(e) => handleChange('document', 'vision_provider', e.target.value)}
            placeholder={t('settings.documentProcessing.visionProviderHint')}
            error={errors['document.vision_provider']}
          />

          <Input
            label={t('settings.documentProcessing.textQualityThreshold')}
            type="number"
            min={0}
            max={1}
            step={0.05}
            value={String(localConfig.document.text_quality_threshold)}
            onChange={(e) => handleChangeNumber('document', 'text_quality_threshold', parseFloat(e.target.value) || 0.2)}
            error={errors['document.text_quality_threshold']}
          />
        </div>
      </section>

      {/* File Naming */}
      <section className="space-y-4" aria-labelledby="naming-heading">
        <h3 id="naming-heading" className="text-lg font-semibold text-gray-900 dark:text-white">
          {t('settings.sections.fileNaming')}
        </h3>
        <div className="grid gap-4 md:grid-cols-2">
          <Input
            label={t('settings.fileNaming.template')}
            value={localConfig.naming.template ?? ''}
            onChange={(e) => handleChange('naming', 'template', e.target.value)}
            placeholder="{date}_{doctype}_{company}_{subject}"
            error={errors['naming.template']}
            className="md:col-span-2 font-mono text-sm"
          />

          <Input
            label={t('settings.fileNaming.fallback')}
            value={localConfig.naming.fallback ?? ''}
            onChange={(e) => handleChange('naming', 'fallback', e.target.value)}
            placeholder="{date}_{doctype}_{company}_Unknown"
            error={errors['naming.fallback']}
            className="md:col-span-2 font-mono text-sm"
          />

          <Input
            label={t('settings.fileNaming.dateFormat')}
            value={localConfig.naming.date_format ?? ''}
            onChange={(e) => handleChange('naming', 'date_format', e.target.value)}
            placeholder="%Y%m%d"
            error={errors['naming.date_format']}
          />

          <Input
            label={t('settings.fileNaming.separator')}
            value={localConfig.naming.separator ?? ''}
            onChange={(e) => handleChange('naming', 'separator', e.target.value)}
            placeholder="_"
            error={errors['naming.separator']}
          />

          <Input
            label={t('settings.fileNaming.maxLength')}
            type="number"
            min={16}
            max={255}
            value={String(localConfig.naming.max_length)}
            onChange={(e) => handleChangeNumber('naming', 'max_length', parseInt(e.target.value) || 128)}
            error={errors['naming.max_length']}
          />

          <Input
            label={t('settings.fileNaming.sequenceZerofill')}
            type="number"
            min={1}
            max={9}
            value={String(localConfig.naming.sequence_zerofill)}
            onChange={(e) => handleChangeNumber('naming', 'sequence_zerofill', parseInt(e.target.value) || 2)}
            error={errors['naming.sequence_zerofill']}
          />
        </div>
      </section>

      {/* AI Languages */}
      <section className="space-y-4" aria-labelledby="lang-heading">
        <h3 id="lang-heading" className="text-lg font-semibold text-gray-900 dark:text-white">
          {t('settings.sections.aiLanguages')}
        </h3>
        <div className="grid gap-4 md:grid-cols-2">
          <Input
            label={t('settings.aiLanguages.primaryLanguage')}
            value={localConfig.naming.primary_language ?? ''}
            onChange={(e) => handleChange('naming', 'primary_language', e.target.value)}
            placeholder="English"
            error={errors['naming.primary_language']}
          />

          <Input
            label={t('settings.aiLanguages.suggestionLanguages')}
            value={localConfig.naming.suggestion_languages?.join(', ') ?? ''}
            onChange={(e) => handleChangeArray('naming', 'suggestion_languages', e.target.value.split(',').map(s => s.trim()).filter(Boolean))}
            placeholder={t('settings.aiLanguages.suggestionLanguagesHint')}
            error={errors['naming.suggestion_languages']}
          />
        </div>
      </section>

      {/* Undo History */}
      <section className="space-y-4" aria-labelledby="undo-heading">
        <h3 id="undo-heading" className="text-lg font-semibold text-gray-900 dark:text-white">
          {t('settings.sections.undoHistory')}
        </h3>
        <div className="grid gap-4 md:grid-cols-2">
          <Toggle
            label={t('settings.undoHistory.enabled')}
            checked={localConfig.undo.enabled}
            onChange={(e) => handleChangeBoolean('undo', 'enabled', e.target.checked)}
          />

          <Input
            label={t('settings.undoHistory.logPath')}
            value={localConfig.undo.log_path ?? ''}
            onChange={(e) => handleChange('undo', 'log_path', e.target.value)}
            placeholder={t('settings.undoHistory.logPathHint')}
            error={errors['undo.log_path']}
          />

          <Input
            label={t('settings.undoHistory.maxEntries')}
            type="number"
            min={1}
            max={100000}
            value={String(localConfig.undo.max_entries)}
            onChange={(e) => handleChangeNumber('undo', 'max_entries', parseInt(e.target.value) || 100)}
            error={errors['undo.max_entries']}
          />
        </div>
      </section>

      {/* General */}
      <section className="space-y-4" aria-labelledby="general-heading">
        <h3 id="general-heading" className="text-lg font-semibold text-gray-900 dark:text-white">
          {t('settings.sections.general')}
        </h3>
        <div className="grid gap-4 md:grid-cols-2">
          <Toggle
            label={t('settings.general.debugMode')}
            checked={localConfig.debug}
            onChange={(e) => handleChangeBoolean('', 'debug', e.target.checked)}
          />

          <Input
            label={t('settings.general.maxWorkers')}
            type="number"
            min={1}
            max={32}
            value={String(localConfig.max_workers)}
            onChange={(e) => handleChangeNumber('', 'max_workers', parseInt(e.target.value) || 4)}
            error={errors['max_workers']}
          />
        </div>
      </section>

      {/* Footer Actions */}
      <div className="flex items-center justify-end gap-3 pt-6 border-t border-gray-200 dark:border-gray-700">
        <Button
          variant="secondary"
          onClick={handleTestConnection}
          disabled={testing || saving}
        >
          {testing ? (
            <>
              <svg className="animate-spin -ml-1 mr-2 h-4 w-4" fill="none" viewBox="0 0 24 24">
                <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
                <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
              </svg>
              {t('settings.testConnection')}
            </>
          ) : (
            t('settings.testConnection')
          )}
        </Button>
        <Button
          variant="primary"
          onClick={handleSave}
          disabled={saving}
        >
          {saving ? (
            <>
              <svg className="animate-spin -ml-1 mr-2 h-4 w-4" fill="none" viewBox="0 0 24 24">
                <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
                <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
              </svg>
              {t('settings.saveAll')}
            </>
          ) : (
            t('settings.saveAll')
          )}
        </Button>
      </div>
    </div>
  );
}
