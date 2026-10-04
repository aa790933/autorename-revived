/**
 * Layout Components - Sidebar Navigation
 */
import { useTranslation } from 'react-i18next';
import { 
  FileText, 
  Settings, 
  History, 
  Info, 
  ChevronLeft, 
  ChevronRight,
  Moon,
  Sun,
  Monitor,
  Globe
} from 'lucide-react';
import { useAppStore } from '@/store';
import { Button } from '@/components/ui';
import { SUPPORTED_LANGUAGES, getLanguageDirection } from '@/i18n';
import { useTranslation as useTranslationHook } from '@/hooks/useTranslation';

const NAV_ITEMS = [
  { id: 'files', icon: FileText, labelKey: 'navigation.files' },
  { id: 'settings', icon: Settings, labelKey: 'navigation.settings' },
  { id: 'history', icon: History, labelKey: 'navigation.history' },
  { id: 'about', icon: Info, labelKey: 'navigation.about' },
] as const;

export function Sidebar() {
  const { t } = useTranslation();
  const { view, setView, sidebarOpen, toggleSidebar, theme, setTheme, language, setLanguage } = useAppStore();
  const { i18n } = useTranslationHook();
  const isRTL = getLanguageDirection(language as any) === 'rtl';

  const handleThemeChange = (newTheme: 'light' | 'dark' | 'system') => {
    setTheme(newTheme);
    if (newTheme === 'system') {
      // Apply system preference
      const systemDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
      document.documentElement.classList.toggle('dark', systemDark);
    } else {
      document.documentElement.classList.toggle('dark', newTheme === 'dark');
    }
    localStorage.setItem('theme', newTheme);
  };

  return (
    <aside
      className={`
        fixed left-0 top-0 z-40 h-full bg-white dark:bg-gray-900 border-r border-gray-200 dark:border-gray-700
        transition-all duration-300 ease-in-out flex flex-col
        ${sidebarOpen ? 'w-64' : 'w-16'}
      `}
      aria-label={t('accessibility.openMenu')}
    >
      {/* Logo / Brand */}
      <div className="flex items-center justify-between h-16 px-4 border-b border-gray-200 dark:border-gray-700">
        <div className="flex items-center gap-3">
          <div className="w-8 h-8 rounded-lg bg-blue-600 flex items-center justify-center">
            <FileText className="w-5 h-5 text-white" />
          </div>
          {sidebarOpen && (
            <span className="font-bold text-lg text-gray-900 dark:text-white">
              AutoRename
            </span>
          )}
        </div>
        <Button
          variant="ghost"
          size="sm"
          onClick={toggleSidebar}
          aria-label={sidebarOpen ? t('accessibility.closeMenu') : t('accessibility.openMenu')}
          className="ml-auto"
        >
          {isRTL ? (
            sidebarOpen ? <ChevronRight className="w-5 h-5" /> : <ChevronLeft className="w-5 h-5" />
          ) : (
            sidebarOpen ? <ChevronLeft className="w-5 h-5" /> : <ChevronRight className="w-5 h-5" />
          )}
        </Button>
      </div>

      {/* Navigation */}
      <nav className="flex-1 px-2 py-4 space-y-1 overflow-y-auto" aria-label={t('accessibility.openMenu')}>
        {NAV_ITEMS.map(({ id, icon: Icon, labelKey }) => (
          <button
            key={id}
            onClick={() => setView(id as any)}
            className={`
              w-full flex items-center gap-3 px-3 py-2.5 rounded-lg transition-all duration-200
              ${view === id 
                ? 'bg-blue-50 dark:bg-blue-900/20 text-blue-600 dark:text-blue-400' 
                : 'text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800'
              }
              ${!sidebarOpen ? 'justify-center' : ''}
            `}
            aria-current={view === id ? 'page' : undefined}
            title={sidebarOpen ? undefined : t(labelKey)}
          >
            <Icon className="w-5 h-5 flex-shrink-0" aria-hidden="true" />
            {sidebarOpen && <span className="font-medium">{t(labelKey)}</span>}
          </button>
        ))}
      </nav>

      {/* Bottom section - Theme & Language */}
      <div className="p-4 border-t border-gray-200 dark:border-gray-700 space-y-4">
        {/* Theme Selector */}
        {sidebarOpen && (
          <div>
            <p className="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider mb-2">
              {t('common.theme')}
            </p>
            <div className="grid grid-cols-3 gap-2">
              {[
                { value: 'light', label: t('common.light'), icon: Sun },
                { value: 'dark', label: t('common.dark'), icon: Moon },
                { value: 'system', label: t('common.system'), icon: Monitor },
              ].map(({ value, label, icon: Icon }) => (
                <button
                  key={value}
                  onClick={() => handleThemeChange(value as any)}
                  className={`
                    flex flex-col items-center gap-1.5 px-3 py-2 rounded-lg text-sm transition-all
                    ${theme === value
                      ? 'bg-blue-50 dark:bg-blue-900/20 text-blue-600 dark:text-blue-400 border border-blue-200 dark:border-blue-800'
                      : 'text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800'
                    }
                  `}
                  aria-pressed={theme === value}
                >
                  <Icon className="w-5 h-5" aria-hidden="true" />
                  <span>{label}</span>
                </button>
              ))}
            </div>
          </div>
        )}

        {/* Language Selector */}
        {sidebarOpen && (
          <div>
            <p className="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider mb-2">
              {t('common.language')}
            </p>
            <div className="space-y-1">
              {SUPPORTED_LANGUAGES.map(({ code, nativeName }) => (
                <button
                  key={code}
                  onClick={() => {
                    setLanguage(code);
                    i18n.changeLanguage(code);
                  }}
                  className={`
                    w-full flex items-center gap-3 px-3 py-2 rounded-lg text-sm transition-all
                    ${language === code
                      ? 'bg-blue-50 dark:bg-blue-900/20 text-blue-600 dark:text-blue-400 border border-blue-200 dark:border-blue-800'
                      : 'text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800'
                    }
                  `}
                  aria-pressed={language === code}
                >
                  <Globe className="w-5 h-5 flex-shrink-0" aria-hidden="true" />
                  <span>{nativeName}</span>
                  {language === code && (
                    <svg className="w-4 h-4 ml-auto text-blue-600 dark:text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M5 13l4 4L19 7" />
                    </svg>
                  )}
                </button>
              ))}
            </div>
          </div>
        )}
      </div>
    </aside>
  );
}