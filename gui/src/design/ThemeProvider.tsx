/**
 * Theme Provider for AutoRename-Revived v4.0.0
 * Handles light/dark/system themes with CSS variables
 */

import { createContext, useContext, useEffect, useState, ReactNode } from 'react';
import { useAppStore } from '@/store';

type ThemeMode = 'light' | 'dark' | 'system';

interface ThemeContextType {
  theme: ThemeMode;
  resolvedTheme: 'light' | 'dark';
  setTheme: (theme: ThemeMode) => void;
  toggleTheme: () => void;
}

const ThemeContext = createContext<ThemeContextType | undefined>(undefined);

export function ThemeProvider({ children }: { children: ReactNode }) {
  const { theme, setTheme: setStoreTheme, sidebarOpen } = useAppStore();
  const [resolvedTheme, setResolvedTheme] = useState<'light' | 'dark'>('light');
  const [mounted, setMounted] = useState(false);

  useEffect(() => {
    setMounted(true);
    const stored = localStorage.getItem('theme') as ThemeMode | null;
    if (stored) {
      setStoreTheme(stored);
    } else {
      // Detect system preference on first load
      const systemDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
      setStoreTheme('system');
      setResolvedTheme(systemDark ? 'dark' : 'light');
    }
  }, [setStoreTheme]);

  useEffect(() => {
    if (!mounted) return;

    const updateResolvedTheme = () => {
      let isDark = false;
      if (theme === 'system') {
        isDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
      } else {
        isDark = theme === 'dark';
      }
      setResolvedTheme(isDark ? 'dark' : 'light');
      document.documentElement.classList.toggle('dark', isDark);
    };

    updateResolvedTheme();

    if (theme === 'system') {
      const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
      mediaQuery.addEventListener('change', updateResolvedTheme);
      return () => mediaQuery.removeEventListener('change', updateResolvedTheme);
    }
  }, [theme, mounted]);

  const handleSetTheme = (newTheme: ThemeMode) => {
    setStoreTheme(newTheme);
    localStorage.setItem('theme', newTheme);
  };

  const toggleTheme = () => {
    const themes: ThemeMode[] = ['light', 'dark', 'system'];
    const currentIndex = themes.indexOf(theme);
    const nextTheme = themes[(currentIndex + 1) % themes.length];
    handleSetTheme(nextTheme);
  };

  if (!mounted) {
    return (
      <div className="min-h-screen bg-gray-50 dark:bg-gray-950" data-theme="loading">
        {children}
      </div>
    );
  }

  return (
    <ThemeContext.Provider value={{ theme, resolvedTheme, setTheme: handleSetTheme, toggleTheme }}>
      <div className={`min-h-screen bg-neutral-50 dark:bg-neutral-950 transition-colors duration-200`} data-theme={resolvedTheme}>
        {children}
      </div>
    </ThemeContext.Provider>
  );
}

export function useTheme() {
  const context = useContext(ThemeContext);
  if (!context) {
    throw new Error('useTheme must be used within a ThemeProvider');
  }
  return context;
}