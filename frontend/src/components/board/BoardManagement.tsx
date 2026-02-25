import { useEffect, useState } from 'react';
import { useBoardStore } from '../../stores/appStore';
import { useAuthStore } from '../../stores/authStore';
import { 
  Crown, 
  UserPlus, 
  Trash2, 
  Users,
  AlertCircle,
  ArrowLeft,
  Shield
} from 'lucide-react';
import { Button, Input, Modal } from '../ui';

import toast from 'react-hot-toast';
import type { BoardMember } from '../../types';

export default function BoardManagement() {
  const { members, fetchMembers } = useBoardStore();
  const { user: authUser, token: authToken } = useAuthStore();
  const [isLoading, setIsLoading] = useState(false);
  const [showAddModal, setShowAddModal] = useState(false);
  const [deleteConfirmMember, setDeleteConfirmMember] = useState<BoardMember | null>(null);

  // Add member form
  const [newUsername, setNewUsername] = useState('');
  const [newPassword, setNewPassword] = useState('');
  const [newName, setNewName] = useState('');

  useEffect(() => {
    if (authUser?.is_director) {
      fetchMembers();
    }
  }, [authUser, fetchMembers]);

  // Check if user is chairman
  const is_chairman = authUser?.is_director && authUser?.username === 'chairman';

  const handleAddMember = async () => {
    if (!newUsername || !newPassword || !newName) {
      toast.error('请填写所有字段');
      return;
    }

    setIsLoading(true);
    try {
      const res = await fetch('/api/board/members', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${authToken || ''}`,
        },
        body: JSON.stringify({
          username: newUsername,
          password: newPassword,
          name: newName,
        }),
      });

      const data = await res.json();
      if (data.success) {
        toast.success('董事添加成功');
        setShowAddModal(false);
        setNewUsername('');
        setNewPassword('');
        setNewName('');
        fetchMembers();
      } else {
        toast.error(data.error || '添加失败');
      }
    } catch (error) {
      toast.error('网络错误');
    } finally {
      setIsLoading(false);
    }
  };

  const handleDeleteMember = async (memberId: string) => {
    setIsLoading(true);
    try {
      const res = await fetch(`/api/board/members/${memberId}`, {
        method: 'DELETE',
        headers: {
          'Authorization': `Bearer ${authToken || ''}`,
        },
      });

      const data = await res.json();
      if (data.success) {
        toast.success('董事已删除');
        setDeleteConfirmMember(null);
        fetchMembers();
      } else {
        toast.error(data.error || '删除失败');
      }
    } catch (error) {
      toast.error('网络错误');
    } finally {
      setIsLoading(false);
    }
  };

  // Not logged in as board member
  if (!authUser?.is_director) {
    return (
      <div className="h-full flex items-center justify-center p-4">
        <div className="max-w-md w-full text-center">
          <div className="w-20 h-20 mx-auto mb-6 rounded-full bg-[var(--tg-secondary-bg-color)] flex items-center justify-center">
            <Shield className="w-10 h-10 text-[var(--tg-hint-color)]" />
          </div>
          <h2 className="text-xl font-bold text-[var(--tg-text-color)] mb-2">
            需要董事局身份
          </h2>
          <p className="text-[var(--tg-hint-color)] mb-6">
            此页面仅限董事局成员访问
          </p>
          <a 
            href="#profile"
            className="px-6 py-2 bg-[var(--tg-button-color)] text-[var(--tg-button-text-color)] rounded-lg font-medium hover:opacity-90 transition-opacity inline-block"
          >
            请先登录
          </a>
        </div>
      </div>
    );
  }

  // Not chairman
  if (!is_chairman) {
    return (
      <div className="h-full flex items-center justify-center p-4">
        <div className="max-w-md w-full text-center">
          <div className="w-20 h-20 mx-auto mb-6 rounded-full bg-red-100 dark:bg-red-900/20 flex items-center justify-center">
            <AlertCircle className="w-10 h-10 text-red-500" />
          </div>
          <h2 className="text-xl font-bold text-[var(--tg-text-color)] mb-2">
            权限不足
          </h2>
          <p className="text-[var(--tg-hint-color)] mb-6">
            只有董事局主席可以管理董事成员
          </p>
          <a href="#profile" className="text-[var(--tg-button-color)] hover:underline">
            返回个人信息
          </a>
        </div>
      </div>
    );
  }

  return (
    <div className="h-full flex">
      <div className="w-full max-w-2xl mx-auto bg-[var(--tg-bg-color)] flex flex-col">
        {/* Header */}
        <div className="h-14 px-4 flex items-center gap-3 border-b border-[var(--tg-section-bg-color)]">
          <a 
            href="#chat"
            className="p-2 -ml-2 hover:bg-black/5 dark:hover:bg-white/5 rounded-full transition-colors"
          >
            <ArrowLeft className="w-6 h-6 text-[var(--tg-hint-color)]" />
          </a>
          <div className="flex-1">
            <span className="text-lg font-semibold">董事局管理</span>
            <span className="ml-2 text-xs px-2 py-0.5 bg-purple-100 dark:bg-purple-900/30 text-purple-700 dark:text-purple-400 rounded-full">
              主席专用
            </span>
          </div>
          <Button
            size="sm"
            onClick={() => setShowAddModal(true)}
            icon={<UserPlus className="w-4 h-4" />}
          >
            添加董事
          </Button>
        </div>

        {/* Stats */}
        <div className="p-4 grid grid-cols-2 gap-3">
          <div className="p-4 bg-[var(--tg-secondary-bg-color)] rounded-xl">
            <div className="flex items-center gap-2 mb-1">
              <Users className="w-5 h-5 text-[var(--tg-button-color)]" />
              <span className="text-xs text-[var(--tg-hint-color)]">董事总数</span>
            </div>
            <p className="text-2xl font-bold text-[var(--tg-text-color)]">{members.length}</p>
          </div>
          <div className="p-4 bg-[var(--tg-secondary-bg-color)] rounded-xl">
            <div className="flex items-center gap-2 mb-1">
              <Crown className="w-5 h-5 text-yellow-500" />
              <span className="text-xs text-[var(--tg-hint-color)]">主席</span>
            </div>
            <p className="text-2xl font-bold text-[var(--tg-text-color)]">
              {members.filter(m => m.is_chairman).length}
            </p>
          </div>
        </div>

        {/* Member List */}
        <div className="flex-1 overflow-y-auto px-4 pb-4">
          <h3 className="text-sm font-medium text-[var(--tg-hint-color)] mb-3 uppercase tracking-wider">
            董事成员
          </h3>
          <div className="space-y-2">
            {members.map((member) => (
              <div
                key={member.id}
                className="p-4 bg-[var(--tg-secondary-bg-color)] rounded-xl flex items-center gap-3"
              >
                <div className="w-12 h-12 rounded-full bg-gradient-to-br from-[var(--tg-button-color)] to-[#64b5ef] flex items-center justify-center text-white font-medium">
                  {member.name.slice(0, 2)}
                </div>
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    <p className="font-medium text-[var(--tg-text-color)] truncate">
                      {member.name}
                    </p>
                    {member.is_chairman && (
                      <span className="px-2 py-0.5 bg-yellow-100 dark:bg-yellow-900/30 text-yellow-700 dark:text-yellow-400 text-xs rounded-full">
                        主席
                      </span>
                    )}
                  </div>
                  <p className="text-sm text-[var(--tg-hint-color)]">
                    @{member.username}
                  </p>
                </div>
                {!member.is_chairman && (
                  <button
                    onClick={() => setDeleteConfirmMember(member)}
                    className="p-2 text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20 rounded-full transition-colors"
                  >
                    <Trash2 className="w-5 h-5" />
                  </button>
                )}
              </div>
            ))}
          </div>
        </div>
      </div>

      {/* Add Member Modal */}
      <Modal
        isOpen={showAddModal}
        onClose={() => setShowAddModal(false)}
        title="添加董事"
        size="sm"
      >
        <div className="space-y-4">
          <p className="text-sm text-[var(--tg-hint-color)]">
            添加新董事后，该用户可以使用用户名和密码登录，并查看 Agent 聊天内容。
          </p>
          <Input
            label="用户名 *"
            placeholder="设置登录用户名"
            value={newUsername}
            onChange={(e) => setNewUsername(e.target.value)}
          />
          <Input
            label="密码 *"
            type="password"
            placeholder="设置登录密码"
            value={newPassword}
            onChange={(e) => setNewPassword(e.target.value)}
          />
          <Input
            label="姓名 *"
            placeholder="董事显示名称"
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
          />
          <Button
            onClick={handleAddMember}
            isLoading={isLoading}
            className="w-full"
          >
            添加董事
          </Button>
        </div>
      </Modal>

      {/* Delete Confirmation Modal */}
      <Modal
        isOpen={!!deleteConfirmMember}
        onClose={() => setDeleteConfirmMember(null)}
        title="确认删除"
        size="sm"
      >
        <div className="space-y-4">
          <p className="text-[var(--tg-text-color)]">
            确定要删除董事 <strong>{deleteConfirmMember?.name}</strong> 吗？
          </p>
          <p className="text-sm text-[var(--tg-hint-color)]">
            删除后，该用户将无法再查看 Agent 聊天内容。
          </p>
          <div className="flex gap-3">
            <Button
              variant="secondary"
              onClick={() => setDeleteConfirmMember(null)}
              className="flex-1"
            >
              取消
            </Button>
            <Button
              variant="danger"
              onClick={() => deleteConfirmMember && handleDeleteMember(deleteConfirmMember.id)}
              isLoading={isLoading}
              className="flex-1"
            >
              删除
            </Button>
          </div>
        </div>
      </Modal>
    </div>
  );
}
