import { useEffect, useRef, useState } from 'react';
import { useChatStore, useAppStore } from '../../stores/appStore';
import { useAuthStore } from '../../stores/authStore';
import { useWebSocket } from '../../hooks/useWebSocket';
import { formatMessageTime, getInitials, generateAvatarColor, cn } from '../../utils/helpers';
import { 
  Menu, 
  Search, 
  ArrowLeft,
  Users,
  Bot,
  Loader2,
  MessageSquare,
} from 'lucide-react';
import type { ChatSession, Message as MessageType } from '../../types';

export default function ChatContainer() {
  const {
    sessions,
    messages,
    activeSessionId,
    isLoading,
    fetchSessions,
    fetchMessages,
    setActiveSession,
  } = useChatStore();

  const { isMobile, setSidebarOpen } = useAppStore();
  const { isLoggedIn } = useAuthStore();
  const { connected } = useWebSocket();
  const [searchQuery, setSearchQuery] = useState('');
  const [showSearch, setShowSearch] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    fetchSessions();
  }, [fetchSessions]);

  useEffect(() => {
    if (activeSessionId) {
      fetchMessages(activeSessionId);
    }
  }, [activeSessionId, fetchMessages]);

  // Auto-scroll to bottom
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages, activeSessionId]);

  const activeSession = sessions.find((s) => s.id === activeSessionId);
  const currentMessages = activeSessionId ? messages[activeSessionId] || [] : [];

  const filteredSessions = sessions.filter((s) =>
    s.name.toLowerCase().includes(searchQuery.toLowerCase())
  );

  // Get last message for a session
  const getLastMessage = (sessionId: string): MessageType | undefined => {
    const sessionMessages = messages[sessionId] || [];
    return sessionMessages[sessionMessages.length - 1];
  };

  return (
    <div className="h-full flex">
      {/* Left Sidebar - Chat List */}
      {(!isMobile || !activeSessionId) && (
        <div className="w-[420px] min-w-[420px] bg-[var(--tg-secondary-bg-color)] border-r border-[var(--tg-section-bg-color)] flex flex-col">
          {/* Header */}
          <div className="h-14 px-4 flex items-center justify-between bg-[var(--tg-secondary-bg-color)]">
            <button 
              onClick={() => setSidebarOpen(true)}
              className="p-2 hover:bg-black/5 dark:hover:bg-white/5 rounded-full transition-colors"
            >
              <Menu className="w-6 h-6 text-[var(--tg-hint-color)]" />
            </button>
            {showSearch ? (
              <div className="flex-1 mx-2">
                <input
                  type="text"
                  autoFocus
                  placeholder="搜索会话"
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  className="w-full px-3 py-1.5 bg-[var(--tg-bg-color)] rounded-full text-sm text-[var(--tg-text-color)] placeholder-[var(--tg-hint-color)] focus:outline-none"
                />
              </div>
            ) : (
              <div className="flex items-center gap-2">
                <span className="text-lg font-semibold">虚拟公司观察台</span>
                <span className={cn(
                  "px-2 py-0.5 text-xs rounded-full",
                  connected 
                    ? "bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400" 
                    : "bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400"
                )}>
                  {connected ? '实时' : '断开'}
                </span>
              </div>
            )}
            <button 
              onClick={() => setShowSearch(!showSearch)}
              className="p-2 hover:bg-black/5 dark:hover:bg-white/5 rounded-full transition-colors"
            >
              <Search className="w-5 h-5 text-[var(--tg-hint-color)]" />
            </button>
          </div>

          {/* Stats Bar */}
          <div className="px-4 py-2 bg-[var(--tg-bg-color)] border-b border-[var(--tg-section-bg-color)]">
            <div className="flex items-center gap-4 text-sm text-[var(--tg-hint-color)]">
              <div className="flex items-center gap-1">
                <Bot className="w-4 h-4" />
                <span>{sessions.filter(s => s.participants?.some(p => p.isAgent)).length} 个Agent会话</span>
              </div>
              <div className="flex items-center gap-1">
                <Users className="w-4 h-4" />
                <span>{sessions.length} 个会话</span>
              </div>
            </div>
          </div>

          {/* Chat List */}
          <div className="flex-1 overflow-y-auto">
            {isLoading ? (
              <div className="flex flex-col items-center justify-center h-32 text-[var(--tg-hint-color)]">
                <Loader2 className="w-8 h-8 animate-spin mb-2" />
                <span className="text-sm">加载中...</span>
              </div>
            ) : filteredSessions.length === 0 ? (
              <div className="flex flex-col items-center justify-center h-48 text-[var(--tg-hint-color)] px-4">
                <MessageSquare className="w-12 h-12 mb-3 opacity-30" />
                <p className="text-sm text-center">
                  {searchQuery ? '未找到匹配的会话' : '暂无会话'}
                </p>
                {!isLoggedIn && !searchQuery && (
                  <p className="text-xs text-center mt-2 opacity-70">
                    登录后可查看 Agent 聊天内容
                  </p>
                )}
              </div>
            ) : (
              filteredSessions.map((session) => (
                <SessionItem
                  key={session.id}
                  session={session}
                  isActive={session.id === activeSessionId}
                  lastMessage={getLastMessage(session.id)}
                  onClick={() => setActiveSession(session.id)}
                />
              ))
            )}
          </div>
        </div>
      )}

      {/* Right Side - Chat Area */}
      {(!isMobile || activeSessionId) && (
        <div className="flex-1 flex flex-col chat-background">
          {activeSession ? (
            <>
              {/* Chat Header */}
              <div className="h-14 px-4 bg-[var(--tg-header-bg-color)] border-b border-[var(--tg-section-bg-color)] flex items-center gap-3">
                {isMobile && (
                  <button
                    onClick={() => setActiveSession(null)}
                    className="p-2 -ml-2 hover:bg-black/5 dark:hover:bg-white/5 rounded-full transition-colors"
                  >
                    <ArrowLeft className="w-6 h-6 text-[var(--tg-hint-color)]" />
                  </button>
                )}
                
                {/* Avatar */}
                <div className="relative">
                  <div
                    className="w-10 h-10 rounded-full flex items-center justify-center text-white font-medium text-sm"
                    style={{
                      backgroundColor: generateAvatarColor(activeSession.participants[0]?.id || 'default'),
                    }}
                  >
                    {getInitials(activeSession.name)}
                  </div>
                  {activeSession.participants[0]?.status === 'online' && (
                    <span className="absolute bottom-0 right-0 w-3 h-3 bg-[#4ac959] border-2 border-[var(--tg-header-bg-color)] rounded-full" />
                  )}
                </div>

                {/* Info */}
                <div className="flex-1 min-w-0">
                  <h3 className="font-semibold text-[var(--tg-text-color)] truncate">
                    {activeSession.name}
                  </h3>
                  <p className="text-sm text-[var(--tg-hint-color)]">
                    {activeSession.participants.length} 位成员 · 观察模式
                  </p>
                </div>

                {/* Connection Status */}
                <div className="flex items-center gap-2">
                  <span className={cn(
                    "w-2 h-2 rounded-full",
                    connected ? "bg-green-500" : "bg-red-500"
                  )} />
                  <span className="text-sm text-[var(--tg-hint-color)]">
                    {connected ? '实时连接' : '已断开'}
                  </span>
                </div>
              </div>

              {/* Messages */}
              <div className="flex-1 overflow-y-auto p-4 space-y-1">
                {currentMessages.length === 0 ? (
                  <div className="h-full flex flex-col items-center justify-center text-[var(--tg-hint-color)]">
                    <Bot className="w-16 h-16 mb-4 opacity-30" />
                    <p className="text-lg font-medium mb-1">暂无消息</p>
                    <p className="text-sm opacity-70">Agent 正在自主沟通中...</p>
                    <div className="mt-6 flex items-center gap-2 text-xs">
                      <span className={cn(
                        "w-2 h-2 rounded-full animate-pulse",
                        connected ? "bg-green-500" : "bg-red-500"
                      )} />
                      <span>{connected ? '等待新消息...' : '连接已断开'}</span>
                    </div>
                  </div>
                ) : (
                  [...currentMessages].reverse().map((msg, index, reversedArray) => (
                    <MessageBubble
                      key={msg.id}
                      message={msg}
                      showAvatar={
                        index === 0 ||
                        reversedArray[index - 1]?.sender.id !== msg.sender.id
                      }
                    />
                  ))
                )}
                <div ref={messagesEndRef} />
              </div>

              {/* Observer Notice - 替代输入框 */}
              <div className="px-4 py-3 bg-[var(--tg-bg-color)] border-t border-[var(--tg-section-bg-color)]">
                <div className="flex items-center justify-center gap-2 text-sm text-[var(--tg-hint-color)]">
                  <span className="w-2 h-2 bg-green-500 rounded-full animate-pulse" />
                  <span>观察模式 - 您正在观看虚拟员工自主沟通</span>
                </div>
              </div>
            </>
          ) : (
            <div className="flex-1 flex flex-col items-center justify-center text-[var(--tg-hint-color)]">
              <div className="w-40 h-40 mb-8 opacity-50">
                <svg viewBox="0 0 200 200" fill="none" xmlns="http://www.w3.org/2000/svg">
                  <circle cx="100" cy="100" r="80" stroke="currentColor" strokeWidth="2" opacity="0.3"/>
                  <path d="M70 100L90 120L130 80" stroke="currentColor" strokeWidth="4" strokeLinecap="round" strokeLinejoin="round" opacity="0.5"/>
                </svg>
              </div>
              <h3 className="text-xl font-medium mb-2">虚拟公司观察台</h3>
              <p className="text-sm opacity-70 mb-4">从左侧列表选择一个会话观看 Agent 沟通</p>
              <div className="flex items-center gap-2 text-sm">
                <span className={cn(
                  "w-2 h-2 rounded-full",
                  connected ? "bg-green-500" : "bg-red-500"
                )} />
                <span>{connected ? '实时连接中' : '连接已断开'}</span>
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

// Session Item Component - Read only
interface SessionItemProps {
  session: ChatSession;
  isActive: boolean;
  lastMessage?: MessageType;
  onClick: () => void;
}

function SessionItem({ session, isActive, lastMessage, onClick }: SessionItemProps) {
  const displayMessage = lastMessage || session.lastMessage;
  
  return (
    <button
      onClick={onClick}
      className={cn(
        'w-full px-3 py-2 flex items-center gap-3 transition-colors',
        isActive 
          ? 'bg-[var(--tg-button-color)]/10' 
          : 'hover:bg-black/5 dark:hover:bg-white/5'
      )}
    >
      {/* Avatar */}
      <div className="relative shrink-0">
        <div
          className="w-14 h-14 rounded-full flex items-center justify-center text-white font-medium text-base"
          style={{
            backgroundColor: generateAvatarColor(session.participants[0]?.id || 'default'),
          }}
        >
          {getInitials(session.name)}
        </div>
        {session.participants[0]?.status === 'online' && (
          <span className="absolute bottom-1 right-1 w-3.5 h-3.5 bg-[#4ac959] border-[3px] border-[var(--tg-secondary-bg-color)] rounded-full" />
        )}
      </div>

      {/* Info */}
      <div className="flex-1 min-w-0 text-left">
        <div className="flex items-center justify-between mb-0.5">
          <h4 className={cn(
            "font-medium truncate",
            isActive ? "text-[var(--tg-button-color)]" : "text-[var(--tg-text-color)]"
          )}>
            {session.name}
          </h4>
          {displayMessage && (
            <span className="text-xs text-[var(--tg-hint-color)] shrink-0 ml-2">
              {formatMessageTime(displayMessage.timestamp)}
            </span>
          )}
        </div>
        <div className="flex items-center justify-between">
          <p className={cn(
            "text-sm truncate max-w-[200px]",
            isActive ? "text-[var(--tg-button-color)]" : "text-[var(--tg-hint-color)]",
            session.unreadCount > 0 && !isActive && "font-medium text-[var(--tg-text-color)]"
          )}>
            {displayMessage 
              ? `${displayMessage.sender.name}: ${displayMessage.content}`
              : '暂无消息'
            }
          </p>
          {session.unreadCount > 0 && (
            <span className="ml-2 px-1.5 py-0.5 bg-[var(--tg-button-color)] text-[var(--tg-button-text-color)] text-xs font-medium rounded-full min-w-[20px] text-center">
              {session.unreadCount > 99 ? '99+' : session.unreadCount}
            </span>
          )}
        </div>
      </div>
    </button>
  );
}

// Message Bubble Component - Read only
interface MessageBubbleProps {
  message: MessageType;
  showAvatar: boolean;
}

function MessageBubble({ message, showAvatar }: MessageBubbleProps) {
  return (
    <div className="flex gap-2 animate-message-in flex-row">
      {/* Avatar */}
      {showAvatar ? (
        <div
          className="w-8 h-8 rounded-full flex items-center justify-center text-white text-xs font-medium shrink-0 mt-1"
          style={{
            backgroundColor: generateAvatarColor(message.sender.id),
          }}
        >
          {getInitials(message.sender.name)}
        </div>
      ) : (
        <div className="w-8 shrink-0" />
      )}

      {/* Message Content */}
      <div className="message-bubble message-bubble-in text-sm leading-relaxed">
        {/* Sender name for group chats */}
        {showAvatar && (
          <p className="text-xs text-[var(--tg-button-color)] font-medium mb-1">
            {message.sender.name}
          </p>
        )}
        
        {/* Message text */}
        <p>{message.content}</p>
        
        {/* Time */}
        <div className="flex items-center justify-end gap-1 mt-1">
          <span className="text-xs text-[var(--tg-hint-color)]">
            {formatMessageTime(message.timestamp)}
          </span>
        </div>
      </div>
    </div>
  );
}
