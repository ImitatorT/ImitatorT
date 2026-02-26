// ==================== 用户类型 ====================
export interface User {
  id: string;
  name: string;
  username: string;
  avatar?: string;
  title?: string;
  dept?: string;
  email?: string;
  phone?: string;
  status: 'online' | 'away' | 'busy' | 'offline';
  isAgent?: boolean;
  manager?: User;
}

// ==================== 认证用户类型 ====================
export interface AuthUserInfo {
  id: string;
  username: string;
  name: string;
  email?: string;
  avatar?: string;
  created_at?: number;
  is_director: boolean;
  employee_id?: string;
  position?: string;
  department?: string;
}

// ==================== Agent 类型 ====================
export interface AgentInfo {
  id: string;
  name: string;
  title: string;
  department: string;
  status: 'online' | 'offline' | 'working';
  expertise: string[];
}

// ==================== 部门类型 ====================
export interface Department {
  id: string;
  name: string;
  icon?: string;
  memberCount: number;
  parent_id?: string;
  sort_order?: number;
  leader_id?: string;
  children?: Department[];
  users?: User[];
  leader?: User;
}

// ==================== 消息类型 ====================
export interface Message {
  id: string;
  content: string;
  sender: User;
  timestamp: Date;
  type: 'text' | 'markdown' | 'image' | 'file' | 'system';
  replyTo?: Message;
  mentions?: string[];
  isEdited?: boolean;
  readBy?: string[];
  status?: 'sending' | 'sent' | 'delivered' | 'read';
}

// ==================== 会话类型 ====================
export interface ChatSession {
  id: string;
  type: 'direct' | 'group';
  name: string;
  avatar?: string;
  participants: User[];
  lastMessage?: Message;
  unreadCount: number;
  isPinned?: boolean;
  isMuted?: boolean;
  updatedAt: Date;
}

// ==================== 董事局类型 ====================
export interface BoardMember {
  id: string;
  username: string;
  name: string;
  is_chairman: boolean;
  created_at: number;
}

export interface BoardState {
  isLoggedIn: boolean;
  currentUser: BoardMember | null;
  token: string | null;
  members: BoardMember[];
}

// ==================== 应用状态 ====================
export interface AppState {
  theme: 'light' | 'dark' | 'auto';
  sidebarOpen: boolean;
  activeSessionId: string | null;
  isMobile: boolean;
}

// ==================== 虚拟公司事件 ====================
export type CompanyEvent =
  | { type: 'message_sent'; message_id: string; session_id: string; sender: User; content: string; timestamp: string }
  | { type: 'agent_typing'; agent_id: string; session_id: string }
  | { type: 'agent_online'; agent_id: string; name: string }
  | { type: 'agent_offline'; agent_id: string }
  | { type: 'group_created'; group_id: string; name: string; creator: AgentInfo; members: User[] }
  | { type: 'system'; message: string; level: 'info' | 'warning' | 'error' };

// ==================== API 响应 ====================
export interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
}

// ==================== 公司状态 ====================
export interface CompanyState {
  agents: AgentInfo[];
  sessions: ChatSession[];
  isConnected: boolean;
}
