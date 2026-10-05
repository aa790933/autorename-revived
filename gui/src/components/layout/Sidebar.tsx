/**
 * Modern Sidebar Navigation - AutoRename-Revived v4.0.0
 * Collapsible sidebar with language/theme controls
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
  Globe,
} from 'lucide-react';
import { useAppStore } from '@/store';
import { Button } from '@/components/ui';
import { SUPPORTED_LANGUAGES, getLanguageDirection } from '@/i18n';
import { useTheme } from '@/design/ThemeProvider';
import clsx from 'clsx';

const NAV_ITEMS = [
  { id: 'files', icon: FileText, labelKey: 'navigation.files' },
  { id: 'settings', icon: Settings, labelKey: 'navigation.settings' },
  { id: 'history', icon: History, labelKey: 'navigation.history' },
  { id: 'about', icon: Info, labelKey: 'navigation.about' },
] as const;

const THEME_OPTIONS = [
  { value: 'light', icon: Sun },
  { value: 'dark', icon: Moon },
  { value: 'system', icon: Monitor },
] as const;

export function Sidebar() {
  const { t } = useTranslation();
  const { view, setView, sidebarOpen, toggleSidebar, theme, setTheme, language } = useAppStore();
  const { theme: resolvedTheme, setTheme: setResolvedTheme } = useTheme();
  const isRTL = getLanguageDirection(language as any) === 'rtl';

  return (
    <aside
      className={clsx(
        'fixed left-0 top-0 z-40 h-full bg-white dark:bg-neutral-900 border-r border-neutral-200 dark:border-neutral-800',
        'transition-all duration-300 ease-in-out flex flex-col',
        sidebarOpen ? 'w-64' : 'w-16'
      )}
      aria-label={t('accessibility.openMenu')}
    >
      {/* Logo / Brand */}
      <div className="flex items-center justify-between h-16 px-4 border-b border-neutral-200 dark:border-neutral-800">
        <div className="flex items-center gap-3">
          <div className="w-8 h-8 rounded-lg bg-primary-600 flex items-center justify-center shadow-sm">
            <FileText className="w-5 h-5 text-white" aria-hidden="true" />
          </div>
          {sidebarOpen && (
            <div className="flex flex-col">
              <span className="font-bold text-sm text-neutral-900 dark:text-neutral-100 leading-none">
                AutoRename
              </span>
              <span className="text-[10px] text-neutral-500 dark:text-neutral-400 mt-0.5">
                v4.0.0
              </span>
            </div>
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
      <nav className="flex-1 px-2 py-4 space-y-1 overflow-y-auto" aria-label={t('navigation.files')}>
        {NAV_ITEMS.map(({ id, icon: Icon, labelKey }) => (
          <button
            key={id}
            onClick={() => setView(id as any)}
            className={clsx(
              'w-full flex items-center gap-3 px-3 py-2.5 rounded-xl transition-all duration-200',
              view === id
                ? 'bg-primary-50 dark:bg-primary-900/20 text-primary-700 dark:text-primary-400 shadow-sm'
                : 'text-neutral-600 dark:text-neutral-400 hover:bg-neutral-100 dark:hover:bg-neutral-800',
              !sidebarOpen && 'justify-center'
            )}
            aria-current={view === id ? 'page' : undefined}
            title={sidebarOpen ? undefined : t(labelKey)}
          >
            <Icon className="w-5 h-5 flex-shrink-0" aria-hidden="true" />
            {sidebarOpen && <span className="font-medium text-sm">{t(labelKey)}</span>}
          </button>
        ))}
      </nav>

      {/* Bottom section - Theme & Language */}
      <div className="p-4 border-t border-neutral-200 dark:border-neutral-800 space-y-4">
        {/* Theme Selector */}
        {sidebarOpen && (
          <div>
            <p className="text-xs font-medium text-neutral-500 dark:text-neutral-400 uppercase tracking-wider mb-2">
              {t('common.theme')}
            </p>
            <div className="grid grid-cols-3 gap-2">
              {THEME_OPTIONS.map(({ value, icon: Icon }) => (
                <button
                  key={value}
                  onClick={() => setTheme(value)}
                  className={clsx(
                    'flex flex-col items-center gap-1.5 px-3 py-2 rounded-xl text-sm transition-all',
                    theme === value
                      ? 'bg-primary-50 dark:bg-primary-900/20 text-primary-700 dark:text-primary-400 border border-primary-200 dark:border-primary-800 shadow-sm'
                      : 'text-neutral-600 dark:text-neutral-400 hover:bg-neutral-100 dark:hover:bg-neutral-800'
                  )}
                  aria-pressed={theme === value}
                  aria-label={t(`common.${value}`)}
                >
                  <Icon className="w-5 h-5" aria-hidden="true" />
                  <span className="text-xs font-medium">{t(`common.${value}`)}</span>
                </button>
              ))}
            </div>
          </div>
        )}

        {/* Language Selector */}
        {sidebarOpen && (
          <div>
            <p className="text-xs font-medium text-neutral-500 dark:text-neutral-400 uppercase tracking-wider mb-2">
              {t('common.language')}
            </p>
            <div className="space-y-1">
              {SUPPORTED_LANGUAGES.map(({ code, nativeName }) => (
                <button
                  key={code}
                  onClick={() => {
                    useAppStore.getState().setLanguage(code);
                  }}
                  className={clsx(
                    'w-full flex items-center gap-3 px-3 py-2 rounded-xl text-sm transition-all',
                    language === code
                      ? 'bg-primary-50 dark:bg-primary-900/20 text-primary-700 dark:text-primary-400 border border-primary-200 dark:border-primary-800 shadow-sm'
                      : 'text-neutral-600 dark:text-neutral-400 hover:bg-neutral-100 dark:hover:bg-neutral-800'
                  )}
                  aria-pressed={language === code}
                >
                  <Globe className="w-5 h-5 flex-shrink-0" aria-hidden="true" />
                  <span>{nativeName}</span>
                  {language === code && (
                    <svg className="w-4 h-4 ml-auto text-primary-600 dark:text-primary-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
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