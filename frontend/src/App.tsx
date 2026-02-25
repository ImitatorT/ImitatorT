import { useEffect, useState } from 'react';
import { useAppStore } from './stores/appStore';
import { useAuthStore } from './stores/authStore';
import { useIsMobile } from './hooks/useMediaQuery';
import MainLayout from './components/layout/MainLayout';
import ChatContainer from './components/chat/ChatContainer';
import OrgTree from './components/org/OrgTree';
import BoardPanel from './components/board/BoardPanel';
import BoardManagement from './components/board/BoardManagement';
import UserProfile from './components/user/UserProfile';
import AuthModal from './components/auth/AuthModal';
import { Loader2 } from 'lucide-react';

function App() {
  const { setIsMobile, setSidebarOpen } = useAppStore();
  const { user, restoreSession } = useAuthStore();
  const isMobile = useIsMobile();
  const [hash, setHash] = useState(window.location.hash || '#chat');
  const [showAuthModal, setShowAuthModal] = useState(false);
  const [isInitializing, setIsInitializing] = useState(true);

  // 响应式处理
  useEffect(() => {
    setIsMobile(isMobile);
    if (isMobile) {
      setSidebarOpen(false);
    }
  }, [isMobile, setIsMobile, setSidebarOpen]);

  // 监听 hash 变化
  useEffect(() => {
    const handleHashChange = () => {
      setHash(window.location.hash || '#chat');
    };

    window.addEventListener('hashchange', handleHashChange);
    return () => window.removeEventListener('hashchange', handleHashChange);
  }, []);

  // 应用初始化：恢复登录态
  useEffect(() => {
    const init = async () => {
      console.log('[App] Initializing...');
      
      // 等待 store 从 localStorage 恢复
      await new Promise(resolve => setTimeout(resolve, 50));
      
      // 尝试恢复会话
      await restoreSession();
      
      setIsInitializing(false);
      console.log('[App] Initialized');
    };
    
    init();
  }, [restoreSession]);

  // 检查用户权限
  const canAccessChat = user?.is_director || false;

  const renderContent = () => {
    switch (hash) {
      case '#org':
        return canAccessChat ? <OrgTree /> : <AccessDenied onShowAuth={() => setShowAuthModal(true)} />;
      case '#board':
        return <BoardPanel />;
      case '#board-mgmt':
        return <BoardManagement />;
      case '#profile':
        return <UserProfile />;
      case '#chat':
      default:
        return canAccessChat ? <ChatContainer /> : <AccessDenied onShowAuth={() => setShowAuthModal(true)} />;
    }
  };

  // 初始化加载中
  if (isInitializing) {
    return (
      <div className="h-screen w-screen flex items-center justify-center bg-[var(--tg-bg-color)]">
        <div className="flex flex-col items-center gap-4">
          <Loader2 className="w-10 h-10 animate-spin text-[var(--tg-button-color)]" />
          <p className="text-[var(--tg-hint-color)]">加载中...</p>
        </div>
      </div>
    );
  }

  return (
    <>
      <MainLayout onShowAuth={() => setShowAuthModal(true)}>
        {renderContent()}
      </MainLayout>
      <AuthModal 
        isOpen={showAuthModal} 
        onClose={() => setShowAuthModal(false)} 
      />
    </>
  );
}

// Access Denied Component
function AccessDenied({ onShowAuth }: { onShowAuth: () => void }) {
  return (
    <div className="h-full flex items-center justify-center p-4">
      <div className="max-w-md w-full text-center">
        <div className="w-20 h-20 mx-auto mb-6 rounded-full bg-[var(--tg-secondary-bg-color)] flex items-center justify-center">
          <svg className="w-10 h-10 text-[var(--tg-hint-color)]" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
          </svg>
        </div>
        <h2 className="text-xl font-bold text-[var(--tg-text-color)] mb-2">
          需要董事权限
        </h2>
        <p className="text-[var(--tg-hint-color)] mb-6">
          只有董事可以查看 Agent 聊天内容。请先登录，或联系董事局主席申请董事权限。
        </p>
        <div className="flex gap-3 justify-center">
          <button
            onClick={onShowAuth}
            className="px-6 py-2 bg-[var(--tg-button-color)] text-[var(--tg-button-text-color)] rounded-lg font-medium hover:opacity-90 transition-opacity"
          >
            登录 / 注册
          </button>
          <a
            href="#profile"
            className="px-6 py-2 bg-[var(--tg-secondary-bg-color)] text-[var(--tg-text-color)] rounded-lg font-medium hover:bg-[var(--tg-section-bg-color)] transition-colors"
          >
            查看个人信息
          </a>
        </div>
      </div>
    </div>
  );
}

export default App;
