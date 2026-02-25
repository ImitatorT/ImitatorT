import { useState, useEffect } from 'react';
import { cn } from '../../utils/helpers';
import { 
  MessageCircle, 
  Building2, 
  Crown, 
  Moon, 
  Sun, 
  X,
  LogOut,
  User,
  Shield
} from 'lucide-react';
import { useAppStore, useBoardStore } from '../../stores/appStore';
import { useAuthStore } from '../../stores/authStore';
import { toast } from 'react-hot-toast';

interface SidebarProps {
  isOpen: boolean;
  onClose: () => void;
  onShowAuth: () => void;
}

export default function Sidebar({ isOpen, onClose, onShowAuth }: SidebarProps) {
  const { theme, setTheme } = useAppStore();
  const { isLoggedIn: isBoardLoggedIn, currentUser: boardUser, logout: boardLogout } = useBoardStore();
  const { isLoggedIn: isAuthLoggedIn, user: authUser, logout: authLogout } = useAuthStore();
  const [activeItem, setActiveItem] = useState(window.location.hash || '#chat');

  // 监听 hash 变化
  useEffect(() => {
    const handleHashChange = () => {
      setActiveItem(window.location.hash || '#chat');
    };
    window.addEventListener('hashchange', handleHashChange);
    return () => window.removeEventListener('hashchange', handleHashChange);
  }, []);

  const isLoggedIn = isBoardLoggedIn || isAuthLoggedIn;
  const currentUser = boardUser || (authUser ? {
    id: authUser.id,
    username: authUser.username,
    name: authUser.name,
    is_chairman: false,
    created_at: authUser.created_at || Date.now(),
  } : null);
  const isDirector = authUser?.is_director || false;
  const isChairman = boardUser?.is_chairman || false;

  const handleLogout = () => {
    boardLogout();
    authLogout();
    toast.success('已退出登录');
    onClose();
  };

  const navItems = [
    { icon: MessageCircle, label: '消息', href: '#chat', requireDirector: true },
    { icon: Building2, label: '组织架构', href: '#org', requireDirector: true },
    ...(isBoardLoggedIn ? [{ icon: Crown, label: '董事局', href: '#board' }] : []),
    ...(isChairman ? [{ icon: Shield, label: '董事局管理', href: '#board-mgmt' }] : []),
  ];

  const handleNavClick = (href: string, requireDirector?: boolean) => {
    if (requireDirector && !isDirector && !isBoardLoggedIn) {
      toast.error('只有董事可以查看此页面');
      onShowAuth();
      return;
    }
    window.location.hash = href;
    onClose();
  };

  return (
    <>
      {/* Backdrop */}
      {isOpen && (
        <div 
          className="fixed inset-0 bg-black/50 z-40 lg:hidden"
          onClick={onClose}
        />
      )}

      {/* Sidebar Drawer */}
      <div
        className={cn(
          'fixed left-0 top-0 bottom-0 w-[280px] bg-[var(--tg-bg-color)] z-50',
          'transform transition-transform duration-300 ease-out',
          'shadow-2xl',
          isOpen ? 'translate-x-0' : '-translate-x-full'
        )}
      >
        {/* Header */}
        <div className="h-16 flex items-center justify-between px-4 border-b border-[var(--tg-section-bg-color)]">
          <span className="text-xl font-bold">ImitatorT</span>
          <button 
            onClick={onClose}
            className="p-2 hover:bg-black/5 dark:hover:bg-white/5 rounded-full transition-colors"
          >
            <X className="w-6 h-6" />
          </button>
        </div>

        {/* User Info */}
        {isLoggedIn && currentUser ? (
          <div className="px-4 py-4 border-b border-[var(--tg-section-bg-color)]">
            <a href="#profile" className="flex items-center gap-3 hover:opacity-80 transition-opacity">
              <div className="w-12 h-12 bg-gradient-to-br from-[var(--tg-button-color)] to-[#64b5ef] rounded-full flex items-center justify-center text-white font-medium">
                {currentUser.name.slice(0, 2)}
              </div>
              <div className="flex-1 min-w-0">
                <p className="font-medium text-[var(--tg-text-color)] truncate">{currentUser.name}</p>
                <p className="text-sm text-[var(--tg-hint-color)]">
                  {isChairman ? '董事局主席' : isDirector ? '董事' : '普通用户'}
                </p>
              </div>
            </a>
          </div>
        ) : (
          <div className="px-4 py-4 border-b border-[var(--tg-section-bg-color)]">
            <button
              onClick={() => {
                onClose();
                onShowAuth();
              }}
              className="w-full flex items-center gap-3 p-3 bg-[var(--tg-button-color)]/10 rounded-xl hover:bg-[var(--tg-button-color)]/20 transition-colors"
            >
              <div className="w-10 h-10 bg-[var(--tg-button-color)] rounded-full flex items-center justify-center text-white">
                <User className="w-5 h-5" />
              </div>
              <div className="flex-1 text-left">
                <p className="font-medium text-[var(--tg-button-color)]">登录 / 注册</p>
                <p className="text-xs text-[var(--tg-hint-color)]">查看个人信息</p>
              </div>
            </button>
          </div>
        )}

        {/* Navigation */}
        <nav className="py-2">
          {navItems.map((item) => (
            <button
              key={item.href}
              onClick={() => handleNavClick(item.href, item.requireDirector)}
              className={cn(
                'w-full flex items-center gap-4 px-4 py-3 transition-colors',
                activeItem === item.href 
                  ? 'bg-[var(--tg-button-color)]/10 text-[var(--tg-button-color)]' 
                  : 'text-[var(--tg-text-color)] hover:bg-black/5 dark:hover:bg-white/5'
              )}
            >
              <item.icon className="w-6 h-6" />
              <span className="text-base font-medium">{item.label}</span>
              {item.requireDirector && !isDirector && !isBoardLoggedIn && (
                <span className="ml-auto text-xs px-2 py-0.5 bg-gray-100 dark:bg-gray-800 text-gray-500 rounded-full">
                  需董事权限
                </span>
              )}
            </button>
          ))}
        </nav>

        {/* Divider */}
        <div className="border-t border-[var(--tg-section-bg-color)] my-2" />

        {/* Settings */}
        <div className="py-2">
          {/* User Profile Link */}
          {isLoggedIn && (
            <a
              href="#profile"
              className="w-full flex items-center gap-4 px-4 py-3 text-[var(--tg-text-color)] hover:bg-black/5 dark:hover:bg-white/5 transition-colors"
            >
              <User className="w-6 h-6" />
              <span className="text-base font-medium">个人信息</span>
            </a>
          )}

          {/* Theme Toggle */}
          <button
            onClick={() => setTheme(theme === 'dark' ? 'light' : 'dark')}
            className="w-full flex items-center gap-4 px-4 py-3 text-[var(--tg-text-color)] hover:bg-black/5 dark:hover:bg-white/5 transition-colors"
          >
            {theme === 'dark' ? (
              <>
                <Sun className="w-6 h-6" />
                <span className="text-base font-medium">浅色模式</span>
              </>
            ) : (
              <>
                <Moon className="w-6 h-6" />
                <span className="text-base font-medium">深色模式</span>
              </>
            )}
          </button>

          {/* Logout */}
          {isLoggedIn && (
            <button
              onClick={handleLogout}
              className="w-full flex items-center gap-4 px-4 py-3 text-[var(--tg-destructive-text-color)] hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors"
            >
              <LogOut className="w-6 h-6" />
              <span className="text-base font-medium">退出登录</span>
            </button>
          )}
        </div>
      </div>
    </>
  );
}
