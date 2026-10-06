/**
 * Header / title bar
 */
import { useTranslation } from 'react-i18next';
import { Minimize, Maximize, X, Menu, Sun, Moon, Monitor } from 'lucide-react';
import { useAppStore } from '@/store';
import { Button } from '@/components/ui';
import { twMerge } from 'tailwind-merge';
import { getCurrentWindow } from '@tauri-apps/api/window';

export function Header() {
  const { t } = useTranslation();
  const { sidebarOpen, toggleSidebar, theme, setTheme, view } = useAppStore();

  const handleMinimize = async () => {
    const window = getCurrentWindow();
    await window.minimize();
  };

  const handleMaximize = async () => {
    const window = getCurrentWindow();
    await window.toggleMaximize();
  };

  const handleClose = async () => {
    const window = getCurrentWindow();
    await window.close();
  };

  return (
    <header
      className={twMerge(
        'fixed top-0 right-0 z-30 h-12 bg-white/80 dark:bg-neutral-900/80 backdrop-blur-sm',
        'border-b border-neutral-200 dark:border-neutral-800',
        'flex items-center px-4',
        sidebarOpen ? 'left-64' : 'left-16',
        'transition-all duration-300 ease-in-out'
      )}
      style={{ width: `calc(100% - ${sidebarOpen ? '16rem' : '4rem'})` }}
      data-tauri-drag-region
    >
      <div className="flex items-center justify-between w-full h-full">
        {/* Left side - Menu button and page title */}
        <div className="flex items-center gap-3">
          <Button
            variant="ghost"
            size="sm"
            onClick={toggleSidebar}
            aria-label={sidebarOpen ? t('accessibility.closeMenu') : t('accessibility.openMenu')}
            className="hidden sm:flex"
          >
            <Menu className="w-5 h-5" aria-hidden="true" />
          </Button>

          <h1 className="text-lg font-semibold text-neutral-900 dark:text-white hidden sm:block">
            {view === 'files' && t('files.title')}
            {view === 'settings' && t('settings.title')}
            {view === 'history' && t('history.title')}
            {view === 'about' && t('about.title')}
          </h1>
        </div>

        {/* Right side - Theme toggle and window controls */}
        <div className="flex items-center gap-2">
          {/* Theme Toggle */}
          <div className="flex items-center gap-1 bg-neutral-100 dark:bg-neutral-800 rounded-lg p-1">
            {[
              { value: 'light', icon: Sun },
              { value: 'dark', icon: Moon },
              { value: 'system', icon: Monitor },
            ].map(({ value, icon: Icon }) => (
              <button
                key={value}
                onClick={() => setTheme(value as any)}
                className={`p-1.5 rounded transition-colors
                  ${theme === value
                    ? 'bg-white dark:bg-neutral-700 text-primary-600 dark:text-primary-400 shadow-sm'
                    : 'text-neutral-500 dark:text-neutral-400 hover:text-neutral-700 dark:hover:text-neutral-200'
                  }`}
                aria-label={t(`common.${value}`)}
                aria-pressed={theme === value}
              >
                <Icon className="w-4 h-4" aria-hidden="true" />
              </button>
            ))}
          </div>

          {/* Window Controls */}
          <div className="flex items-center gap-1 ml-2" data-tauri-drag-region="false">
            <Button
              variant="ghost"
              size="sm"
              onClick={handleMinimize}
              aria-label="Minimize"
              className="w-8 h-8 p-0"
            >
              <Minimize className="w-4 h-4" aria-hidden="true" />
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={handleMaximize}
              aria-label="Maximize"
              className="w-8 h-8 p-0"
            >
              <Maximize className="w-4 h-4" aria-hidden="true" />
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={handleClose}
              aria-label="Close"
              className="w-8 h-8 p-0 text-neutral-500 hover:text-danger-500 hover:bg-danger-50 dark:hover:bg-danger-900/20"
            >
              <X className="w-4 h-4" aria-hidden="true" />
            </Button>
          </div>
        </div>
      </div>
    </header>
  );
}