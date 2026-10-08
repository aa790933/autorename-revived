// Sidebar navigation
import { useTranslation } from 'react-i18next';
import {
  FileText,
  Settings,
  History,
  Info,
  ChevronLeft,
  ChevronRight,
} from 'lucide-react';
import { useAppStore } from '@/store';
import { Button } from '@/components/ui';
import { getLanguageDirection, type SupportedLanguage } from '@/i18n';
import { AppView } from '@/types';
import clsx from 'clsx';

const NAV_ITEMS = [
  { id: 'files', icon: FileText, labelKey: 'navigation.files' },
  { id: 'settings', icon: Settings, labelKey: 'navigation.settings' },
  { id: 'history', icon: History, labelKey: 'navigation.history' },
  { id: 'about', icon: Info, labelKey: 'navigation.about' },
] as const;

export function Sidebar() {
  const { t } = useTranslation();
  const { view, setView, sidebarOpen, toggleSidebar, language } = useAppStore();
  const isRTL = getLanguageDirection(language as SupportedLanguage) === 'rtl';

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
            onClick={() => setView(id as AppView)}
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

</aside>
  );
}