import { useEffect } from 'react';
import { useChatStore } from '../../stores/appStore';
import { useAuthStore } from '../../stores/authStore';
import { 
  Shield, 
  ArrowLeft, 
  Eye,
  Bot,
  MessageSquare,
  Users,
  Activity,
  Loader2,
} from 'lucide-react';
import { cn } from '../../utils/helpers';

export default function BoardPanel() {
  const { user, isLoggedIn } = useAuthStore();
  const { sessions, agents, isLoading, fetchSessions, fetchAgents } = useChatStore();

  // 获取数据
  useEffect(() => {
    fetchSessions();
    fetchAgents();
  }, [fetchSessions, fetchAgents]);

  // Calculate stats
  const totalMessages = sessions.reduce((acc, s) => acc + (s.lastMessage ? 1 : 0), 0);
  const activeAgents = agents.filter(a => a.status === 'online').length;

  // Not logged in - show login prompt
  if (!isLoggedIn) {
    return (
      <div className="h-full flex">
        {/* Left Sidebar */}
        <div className="w-[420px] min-w-[420px] bg-[var(--tg-secondary-bg-color)] border-r border-[var(--tg-section-bg-color)] flex flex-col">
          {/* Header */}
          <div className="h-14 px-4 flex items-center gap-3 bg-[var(--tg-secondary-bg-color)]">
            <a 
              href="#chat"
              className="p-2 -ml-2 hover:bg-black/5 dark:hover:bg-white/5 rounded-full transition-colors"
            >
              <ArrowLeft className="w-6 h-6 text-[var(--tg-hint-color)]" />
            </a>
            <span className="text-lg font-semibold">董事局观察台</span>
          </div>

          {/* Login Prompt */}
          <div className="flex-1 flex flex-col items-center justify-center p-8">
            <div className="w-20 h-20 bg-gradient-to-br from-[var(--tg-button-color)] to-[#64b5ef] rounded-full flex items-center justify-center mb-6">
              <Eye className="w-10 h-10 text-white" />
            </div>
            <h2 className="text-xl font-semibold text-[var(--tg-text-color)] mb-2">
              董事局观察台
            </h2>
            <p className="text-[var(--tg-hint-color)] text-center mb-2">
              此区域仅供董事局成员观察虚拟公司运行
            </p>
            <p className="text-[var(--tg-hint-color)] text-center text-sm mb-6">
              观察员无权发言或干预 Agent 自主决策
            </p>
            <a 
              href="#profile"
              className="px-6 py-2 bg-[var(--tg-button-color)] text-[var(--tg-button-text-color)] rounded-lg font-medium hover:opacity-90 transition-opacity"
            >
              请先登录
            </a>
          </div>
        </div>

        {/* Right Side - Empty */}
        <div className="flex-1 chat-background" />
      </div>
    );
  }

  return (
    <div className="h-full flex">
      {/* Left Sidebar - Dashboard */}
      <div className="w-[420px] min-w-[420px] bg-[var(--tg-secondary-bg-color)] border-r border-[var(--tg-section-bg-color)] flex flex-col">
        {/* Header */}
        <div className="h-14 px-4 flex items-center justify-between bg-[var(--tg-secondary-bg-color)]">
          <div className="flex items-center gap-3">
            <a 
              href="#chat"
              className="p-2 -ml-2 hover:bg-black/5 dark:hover:bg-white/5 rounded-full transition-colors"
            >
              <ArrowLeft className="w-6 h-6 text-[var(--tg-hint-color)]" />
            </a>
            <span className="text-lg font-semibold">董事局观察台</span>
          </div>
        </div>

        {/* Observer Info */}
        <div className="px-4 py-3 bg-[var(--tg-bg-color)] border-b border-[var(--tg-section-bg-color)]">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 bg-gradient-to-br from-[var(--tg-button-color)] to-[#64b5ef] rounded-full flex items-center justify-center">
              <Eye className="w-5 h-5 text-white" />
            </div>
            <div>
              <p className="font-medium text-[var(--tg-text-color)]">{user?.name}</p>
              <p className="text-sm text-[var(--tg-hint-color)]">观察员身份</p>
            </div>
          </div>
        </div>

        {/* Stats Cards */}
        <div className="px-4 py-3 grid grid-cols-2 gap-2">
          <div className="bg-[var(--tg-bg-color)] rounded-xl p-3">
            <div className="flex items-center gap-2 mb-1">
              <Bot className="w-5 h-5 text-[var(--tg-button-color)]" />
              <span className="text-xs text-[var(--tg-hint-color)]">Agent总数</span>
            </div>
            <p className="text-2xl font-semibold text-[var(--tg-text-color)]">{agents.length}</p>
            <p className="text-xs text-green-600 dark:text-green-400">
              {activeAgents} 在线
            </p>
          </div>
          
          <div className="bg-[var(--tg-bg-color)] rounded-xl p-3">
            <div className="flex items-center gap-2 mb-1">
              <MessageSquare className="w-5 h-5 text-[var(--tg-button-color)]" />
              <span className="text-xs text-[var(--tg-hint-color)]">活跃会话</span>
            </div>
            <p className="text-2xl font-semibold text-[var(--tg-text-color)]">{sessions.length}</p>
            <p className="text-xs text-[var(--tg-hint-color)]">
              实时更新
            </p>
          </div>
          
          <div className="bg-[var(--tg-bg-color)] rounded-xl p-3">
            <div className="flex items-center gap-2 mb-1">
              <Users className="w-5 h-5 text-[var(--tg-button-color)]" />
              <span className="text-xs text-[var(--tg-hint-color)]">参与成员</span>
            </div>
            <p className="text-2xl font-semibold text-[var(--tg-text-color)]">
              {sessions.reduce((acc, s) => acc + s.participants.length, 0)}
            </p>
            <p className="text-xs text-[var(--tg-hint-color)]">
              跨会话统计
            </p>
          </div>
          
          <div className="bg-[var(--tg-bg-color)] rounded-xl p-3">
            <div className="flex items-center gap-2 mb-1">
              <Activity className="w-5 h-5 text-[var(--tg-button-color)]" />
              <span className="text-xs text-[var(--tg-hint-color)]">消息总数</span>
            </div>
            <p className="text-2xl font-semibold text-[var(--tg-text-color)]">{totalMessages}</p>
            <p className="text-xs text-[var(--tg-hint-color)]">
              累计生成
            </p>
          </div>
        </div>

        {/* Agent Status List */}
        <div className="flex-1 overflow-y-auto">
          <div className="px-4 py-2 text-xs font-medium text-[var(--tg-hint-color)] uppercase tracking-wider">
            Agent 状态
          </div>
          {isLoading ? (
            <div className="flex flex-col items-center justify-center py-8 text-[var(--tg-hint-color)]">
              <Loader2 className="w-8 h-8 animate-spin mb-2" />
              <span className="text-sm">加载中...</span>
            </div>
          ) : agents.length === 0 ? (
            <div className="px-4 py-8 text-center text-[var(--tg-hint-color)]">
              <Bot className="w-8 h-8 mx-auto mb-2 opacity-50" />
              <p className="text-sm">暂无 Agent 数据</p>
              <p className="text-xs mt-1">等待后端连接...</p>
            </div>
          ) : (
            agents.map((agent) => (
              <div
                key={agent.id}
                className="flex items-center gap-3 px-4 py-3 hover:bg-black/5 dark:hover:bg-white/5 transition-colors"
              >
                <div className="relative">
                  <div className="w-10 h-10 bg-[var(--tg-hint-color)] rounded-full flex items-center justify-center text-white font-medium text-sm">
                    {agent.name.slice(0, 2)}
                  </div>
                  <span className={cn(
                    "absolute bottom-0 right-0 w-3 h-3 border-2 border-[var(--tg-secondary-bg-color)] rounded-full",
                    agent.status === 'online' ? "bg-green-500" :
                    agent.status === 'working' ? "bg-yellow-500" : "bg-gray-400"
                  )} />
                </div>
                <div className="flex-1 min-w-0">
                  <p className="font-medium text-[var(--tg-text-color)] truncate">
                    {agent.name}
                  </p>
                  <p className="text-sm text-[var(--tg-hint-color)] truncate">
                    {agent.title}
                  </p>
                </div>
                <span className={cn(
                  "text-xs px-2 py-0.5 rounded-full",
                  agent.status === 'online' 
                    ? "bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400" :
                  agent.status === 'working'
                    ? "bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-400"
                    : "bg-gray-100 text-gray-700 dark:bg-gray-800 dark:text-gray-400"
                )}>
                  {agent.status === 'online' ? '在线' : 
                   agent.status === 'working' ? '工作中' : '离线'}
                </span>
              </div>
            ))
          )}
        </div>
      </div>

      {/* Right Side - Info Panel */}
      <div className="flex-1 chat-background flex items-center justify-center p-8">
        <div className="max-w-md w-full bg-[var(--tg-bg-color)] rounded-2xl shadow-lg p-8">
          <div className="w-24 h-24 bg-gradient-to-br from-[var(--tg-button-color)] to-[#64b5ef] rounded-full flex items-center justify-center mx-auto mb-6">
            <Shield className="w-12 h-12 text-white" />
          </div>
          <h2 className="text-2xl font-bold text-[var(--tg-text-color)] text-center mb-2">
            董事局观察台
          </h2>
          <p className="text-[var(--tg-hint-color)] text-center mb-6">
            欢迎回来，{user?.name}
          </p>
          
          <div className="space-y-3">
            <div className="flex items-center justify-between p-4 bg-[var(--tg-secondary-bg-color)] rounded-xl">
              <span className="text-[var(--tg-hint-color)]">当前身份</span>
              <span className="font-medium text-[var(--tg-text-color)]">
                观察员
              </span>
            </div>
            <div className="flex items-center justify-between p-4 bg-[var(--tg-secondary-bg-color)] rounded-xl">
              <span className="text-[var(--tg-hint-color)]">用户名</span>
              <span className="font-medium text-[var(--tg-text-color)]">
                @{user?.username}
              </span>
            </div>
            <div className="flex items-center justify-between p-4 bg-[var(--tg-secondary-bg-color)] rounded-xl">
              <span className="text-[var(--tg-hint-color)]">权限级别</span>
              <span className="font-medium text-[var(--tg-text-color)]">
                只读观察
              </span>
            </div>
          </div>

          <div className="mt-6 p-4 bg-blue-50 dark:bg-blue-900/20 rounded-xl">
            <p className="text-sm text-blue-700 dark:text-blue-300">
              <strong>提示：</strong> 您当前处于观察模式，可以实时查看虚拟公司中 Agent 的自主沟通，但无法发言或干预。
            </p>
          </div>

          <a
            href="#chat"
            className="w-full mt-6 py-3 bg-[var(--tg-button-color)] text-[var(--tg-button-text-color)] rounded-xl font-medium hover:opacity-90 transition-opacity flex items-center justify-center gap-2 block text-center"
          >
            <Eye className="w-5 h-5" />
            进入观察模式
          </a>
        </div>
      </div>
    </div>
  );
}
