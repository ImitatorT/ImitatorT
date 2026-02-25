import { create } from 'zustand';
import { persist, createJSONStorage } from 'zustand/middleware';

// ==================== 存储配置 ====================
const STORAGE_KEY = 'imitatort-backend-v1';

// ==================== 工具函数 ====================

/**
 * 验证后端地址格式
 * 支持: http://localhost:8080, http://127.0.0.1:8080, http://192.168.x.x:8080, https://xxx.com
 */
export const validateBackendUrl = (url: string): { valid: boolean; error?: string } => {
  if (!url) {
    return { valid: false, error: '请输入后端地址' };
  }

  try {
    const urlObj = new URL(url);

    // 必须是 http 或 https 协议
    if (urlObj.protocol !== 'http:' && urlObj.protocol !== 'https:') {
      return { valid: false, error: '地址必须使用 http:// 或 https:// 协议' };
    }

    // 必须有主机名
    if (!urlObj.hostname) {
      return { valid: false, error: '地址格式不正确，缺少主机名' };
    }

    // 必须有端口号（生产环境可能不需要，但这里默认需要）
    // 如果是标准端口（80/443），可以不显示
    if (!urlObj.port && urlObj.protocol === 'http:') {
      // 使用默认 80 端口，允许
    }

    return { valid: true };
  } catch (e) {
    return { valid: false, error: '地址格式不正确，示例: http://localhost:8080' };
  }
};

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
  setBackendUrl: (url: string) => { valid: boolean; error?: string };
  getApiUrl: (path: string) => string;
  getWsUrl: () => string;
  getBaseUrl: () => string;
}

// ==================== 创建 Store ====================
export const useBackendStore = create<BackendState>()(
  persist(
    (set, get) => ({
      // 初始状态
      backendUrl: 'http://localhost:8080',
      isValid: true,

      // 设置后端地址
      setBackendUrl: (url: string) => {
        const validation = validateBackendUrl(url);

        if (validation.valid) {
          // 移除末尾的斜杠
          const cleanUrl = url.replace(/\/$/, '');
          set({ backendUrl: cleanUrl, isValid: true });
          console.log('[Backend] URL updated:', cleanUrl);
        } else {
          set({ isValid: false });
          console.log('[Backend] URL validation failed:', validation.error);
        }

        return validation;
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
