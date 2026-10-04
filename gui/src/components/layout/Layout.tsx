/**
 * Layout Components - Main Layout Wrapper
 */
import { ReactNode } from 'react';
import { Sidebar } from './Sidebar';
import { Header } from './Header';
import { Toaster } from './Toaster';

interface LayoutProps {
  children: ReactNode;
}

export function Layout({ children }: LayoutProps) {
  return (
    <div className="min-h-screen bg-gray-50 dark:bg-gray-950">
      <Sidebar />
      <Header />
      <main
        className={`
          pt-12 min-h-screen transition-all duration-300
          lg:pl-64
        `}
        style={{ marginLeft: '4rem' }}
      >
        <div className="p-4 lg:p-6">
          {children}
        </div>
      </main>
      <Toaster />
    </div>
  );
}