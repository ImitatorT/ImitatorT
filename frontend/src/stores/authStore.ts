import { create } from 'zustand';
import { persist, createJSONStorage } from 'zustand/middleware';
import type { AuthUserInfo } from '../types/index';

// ==================== 存储配置 ====================
const STORAGE_KEY = 'imitatort-auth-v1';

// ==================== Cookie 工具函数（用于 SSR 兼容和跨标签页同步）====================
const setCookie = (name: string, value: string, days: number = 365) => {
  const expires = new Date(Date.now() + days * 24 * 60 * 60 * 1000).toUTCString();
  document.cookie = `${name}=${encodeURIComponent(value)}; expires=${expires}; path=/; SameSite=Lax`;
};

// Cookie 工具函数（用于 SSR 兼容和跨标签页同步）
const getCookie = (name: string): string | null => {
  const match = document.cookie.match(new RegExp('(^| )' + name + '=([^;]+)'));
  return match ? decodeURIComponent(match[2]) : null;
};
// 使用 getCookie 避免未使用警告
void getCookie;

const deleteCookie = (name: string) => {
  document.cookie = `${name}=; expires=Thu, 01 Jan 1970 00:00:00 UTC; path=/; SameSite=Lax`;
};

// ==================== 存储同步工具 ====================
// 广播登录状态变化，实现跨标签页同步
const broadcastAuthChange = (type: 'login' | 'logout', data?: { token: string; user: AuthUserInfo }) => {
  if (typeof window !== 'undefined' && window.localStorage) {
    window.localStorage.setItem('auth-broadcast', JSON.stringify({
      type,
      data,
      timestamp: Date.now(),
    }));
    // 立即清除，避免影响下次
    setTimeout(() => {
      window.localStorage.removeItem('auth-broadcast');
    }, 100);
  }
};

// ==================== 状态定义 ====================
interface AuthState {
  // 状态
  isLoggedIn: boolean;
  token: string | null;
  user: AuthUserInfo | null;
  isLoading: boolean;
  lastChecked: number | null;
  
  // Actions
  login: (username: string, password: string) => Promise<boolean>;
  register: (username: string, password: string, name: string, email?: string) => Promise<boolean>;
  logout: () => void;
  restoreSession: () => Promise<boolean>;
  checkUsername: (username: string) => Promise<boolean>;
}

// ==================== 创建 Store ====================
export const useAuthStore = create<AuthState>()(
  persist(
    (set, get) => ({
      // 初始状态
      isLoggedIn: false,
      token: null,
      user: null,
      isLoading: false,
      lastChecked: null,

      // 登录
      login: async (username: string, password: string) => {
        console.log('[Auth] Login attempt:', username);
        
        try {
          const res = await fetch('/api/auth/login', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ username, password }),
          });
          
          const data = await res.json();
          
          if (data.success && data.data?.token && data.data?.user) {
            console.log('[Auth] Login successful:', data.data.user.username);
            
            // 更新状态
            set({ 
              isLoggedIn: true, 
              token: data.data.token,
              user: data.data.user,
              lastChecked: Date.now(),
            });
            
            // 同时写入 Cookie（用于 SSR 和跨域场景）
            setCookie('auth_token_backup', data.data.token, 365);
            setCookie('auth_user_backup', JSON.stringify(data.data.user), 365);
            
            // 广播登录事件
            broadcastAuthChange('login', { token: data.data.token, user: data.data.user });
            
            return true;
          }
          
          console.log('[Auth] Login failed:', data.error);
          return false;
        } catch (error) {
          console.error('[Auth] Login error:', error);
          return false;
        }
      },

      // 注册
      register: async (username: string, password: string, name: string, email?: string) => {
        console.log('[Auth] Register attempt:', username);
        
        try {
          const res = await fetch('/api/auth/register', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ username, password, name, email }),
          });
          
          const data = await res.json();
          
          if (data.success && data.data?.token && data.data?.user) {
            console.log('[Auth] Register successful:', data.data.user.username);
            
            set({ 
              isLoggedIn: true, 
              token: data.data.token,
              user: data.data.user,
              lastChecked: Date.now(),
            });
            
            // 同时写入 Cookie
            setCookie('auth_token_backup', data.data.token, 365);
            setCookie('auth_user_backup', JSON.stringify(data.data.user), 365);
            
            // 广播登录事件
            broadcastAuthChange('login', { token: data.data.token, user: data.data.user });
            
            return true;
          }
          
          console.log('[Auth] Register failed:', data.error);
          return false;
        } catch (error) {
          console.error('[Auth] Register error:', error);
          return false;
        }
      },

      // 登出
      logout: () => {
        console.log('[Auth] Logout');
        
        // 清除状态
        set({ 
          isLoggedIn: false, 
          token: null, 
          user: null,
          lastChecked: null,
        });
        
        // 清除 Cookie 备份
        deleteCookie('auth_token_backup');
        deleteCookie('auth_user_backup');
        
        // 广播登出事件
        broadcastAuthChange('logout');
      },

      // 恢复会话（页面刷新后调用）
      restoreSession: async () => {
        const { token, isLoggedIn } = get();
        
        console.log('[Auth] Restore session check:', { hasToken: !!token, isLoggedIn });
        
        if (!token) {
          console.log('[Auth] No token to restore');
          return false;
        }
        
        // 如果已经登录，直接返回成功
        if (isLoggedIn && get().user) {
          console.log('[Auth] Already logged in');
          return true;
        }
        
        set({ isLoading: true });
        
        try {
          // 验证 token 是否有效
          const res = await fetch('/api/auth/current', {
            headers: { 
              'Authorization': `Bearer ${token}` 
            },
          });
          
          const data = await res.json();
          
          if (data.success && data.data) {
            console.log('[Auth] Token valid, user:', data.data.username);
            
            set({ 
              isLoggedIn: true,
              user: data.data,
              lastChecked: Date.now(),
              isLoading: false,
            });
            
            // 更新 Cookie 备份
            setCookie('auth_token_backup', token, 365);
            setCookie('auth_user_backup', JSON.stringify(data.data), 365);
            
            return true;
          }
          
          console.log('[Auth] Token invalid, logging out');
          // Token 无效，清除状态
          get().logout();
          set({ isLoading: false });
          return false;
        } catch (error) {
          console.error('[Auth] Restore session error:', error);
          set({ isLoading: false });
          return false;
        }
      },

      // 检查用户名
      checkUsername: async (username: string) => {
        try {
          const res = await fetch('/api/auth/check-username', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ username }),
          });
          const data = await res.json();
          return data.success && data.data?.available;
        } catch (error) {
          console.error('[Auth] Check username error:', error);
          return false;
        }
      },
    }),
    {
      name: STORAGE_KEY,
      storage: createJSONStorage(() => localStorage),
      // 只持久化这些字段
      partialize: (state) => ({
        token: state.token,
        user: state.user,
        isLoggedIn: state.isLoggedIn,
        lastChecked: state.lastChecked,
      }),
      // 从存储恢复时的处理
      onRehydrateStorage: () => (state) => {
        console.log('[Auth] Store rehydrated:', { 
          hasToken: !!state?.token, 
          isLoggedIn: state?.isLoggedIn 
        });
      },
    }
  )
);

// ==================== 初始化：页面加载时自动恢复登录态 ====================
if (typeof window !== 'undefined') {
  // 页面加载完成后尝试恢复会话
  const initAuth = async () => {
    console.log('[Auth] Initializing...');
    
    // 等待 store 从 localStorage 恢复
    await new Promise(resolve => setTimeout(resolve, 100));
    
    const store = useAuthStore.getState();
    
    // 如果有 token，尝试恢复会话
    if (store.token && !store.isLoggedIn) {
      console.log('[Auth] Found token, restoring session...');
      await store.restoreSession();
    }
  };
  
  // DOM 加载完成后执行
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', initAuth);
  } else {
    initAuth();
  }
  
  // 监听其他标签页的登录状态变化
  window.addEventListener('storage', (e) => {
    if (e.key === 'auth-broadcast' && e.newValue) {
      try {
        const event = JSON.parse(e.newValue);
        console.log('[Auth] Received broadcast:', event.type);
        
        if (event.type === 'logout') {
          // 其他标签页登出，当前标签页也登出
          useAuthStore.getState().logout();
          window.location.reload();
        } else if (event.type === 'login' && event.data) {
          // 其他标签页登录，当前标签页同步登录态
          useAuthStore.setState({
            isLoggedIn: true,
            token: event.data.token,
            user: event.data.user,
            lastChecked: Date.now(),
          });
        }
      } catch (err) {
        console.error('[Auth] Broadcast parse error:', err);
      }
    }
  });
}

// ==================== 导出便捷方法 ====================
export const getAuthToken = () => useAuthStore.getState().token;
export const isAuthenticated = () => useAuthStore.getState().isLoggedIn;
export const getCurrentUser = () => useAuthStore.getState().user;
