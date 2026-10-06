/**
 * Main App Component
 */
import { useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { Layout } from '@/components/layout';
import { FileList } from '@/components/file-list';
import { SettingsForm } from '@/components/settings';
import { HistoryView } from '@/components/views';
import { AboutView } from '@/components/views';
import { useAppStore } from '@/store';
import { loadAppConfig, validateConfig } from '@/services/api';
import { showToast } from '@/hooks/useToast';
import { t } from '@/hooks/useTranslation';
import { getLanguageDirection } from '@/i18n';

function ViewContent() {
  const { view } = useAppStore();

  switch (view) {
    case 'files':
      return <FileList />;
    case 'settings':
      return (
        <SettingsForm
          onBack={() => useAppStore.getState().setView('files')}
        />
      );
    case 'history':
      return <HistoryView />;
    case 'about':
      return <AboutView />;
    default:
      return <FileList />;
  }
}

export function App() {
  const { i18n } = useTranslation();
  const { setConfig, setLanguage, language, theme, setTheme } = useAppStore();

  // Initialize app on mount
  useEffect(() => {
    let mounted = true;

    const initialize = async () => {
      try {
        // Load config from backend
        const loadedConfig = await loadAppConfig();
        if (mounted) {
          setConfig(loadedConfig);
          
          // Initial language sync
          const savedLang = localStorage.getItem('language') || loadedConfig?.naming?.primary_language?.toLowerCase() || 'en';
          if (savedLang && i18n.languages.includes(savedLang)) {
            setLanguage(savedLang);
          }

          // Initial theme sync
          const savedTheme = localStorage.getItem('theme') || 'system' as any;
          setTheme(savedTheme);
        }

        // Validate config after a short delay
        setTimeout(async () => {
          try {
            const validation = await validateConfig();
            if (!validation.valid) {
              const errors = validation.issues.filter(i => i.level === 'error');
              if (errors.length > 0) {
                showToast(t('toasts.configErrors', { errors: errors.map(e => e.message).join(', ') }), 'warning');
              }
            }
          } catch {
            // Ignore validation errors on startup
          }
        }, 1000);
      } catch (error) {
        console.error('Failed to initialize app:', error);
        showToast(t('errors.configLoadError'), 'danger');
      }
    };

    initialize();

    return () => {
      mounted = false;
    };
  }, [setConfig, setLanguage, setTheme, t]);

  // Reactive Language Effect
  useEffect(() => {
    if (language) {
      i18n.changeLanguage(language);
      document.documentElement.dir = getLanguageDirection(language as any);
      localStorage.setItem('language', language);
    }
  }, [language, i18n]);

  // Reactive Theme Effect
  useEffect(() => {
    const applyTheme = (t: string) => {
      if (t === 'dark' || (t === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches)) {
        document.documentElement.classList.add('dark');
      } else {
        document.documentElement.classList.remove('dark');
      }
    };

    applyTheme(theme);
    localStorage.setItem('theme', theme);

    if (theme === 'system') {
      const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
      const listener = (e: MediaQueryListEvent) => applyTheme(e.matches ? 'dark' : 'light');
      mediaQuery.addEventListener('change', listener);
      return () => mediaQuery.removeEventListener('change', listener);
    }
  }, [theme]);
  return (
    <Layout>
      <ViewContent />
    </Layout>
  );
}