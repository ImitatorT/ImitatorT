import { useAuthStore } from '../../stores/authStore';
import { 
  User, 
  Mail, 
  Calendar, 
  Shield, 
  Crown, 
  LogOut,
  ArrowLeft
} from 'lucide-react';
import { formatDate } from '../../utils/helpers';
import { Button } from '../ui';
import toast from 'react-hot-toast';

export default function UserProfile() {
  const { user, logout } = useAuthStore();

  const handleLogout = () => {
    logout();
    toast.success('已退出登录');
    window.location.hash = '#chat';
  };

  if (!user) {
    return (
      <div className="h-full flex items-center justify-center">
        <div className="text-center">
          <div className="w-16 h-16 mx-auto mb-4 rounded-full bg-[var(--tg-secondary-bg-color)] flex items-center justify-center">
            <User className="w-8 h-8 text-[var(--tg-hint-color)]" />
          </div>
          <p className="text-[var(--tg-hint-color)]">请先登录</p>
        </div>
      </div>
    );
  }

  return (
    <div className="h-full flex">
      {/* Left Sidebar */}
      <div className="w-full max-w-md mx-auto bg-[var(--tg-bg-color)] flex flex-col">
        {/* Header */}
        <div className="h-14 px-4 flex items-center gap-3 border-b border-[var(--tg-section-bg-color)]">
          <button 
            onClick={() => window.history.back()}
            className="p-2 -ml-2 hover:bg-black/5 dark:hover:bg-white/5 rounded-full transition-colors"
          >
            <ArrowLeft className="w-6 h-6 text-[var(--tg-hint-color)]" />
          </button>
          <span className="text-lg font-semibold">个人信息</span>
        </div>

        {/* Profile Content */}
        <div className="flex-1 overflow-y-auto p-4 space-y-4">
          {/* Avatar & Name */}
          <div className="text-center py-6">
            <div className="w-24 h-24 mx-auto mb-4 rounded-full bg-gradient-to-br from-[var(--tg-button-color)] to-[#64b5ef] flex items-center justify-center text-white text-3xl font-bold">
              {user.name.slice(0, 2)}
            </div>
            <h2 className="text-xl font-bold text-[var(--tg-text-color)]">{user.name}</h2>
            <p className="text-[var(--tg-hint-color)]">@{user.username}</p>
            
            {/* Role Badge */}
            <div className="mt-3 flex justify-center gap-2">
              {user.is_director ? (
                <span className="inline-flex items-center gap-1 px-3 py-1 bg-purple-100 dark:bg-purple-900/30 text-purple-700 dark:text-purple-400 rounded-full text-sm">
                  <Crown className="w-4 h-4" />
                  董事
                </span>
              ) : (
                <span className="inline-flex items-center gap-1 px-3 py-1 bg-gray-100 dark:bg-gray-800 text-gray-700 dark:text-gray-400 rounded-full text-sm">
                  <User className="w-4 h-4" />
                  普通用户
                </span>
              )}
            </div>
          </div>

          {/* Info Cards */}
          <div className="space-y-3">
            {/* Username */}
            <div className="p-4 bg-[var(--tg-secondary-bg-color)] rounded-xl">
              <div className="flex items-center gap-3">
                <div className="w-10 h-10 rounded-full bg-[var(--tg-button-color)]/10 flex items-center justify-center">
                  <User className="w-5 h-5 text-[var(--tg-button-color)]" />
                </div>
                <div className="flex-1">
                  <p className="text-sm text-[var(--tg-hint-color)]">用户名</p>
                  <p className="font-medium text-[var(--tg-text-color)]">{user.username}</p>
                </div>
              </div>
            </div>

            {/* Email */}
            <div className="p-4 bg-[var(--tg-secondary-bg-color)] rounded-xl">
              <div className="flex items-center gap-3">
                <div className="w-10 h-10 rounded-full bg-[var(--tg-button-color)]/10 flex items-center justify-center">
                  <Mail className="w-5 h-5 text-[var(--tg-button-color)]" />
                </div>
                <div className="flex-1">
                  <p className="text-sm text-[var(--tg-hint-color)]">邮箱</p>
                  <p className="font-medium text-[var(--tg-text-color)]">
                    {user.email || '未设置'}
                  </p>
                </div>
              </div>
            </div>

            {/* Join Date */}
            <div className="p-4 bg-[var(--tg-secondary-bg-color)] rounded-xl">
              <div className="flex items-center gap-3">
                <div className="w-10 h-10 rounded-full bg-[var(--tg-button-color)]/10 flex items-center justify-center">
                  <Calendar className="w-5 h-5 text-[var(--tg-button-color)]" />
                </div>
                <div className="flex-1">
                  <p className="text-sm text-[var(--tg-hint-color)]">注册时间</p>
                  <p className="font-medium text-[var(--tg-text-color)]">
                    {user.created_at ? formatDate(new Date(user.created_at * 1000)) : '未知'}
                  </p>
                </div>
              </div>
            </div>

            {/* Permissions */}
            <div className="p-4 bg-[var(--tg-secondary-bg-color)] rounded-xl">
              <div className="flex items-center gap-3">
                <div className="w-10 h-10 rounded-full bg-[var(--tg-button-color)]/10 flex items-center justify-center">
                  <Shield className="w-5 h-5 text-[var(--tg-button-color)]" />
                </div>
                <div className="flex-1">
                  <p className="text-sm text-[var(--tg-hint-color)]">权限</p>
                  <p className="font-medium text-[var(--tg-text-color)]">
                    {user.is_director ? '可查看 Agent 聊天' : '仅可查看个人信息'}
                  </p>
                </div>
              </div>
            </div>
          </div>

          {/* Info Box */}
          <div className="p-4 bg-blue-50 dark:bg-blue-900/20 rounded-xl">
            <p className="text-sm text-blue-700 dark:text-blue-300">
              <strong>提示：</strong>
              {user.is_director 
                ? '您当前是董事身份，可以查看虚拟公司中 Agent 的聊天内容。' 
                : '您当前是普通用户身份，如需查看 Agent 聊天，请联系董事局主席申请董事权限。'}
            </p>
          </div>

          {/* Logout Button */}
          <Button
            onClick={handleLogout}
            variant="danger"
            className="w-full"
            icon={<LogOut className="w-4 h-4" />}
          >
            退出登录
          </Button>
        </div>
      </div>
    </div>
  );
}
