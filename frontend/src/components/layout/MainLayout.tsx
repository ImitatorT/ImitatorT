import { useEffect } from 'react';
import { useAppStore } from '../../stores/appStore';
import { Toaster } from 'react-hot-toast';
import Sidebar from './Sidebar';

interface MainLayoutProps {
  children: React.ReactNode;
  onShowAuth: () => void;
}

export default function MainLayout({ children, onShowAuth }: MainLayoutProps) {
  const { theme, sidebarOpen, setSidebarOpen } = useAppStore();

  useEffect(() => {
    // Apply theme
    if (theme === 'dark' || (theme === 'auto' && window.matchMedia('(prefers-color-scheme: dark)').matches)) {
      document.documentElement.classList.add('dark');
    } else {
      document.documentElement.classList.remove('dark');
    }
  }, [theme]);

  return (
    <div className="h-screen w-screen overflow-hidden bg-[var(--tg-bg-color)] text-[var(--tg-text-color)]">
      <Toaster
        position="top-right"
        toastOptions={{
          style: {
            background: 'var(--tg-secondary-bg-color)',
            color: 'var(--tg-text-color)',
            border: '1px solid var(--tg-section-bg-color)',
          },
        }}
      />

      {/* Header with menu toggle button */}
      <header className="fixed top-0 left-0 right-0 h-16 bg-[var(--tg-bg-color)] border-b border-[var(--tg-section-bg-color)] z-40 flex items-center px-4">
        <button
          onClick={() => setSidebarOpen(true)}
          className="p-2 hover:bg-black/5 dark:hover:bg-white/5 rounded-lg transition-colors"
          aria-label="菜单"
        >
          <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 6h16M4 12h16M4 18h16" />
          </svg>
        </button>
        <div className="ml-4 text-lg font-semibold">ImitatorT</div>
      </header>

      {/* Sidebar */}
      <Sidebar
        isOpen={sidebarOpen}
        onClose={() => setSidebarOpen(false)}
        onShowAuth={onShowAuth}
      />

      {/* Main Content */}
      <main className="h-full w-full pt-16"> {/* Add padding to account for fixed header */}
        {children}
      </main>
    </div>
  );
}
