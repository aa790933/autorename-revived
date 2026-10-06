/**
 * Main layout wrapper
 */
import { ReactNode } from 'react';
import { twMerge } from 'tailwind-merge';
import { Sidebar } from './Sidebar';
import { Header } from './Header';
import { Toaster } from './Toaster';
import { useAppStore } from '@/store';

interface LayoutProps {
  children: ReactNode;
}

export function Layout({ children }: LayoutProps) {
  const sidebarOpen = useAppStore(s => s.sidebarOpen);

  return (
    <div className="min-h-screen bg-neutral-50 dark:bg-neutral-950">
      <Sidebar />
      <Header />
      <main
        className={twMerge(
          'pt-12 min-h-screen transition-all duration-300 ease-in-out',
          sidebarOpen ? 'pl-64' : 'pl-16'
        )}
      >
        <div className="p-4 lg:p-6 max-w-full">
          {children}
        </div>
      </main>
      <Toaster />
    </div>
  );
}