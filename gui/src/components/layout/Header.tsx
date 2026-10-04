/**
 * Layout Components - Header / Title Bar
 */
import { useTranslation } from 'react-i18next';
import { Minimize, Maximize, X, Menu, Sun, Moon, Monitor } from 'lucide-react';
import { useAppStore } from '@/store';
import { Button } from '@/components/ui';
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

  const handleThemeChange = (newTheme: 'light' | 'dark' | 'system') => {
    setTheme(newTheme);
    if (newTheme === 'system') {
      const systemDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
      document.documentElement.classList.toggle('dark', systemDark);
    } else {
      document.documentElement.classList.toggle('dark', newTheme === 'dark');
    }
    localStorage.setItem('theme', newTheme);
  };

  return (
    <header
      className={`
        fixed top-0 right-0 z-30 h-12 bg-white/80 dark:bg-gray-900/80 backdrop-blur-sm 
        border-b border-gray-200 dark:border-gray-700
        flex items-center px-4
        ${sidebarOpen ? 'left-64' : 'left-16'}
        transition-all duration-300
      `}
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
            <Menu className="w-5 h-5" />
          </Button>
          
          <h1 className="text-lg font-semibold text-gray-900 dark:text-white hidden sm:block">
            {view === 'files' && t('files.title')}
            {view === 'settings' && t('settings.title')}
            {view === 'history' && t('history.title')}
            {view === 'about' && t('about.title')}
          </h1>
        </div>

        {/* Right side - Theme toggle and window controls */}
        <div className="flex items-center gap-2">
          {/* Theme Toggle */}
          <div className="flex items-center gap-1 bg-gray-100 dark:bg-gray-800 rounded-lg p-1">
            {[
              { value: 'light', icon: Sun },
              { value: 'dark', icon: Moon },
              { value: 'system', icon: Monitor },
            ].map(({ value, icon: Icon }) => (
              <button
                key={value}
                onClick={() => handleThemeChange(value as any)}
                className={`
                  p-1.5 rounded transition-colors
                  ${theme === value
                    ? 'bg-white dark:bg-gray-700 text-blue-600 dark:text-blue-400 shadow-sm'
                    : 'text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200'
                  }
                `}
                aria-label={t(`common.${value}`)}
                aria-pressed={theme === value}
              >
                <Icon className="w-4 h-4" />
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
              <Minimize className="w-4 h-4" />
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={handleMaximize}
              aria-label="Maximize"
              className="w-8 h-8 p-0"
            >
              <Maximize className="w-4 h-4" />
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={handleClose}
              aria-label="Close"
              className="w-8 h-8 p-0 text-gray-500 hover:text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20"
            >
              <X className="w-4 h-4" />
            </Button>
          </div>
        </div>
      </div>
    </header>
  );
}