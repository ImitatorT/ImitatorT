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
      
      {/* Sidebar */}
      <Sidebar 
        isOpen={sidebarOpen} 
        onClose={() => setSidebarOpen(false)}
        onShowAuth={onShowAuth}
      />
      
      {/* Main Content */}
      <main className="h-full w-full">
        {children}
      </main>
    </div>
  );
}
