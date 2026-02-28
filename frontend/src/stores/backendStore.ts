import { create } from 'zustand';
import { persist, createJSONStorage } from 'zustand/middleware';

// ==================== 存储配置 ====================
const STORAGE_KEY = 'imitatort-backend-v1';


/**
 * 将 http/https 转换为 ws/wss
 */
const convertToWsUrl = (httpUrl: string): string => {
  try {
    const urlObj = new URL(httpUrl);
    const protocol = urlObj.protocol === 'https:' ? 'wss:' : 'ws:';
    const port = urlObj.port ? `:${urlObj.port}` : '';
    return `${protocol}//${urlObj.hostname}${port}`;
  } catch (e) {
    console.error('[Backend] Failed to convert to WS URL:', e);
    return 'ws://localhost:8080';
  }
};

// ==================== 状态定义 ====================
interface BackendState {
  // 状态
  backendUrl: string;
  isValid: boolean;

  // Actions
  setBackendUrl: (url: string) => { valid: boolean };
  getApiUrl: (path: string) => string;
  getWsUrl: () => string;
  getBaseUrl: () => string;
}

// ==================== 创建 Store ====================
export const useBackendStore = create<BackendState>()(
  persist(
    (set, get) => ({
      // 初始状态
      backendUrl: typeof window !== 'undefined' && (window as any).IMITATOR_CONFIG?.defaultBackendUrl || 'http://localhost:8080',
      isValid: true,

      // 设置后端地址
      setBackendUrl: (url: string) => {
        // 移除末尾的斜杠
        const cleanUrl = url.replace(/\/$/, '');
        set({ backendUrl: cleanUrl, isValid: true });
        console.log('[Backend] URL updated:', cleanUrl);

        return { valid: true };
      },

      // 获取完整 API URL
      getApiUrl: (path: string) => {
        const { backendUrl } = get();
        const cleanPath = path.startsWith('/') ? path : `/${path}`;
        return `${backendUrl}${cleanPath}`;
      },

      // 获取 WebSocket URL
      getWsUrl: () => {
        const { backendUrl } = get();
        const wsUrl = convertToWsUrl(backendUrl);
        return `${wsUrl}/ws`;
      },

      // 获取基础 URL（不含路径）
      getBaseUrl: () => {
        return get().backendUrl;
      },
    }),
    {
      name: STORAGE_KEY,
      storage: createJSONStorage(() => localStorage),
      // 只持久化这些字段
      partialize: (state) => ({
        backendUrl: state.backendUrl,
      }),
      onRehydrateStorage: () => (state) => {
        console.log('[Backend] Store rehydrated:', state?.backendUrl);
      },
    }
  )
);

// ==================== 导出便捷方法 ====================
export const getBackendUrl = () => useBackendStore.getState().backendUrl;
export const getApiUrl = (path: string) => useBackendStore.getState().getApiUrl(path);
export const getWsUrl = () => useBackendStore.getState().getWsUrl();
export const setBackendUrl = (url: string) => useBackendStore.getState().setBackendUrl(url);
