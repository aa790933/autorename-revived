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
  const { setConfig, setLanguage } = useAppStore();

  // Initialize app on mount
  useEffect(() => {
    let mounted = true;

    const initialize = async () => {
      try {
        // Load config from backend
        const loadedConfig = await loadAppConfig();
        if (mounted) {
          setConfig(loadedConfig);
        }

        // Set language from config or localStorage
        const savedLang = localStorage.getItem('language') || loadedConfig?.naming?.primary_language?.toLowerCase() || 'en';
        if (savedLang && i18n.languages.includes(savedLang)) {
          await i18n.changeLanguage(savedLang);
          if (mounted) setLanguage(savedLang);
        }

        // Apply theme
        const savedTheme = localStorage.getItem('theme') || 'system';
        if (savedTheme === 'dark' || (savedTheme === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches)) {
          document.documentElement.classList.add('dark');
        } else {
          document.documentElement.classList.remove('dark');
        }

        // Set RTL direction for Arabic
        const currentLang = i18n.language;
        if (getLanguageDirection(currentLang as any) === 'rtl') {
          document.documentElement.dir = 'rtl';
        } else {
          document.documentElement.dir = 'ltr';
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
  }, [i18n, setConfig, setLanguage]);

  // Listen for language changes
  useEffect(() => {
    const handleLanguageChange = (lng: string) => {
      if (getLanguageDirection(lng as any) === 'rtl') {
        document.documentElement.dir = 'rtl';
      } else {
        document.documentElement.dir = 'ltr';
      }
      localStorage.setItem('language', lng);
    };

    i18n.on('languageChanged', handleLanguageChange);
    return () => i18n.off('languageChanged', handleLanguageChange);
  }, [i18n]);

  return (
    <Layout>
      <ViewContent />
    </Layout>
  );
}