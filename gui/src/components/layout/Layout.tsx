/**
 * Main layout wrapper
 */
import { ReactNode } from 'react';
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
        className={`
          pt-12 min-h-screen transition-all duration-300
          lg:pl-64
        `}
        style={{ marginLeft: sidebarOpen ? '16rem' : '4rem' }}
      >
        <div className="p-4 lg:p-6">
          {children}
        </div>
      </main>
      <Toaster />
    </div>
  );
}