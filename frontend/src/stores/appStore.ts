import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { getApiUrl } from './backendStore';
import type {
  AppState,
  BoardState,
  ChatSession,
  Message,
  AgentInfo,
  Department,
} from '../types/index';

// ==================== App Store ====================
interface ExtendedAppState extends AppState {
  setTheme: (theme: AppState['theme']) => void;
  toggleSidebar: () => void;
  setSidebarOpen: (open: boolean) => void;
  setActiveSession: (sessionId: string | null) => void;
  setIsMobile: (isMobile: boolean) => void;
}

export const useAppStore = create<ExtendedAppState>()(
  persist(
    (set) => ({
      theme: 'light',
      sidebarOpen: false,
      activeSessionId: null,
      isMobile: false,

      setTheme: (theme) => set({ theme }),
      toggleSidebar: () => set((state) => ({ sidebarOpen: !state.sidebarOpen })),
      setSidebarOpen: (sidebarOpen) => set({ sidebarOpen }),
      setActiveSession: (activeSessionId) => set({ activeSessionId }),
      setIsMobile: (isMobile) => set({ isMobile }),
    }),
    {
      name: 'app-storage',
      partialize: (state) => ({ theme: state.theme }),
    }
  )
);

// ==================== Board Store (只读观察) ====================
interface ExtendedBoardState extends BoardState {
  token: string | null;
  login: (username: string, password: string) => Promise<boolean>;
  logout: () => void;
  fetchMembers: () => Promise<void>;
  addMember: (member: { username: string; password: string; name: string; is_chairman?: boolean }) => Promise<boolean>;
  deleteMember: (memberId: string) => Promise<boolean>;
}

export const useBoardStore = create<ExtendedBoardState>()(
  persist(
    (set, get) => ({
      isLoggedIn: false,
      currentUser: null,
      token: null,
      members: [],

      login: async (username, password) => {
        try {
          const res = await fetch(getApiUrl('/api/board/login'), {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ username, password }),
          });
          const data = await res.json();
          if (data.success) {
            set({ 
              isLoggedIn: true, 
              currentUser: data.data.member,
              token: data.data.token 
            });
            return true;
          }
          return false;
        } catch (_error) {
          console.error('Login error:', _error);
          // Fallback for development
          if (username === 'observer' && password === 'observe') {
            set({ 
              isLoggedIn: true, 
              currentUser: {
                id: 'observer-001',
                username: 'observer',
                name: '观察员',
                is_chairman: false,
                created_at: Date.now(),
              },
              token: null
            });
            return true;
          }
          return false;
        }
      },

      logout: () => {
        set({ isLoggedIn: false, currentUser: null, token: null, members: [] });
      },

      fetchMembers: async () => {
        try {
          const { token } = get();
          const res = await fetch(getApiUrl('/api/board/members'), {
            headers: token ? { 'Authorization': `Bearer ${token}` } : {},
          });
          const data = await res.json();
          if (data.success) {
            set({ members: data.data });
          }
        } catch (_error) {
          console.log('Board members fetch failed, using empty list');
          set({ members: [] });
        }
      },

      addMember: async (member) => {
        try {
          const { token } = get();
          const res = await fetch(getApiUrl('/api/board/members'), {
            method: 'POST',
            headers: { 
              'Content-Type': 'application/json',
              'Authorization': `Bearer ${token}` 
            },
            body: JSON.stringify(member),
          });
          const data = await res.json();
          if (data.success) {
            // Refresh members list
            await get().fetchMembers();
            return true;
          }
          return false;
        } catch (_error) {
          console.error('Add member error:', _error);
          return false;
        }
      },

      deleteMember: async (memberId) => {
        try {
          const { token } = get();
          const res = await fetch(getApiUrl(`/api/board/members/${memberId}`), {
            method: 'DELETE',
            headers: { 
              'Authorization': `Bearer ${token}` 
            },
          });
          const data = await res.json();
          if (data.success) {
            // Refresh members list
            await get().fetchMembers();
            return true;
          }
          return false;
        } catch (_error) {
          console.error('Delete member error:', _error);
          return false;
        }
      },
    }),
    {
      name: 'board-storage',
      partialize: (state) => ({ 
        isLoggedIn: state.isLoggedIn, 
        currentUser: state.currentUser,
        token: state.token 
      }),
    }
  )
);

// ==================== Chat Store (只读观察模式) ====================
interface ExtendedChatStore {
  sessions: ChatSession[];
  messages: Record<string, Message[]>;
  typingUsers: Record<string, string[]>;
  agents: AgentInfo[];
  departments: Department[];
  isLoading: boolean;
  activeSessionId: string | null;
  
  fetchSessions: () => Promise<void>;
  fetchMessages: (sessionId: string) => Promise<void>;
  fetchAgents: () => Promise<void>;
  fetchDepartments: () => Promise<void>;
  setActiveSession: (sessionId: string | null) => void;
  addMessage: (sessionId: string, message: Message) => void;
  setTyping: (sessionId: string, userId: string, isTyping: boolean) => void;
  updateSession: (session: ChatSession) => void;
  updateAgents: (agents: AgentInfo[]) => void;
}

export const useChatStore = create<ExtendedChatStore>()((set) => ({
  sessions: [],
  messages: {},
  typingUsers: {},
  agents: [],
  departments: [],
  isLoading: false,
  activeSessionId: null,

  fetchSessions: async () => {
    set({ isLoading: true });
    try {
      const res = await fetch(getApiUrl('/api/chat/list'));
      const data = await res.json();
      if (data.success) {
        set({ sessions: data.data });
      }
    } catch (_error) {
      console.error('Fetch sessions error:', _error);
      set({ sessions: [] });
    } finally {
      set({ isLoading: false });
    }
  },

  setActiveSession: (sessionId) => {
    set({ activeSessionId: sessionId });
  },

  fetchMessages: async (sessionId) => {
    try {
      const res = await fetch(getApiUrl(`/api/chat/${sessionId}/messages`));
      const data = await res.json();
      if (data.success) {
        set((state) => ({
          messages: { ...state.messages, [sessionId]: data.data },
        }));
      }
    } catch (_error) {
      console.error('Fetch messages error:', _error);
      set((state) => ({
        messages: { ...state.messages, [sessionId]: [] },
      }));
    }
  },

  fetchAgents: async () => {
    try {
      const res = await fetch(getApiUrl('/api/agents'));
      const data = await res.json();
      if (data.success) {
        set({ agents: data.data });
      }
    } catch (_error) {
      console.error('Fetch agents error:', _error);
    }
  },

  fetchDepartments: async () => {
    try {
      const res = await fetch(getApiUrl('/api/org/tree'));
      const data = await res.json();
      if (data.success) {
        set({ departments: data.data });
      }
    } catch (_error) {
      console.error('Fetch departments error:', _error);
    }
  },

  addMessage: (sessionId, message) => {
    set((state) => ({
      messages: {
        ...state.messages,
        [sessionId]: [...(state.messages[sessionId] || []), message],
      },
      // Update session's last message
      sessions: state.sessions.map(s => 
        s.id === sessionId 
          ? { ...s, lastMessage: message, updatedAt: new Date() }
          : s
      ),
    }));
  },

  setTyping: (sessionId, userId, isTyping) => {
    set((state) => {
      const current = state.typingUsers[sessionId] || [];
      const updated = isTyping
        ? [...new Set([...current, userId])]
        : current.filter((id) => id !== userId);
      return {
        typingUsers: { ...state.typingUsers, [sessionId]: updated },
      };
    });
  },

  updateSession: (session) => {
    set((state) => {
      const exists = state.sessions.find(s => s.id === session.id);
      if (exists) {
        return {
          sessions: state.sessions.map(s => 
            s.id === session.id ? { ...s, ...session } : s
          ),
        };
      }
      return {
        sessions: [session, ...state.sessions],
      };
    });
  },

  updateAgents: (agents) => {
    set({ agents });
  },
}));
