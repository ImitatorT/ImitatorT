import { useState, useEffect } from 'react';
import { useAuthStore } from '../../stores/authStore';
import { Button, Input, Modal } from '../ui';
import { Key, Plus, Copy, Trash2, Calendar, Users, User, Building2, Briefcase } from 'lucide-react';

interface InviteCode {
  id: string;
  code: string;
  created_by: string;
  created_at: number;
  expiry_time: number;
  is_used: boolean;
  max_usage: number;
  current_usage: number;
}

interface User {
  id: string;
  username: string;
  name: string;
  email?: string;
  employee_id: string;
  position: string;
  department: string;
  created_at: number;
}

export default function UserManagement() {
  const { user, token } = useAuthStore();
  const [isLoading, setIsLoading] = useState(false);
  const [inviteCodes, setInviteCodes] = useState<InviteCode[]>([]);
  const [users, setUsers] = useState<User[]>([]);
  const [showCreateModal, setShowCreateModal] = useState(false);

  // 创建邀请码表单
  const [newInviteMaxUsage, setNewInviteMaxUsage] = useState(1);

  // 权限检查
  const isChairman = user?.position === 'Chairman' || user?.is_director;

  useEffect(() => {
    if (isChairman) {
      loadInviteCodes();
      loadUsers();
    }
  }, [user]);

  const loadInviteCodes = async () => {
    setIsLoading(true);
    try {
      const res = await fetch('/api/admin/invite-codes', {
        headers: {
          'Authorization': `Bearer ${token || ''}`,
        },
      });
      const data = await res.json();
      if (data.success) {
        setInviteCodes(data.data || []);
      }
    } catch (error) {
      console.error('加载邀请码失败:', error);
    } finally {
      setIsLoading(false);
    }
  };

  const loadUsers = async () => {
    setIsLoading(true);
    try {
      const res = await fetch('/api/admin/users', {
        headers: {
          'Authorization': `Bearer ${token || ''}`,
        },
      });
      const data = await res.json();
      if (data.success) {
        setUsers(data.data || []);
      }
    } catch (error) {
      console.error('加载用户列表失败:', error);
    } finally {
      setIsLoading(false);
    }
  };

  const createInviteCode = async () => {
    setIsLoading(true);
    try {
      const body = {
        max_usage: newInviteMaxUsage
      };

      const res = await fetch('/api/admin/invite-codes', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${token || ''}`,
        },
        body: JSON.stringify(body),
      });

      const data = await res.json();
      if (data.success) {
        alert('邀请码创建成功');
        setShowCreateModal(false);
        setNewInviteMaxUsage(1);
        loadInviteCodes();
      } else {
        alert(data.error || '创建失败');
      }
    } catch (error) {
      alert('网络错误');
    } finally {
      setIsLoading(false);
    }
  };

  const deleteInviteCode = async (id: string) => {
    if (!window.confirm('确定要删除这个邀请码吗？')) return;

    setIsLoading(true);
    try {
      const res = await fetch(`/api/admin/invite-codes/${id}`, {
        method: 'DELETE',
        headers: {
          'Authorization': `Bearer ${token || ''}`,
        },
      });

      const data = await res.json();
      if (data.success) {
        alert('邀请码已删除');
        loadInviteCodes();
      } else {
        alert(data.error || '删除失败');
      }
    } catch (error) {
      alert('网络错误');
    } finally {
      setIsLoading(false);
    }
  };

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text);
    alert('已复制到剪贴板');
  };

  if (!isChairman) {
    return (
      <div className="h-full flex items-center justify-center p-4">
        <div className="max-w-md w-full text-center">
          <div className="w-20 h-20 mx-auto mb-6 rounded-full bg-[var(--tg-secondary-bg-color)] flex items-center justify-center">
            <Key className="w-10 h-10 text-[var(--tg-hint-color)]" />
          </div>
          <h2 className="text-xl font-bold text-[var(--tg-text-color)] mb-2">
            权限不足
          </h2>
          <p className="text-[var(--tg-hint-color)] mb-6">
            此页面仅限集团主席访问
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="h-full flex">
      <div className="w-full max-w-6xl mx-auto bg-[var(--tg-bg-color)] flex flex-col">
        {/* Header */}
        <div className="h-14 px-4 flex items-center gap-3 border-b border-[var(--tg-section-bg-color)]">
          <span className="text-lg font-semibold">用户管理</span>
          <Button
            size="sm"
            onClick={() => setShowCreateModal(true)}
            icon={<Plus className="w-4 h-4" />}
          >
            生成邀请码
          </Button>
        </div>

        <div className="flex-1 overflow-y-auto p-4 space-y-6">
          {/* Statistics */}
          <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
            <div className="p-4 bg-[var(--tg-secondary-bg-color)] rounded-xl">
              <div className="flex items-center gap-2 mb-1">
                <Users className="w-5 h-5 text-[var(--tg-button-color)]" />
                <span className="text-xs text-[var(--tg-hint-color)]">总用户数</span>
              </div>
              <p className="text-2xl font-bold text-[var(--tg-text-color)]">{users.length}</p>
            </div>
            <div className="p-4 bg-[var(--tg-secondary-bg-color)] rounded-xl">
              <div className="flex items-center gap-2 mb-1">
                <Key className="w-5 h-5 text-blue-500" />
                <span className="text-xs text-[var(--tg-hint-color)]">可用邀请码</span>
              </div>
              <p className="text-2xl font-bold text-[var(--tg-text-color)]">
                {inviteCodes.filter(c => !c.is_used).length}
              </p>
            </div>
            <div className="p-4 bg-[var(--tg-secondary-bg-color)] rounded-xl">
              <div className="flex items-center gap-2 mb-1">
                <Briefcase className="w-5 h-5 text-purple-500" />
                <span className="text-xs text-[var(--tg-hint-color)]">管理层</span>
              </div>
              <p className="text-2xl font-bold text-[var(--tg-text-color)]">
                {users.filter(u => u.position === 'Management').length}
              </p>
            </div>
            <div className="p-4 bg-[var(--tg-secondary-bg-color)] rounded-xl">
              <div className="flex items-center gap-2 mb-1">
                <User className="w-5 h-5 text-green-500" />
                <span className="text-xs text-[var(--tg-hint-color)]">普通员工</span>
              </div>
              <p className="text-2xl font-bold text-[var(--tg-text-color)]">
                {users.filter(u => u.position === 'Employee').length}
              </p>
            </div>
          </div>

          {/* Invite Codes Section */}
          <div>
            <h3 className="text-lg font-semibold mb-3 flex items-center gap-2">
              <Key className="w-5 h-5" />
              邀请码管理
            </h3>
            <div className="overflow-x-auto">
              <table className="w-full">
                <thead>
                  <tr className="border-b border-[var(--tg-section-bg-color)]">
                    <th className="text-left py-2 px-3">邀请码</th>
                    <th className="text-left py-2 px-3">创建者</th>
                    <th className="text-left py-2 px-3">创建时间</th>
                    <th className="text-left py-2 px-3">有效期</th>
                    <th className="text-left py-2 px-3">使用情况</th>
                    <th className="text-left py-2 px-3">状态</th>
                    <th className="text-right py-2 px-3">操作</th>
                  </tr>
                </thead>
                <tbody>
                  {inviteCodes.map(code => (
                    <tr key={code.id} className="border-b border-[var(--tg-section-bg-color)]">
                      <td className="py-3 px-3">
                        <div className="flex items-center gap-2">
                          <code className="font-mono text-sm bg-gray-100 dark:bg-gray-800 px-2 py-1 rounded">
                            {code.code}
                          </code>
                          <button
                            onClick={() => copyToClipboard(code.code)}
                            className="text-[var(--tg-hint-color)] hover:text-[var(--tg-text-color)]"
                            title="复制邀请码"
                          >
                            <Copy className="w-4 h-4" />
                          </button>
                        </div>
                      </td>
                      <td className="py-3 px-3">{code.created_by}</td>
                      <td className="py-3 px-3">
                        <div className="flex items-center gap-1">
                          <Calendar className="w-4 h-4" />
                          {new Date(code.created_at * 1000).toLocaleString()}
                        </div>
                      </td>
                      <td className="py-3 px-3">
                        {new Date(code.expiry_time * 1000).toLocaleString()}
                      </td>
                      <td className="py-3 px-3">
                        {code.current_usage}/{code.max_usage}
                      </td>
                      <td className="py-3 px-3">
                        <span className={`px-2 py-1 rounded-full text-xs ${
                          !code.is_used && Date.now() < code.expiry_time * 1000
                            ? 'bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-400'
                            : 'bg-red-100 dark:bg-red-900/30 text-red-700 dark:text-red-400'
                        }`}>
                          {!code.is_used && Date.now() < code.expiry_time * 1000 ? '有效' : '已失效'}
                        </span>
                      </td>
                      <td className="py-3 px-3 text-right">
                        <button
                          onClick={() => deleteInviteCode(code.id)}
                          className="p-1 text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20 rounded"
                          title="删除邀请码"
                        >
                          <Trash2 className="w-4 h-4" />
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>

          {/* Users List */}
          <div>
            <h3 className="text-lg font-semibold mb-3 flex items-center gap-2">
              <Users className="w-5 h-5" />
              用户列表
            </h3>
            <div className="space-y-2">
              {users.map(user => (
                <div
                  key={user.id}
                  className="p-4 bg-[var(--tg-secondary-bg-color)] rounded-xl flex items-center gap-3"
                >
                  <div className="w-10 h-10 rounded-full bg-gradient-to-br from-[var(--tg-button-color)] to-[#64b5ef] flex items-center justify-center text-white font-medium">
                    {user.name.slice(0, 2)}
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2">
                      <p className="font-medium text-[var(--tg-text-color)] truncate">
                        {user.name} ({user.username})
                      </p>
                      <span className={`px-2 py-0.5 text-xs rounded-full ${
                        user.position === 'Chairman'
                          ? 'bg-yellow-100 dark:bg-yellow-900/30 text-yellow-700 dark:text-yellow-400'
                          : user.position === 'Management'
                          ? 'bg-purple-100 dark:bg-purple-900/30 text-purple-700 dark:text-purple-400'
                          : 'bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-400'
                      }`}>
                        {user.position === 'Chairman' ? '集团主席' :
                         user.position === 'Management' ? '管理层' : '员工'}
                      </span>
                    </div>
                    <div className="flex items-center gap-4 mt-1 text-sm text-[var(--tg-hint-color)]">
                      <span className="flex items-center gap-1">
                        <Briefcase className="w-3 h-3" />
                        {user.department}
                      </span>
                      <span className="flex items-center gap-1">
                        <Building2 className="w-3 h-3" />
                        {user.employee_id}
                      </span>
                      <span>
                        {new Date(user.created_at * 1000).toLocaleDateString()}
                      </span>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>

      {/* Create Invite Code Modal */}
      <Modal
        isOpen={showCreateModal}
        onClose={() => setShowCreateModal(false)}
        title="生成邀请码"
        size="md"
      >
        <div className="space-y-4">
          <p className="text-sm text-[var(--tg-hint-color)]">
            设置邀请码的使用限制，确保只有授权人员可以加入。
          </p>

          <Input
            label="最大使用次数"
            type="number"
            min="1"
            placeholder="默认为1次"
            value={newInviteMaxUsage}
            onChange={(e) => setNewInviteMaxUsage(Number(e.target.value))}
          />

          <div className="flex gap-3 pt-2">
            <Button
              onClick={() => setShowCreateModal(false)}
              variant="secondary"
              className="flex-1"
            >
              取消
            </Button>
            <Button
              onClick={createInviteCode}
              isLoading={isLoading}
              className="flex-1"
              icon={<Plus className="w-4 h-4" />}
            >
              生成
            </Button>
          </div>
        </div>
      </Modal>
    </div>
  );
}