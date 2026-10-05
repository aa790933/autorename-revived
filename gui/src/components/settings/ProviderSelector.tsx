/**
 * AI provider selector
 */

import { useTranslation } from 'react-i18next';
import { twMerge } from 'tailwind-merge';

export const PROVIDERS = [
  {
    id: 'gemini',
    label: 'Google Gemini',
    icon: (
      <svg viewBox="0 0 24 24" fill="currentColor" className="w-5 h-5" aria-hidden="true">
        <path d="M12 2L2 19.5h20L12 2zm0 4l6.5 11.5h-13L12 6z" />
      </svg>
    ),
  },
  {
    id: 'openai',
    label: 'OpenAI',
    icon: (
      <svg viewBox="0 0 24 24" fill="currentColor" className="w-5 h-5" aria-hidden="true">
        <circle cx="12" cy="12" r="10" />
      </svg>
    ),
  },
  {
    id: 'anthropic',
    label: 'Anthropic',
    icon: (
      <svg viewBox="0 0 24 24" fill="currentColor" className="w-5 h-5" aria-hidden="true">
        <path d="M12 3l9 18h-4.5l-1.8-3.9H9.3L7.5 21H3L12 3zm0 6.4l-2 4.3h4l-2-4.3z" />
      </svg>
    ),
  },
  {
    id: 'ollama',
    label: 'Ollama (Local)',
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" className="w-5 h-5" aria-hidden="true">
        <rect x="3" y="4" width="18" height="16" rx="3" />
        <circle cx="12" cy="12" r="3" />
      </svg>
    ),
  },
  {
    id: 'xai',
    label: 'xAI',
    icon: (
      <svg viewBox="0 0 24 24" fill="currentColor" className="w-5 h-5" aria-hidden="true">
        <path d="M3 3h4.2l4.3 6 4.4-6H21l-6.6 8.9L21 21h-4.2l-4.4-6.1L8 21H3l6.8-9.1L3 3z" />
      </svg>
    ),
  },
  {
    id: 'custom',
    label: 'Custom / vLLM',
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" className="w-5 h-5" aria-hidden="true">
        <path d="M12 2v20M2 12h20" />
      </svg>
    ),
  },
];

interface ProviderSelectorProps {
  currentProvider: string;
  onChange: (provider: string) => void;
}

export function ProviderSelector({ currentProvider, onChange }: ProviderSelectorProps) {
  const { t } = useTranslation();

  return (
    <div className="space-y-3">
      <label className="block text-sm font-medium text-neutral-700 dark:text-neutral-300">
        {t('settings.sections.aiProvider')}
      </label>
      <div className="flex flex-wrap gap-2" role="radiogroup" aria-label={t('settings.sections.aiProvider')}>
        {PROVIDERS.map(({ id, label, icon }) => (
          <button
            key={id}
            type="button"
            onClick={() => onChange(id)}
            className={twMerge(
              'flex items-center gap-2 px-4 py-2.5 rounded-xl border-2 transition-all duration-200',
              'focus:outline-none focus-visible:ring-2 focus-visible:ring-primary-500 focus-visible:ring-offset-2',
              currentProvider === id
                ? 'border-primary-500 bg-primary-50 dark:bg-primary-900/20 text-primary-700 dark:text-primary-300'
                : 'border-neutral-200 dark:border-neutral-700 hover:border-primary-300 dark:hover:border-primary-600 text-neutral-700 dark:text-neutral-300'
            )}
            role="radio"
            aria-checked={currentProvider === id}
            aria-label={label}
          >
            <span className="flex-shrink-0">{icon}</span>
            <span className="font-medium">{label}</span>
          </button>
        ))}
      </div>
    </div>
  );
}