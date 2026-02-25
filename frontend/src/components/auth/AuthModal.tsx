import { useState, useEffect } from 'react';
import { useAuthStore } from '../../stores/authStore';
import { useBackendStore, validateBackendUrl } from '../../stores/backendStore';
import { Modal, Button, Input } from '../ui';
import { UserPlus, LogIn, Eye, EyeOff, Check, X } from 'lucide-react';
import { cn } from '../../utils/helpers';

interface AuthModalProps {
  isOpen: boolean;
  onClose: () => void;
  defaultTab?: 'login' | 'register';
}

export default function AuthModal({ isOpen, onClose, defaultTab = 'login' }: AuthModalProps) {
  const [activeTab, setActiveTab] = useState<'login' | 'register'>(defaultTab);
  const { login, register, checkUsername } = useAuthStore();
  const { backendUrl, setBackendUrl } = useBackendStore();

  // Backend URL state
  const [backendUrlInput, setBackendUrlInput] = useState(backendUrl);
  const [backendUrlError, setBackendUrlError] = useState('');

  // Login form state
  const [loginUsername, setLoginUsername] = useState('');
  const [loginPassword, setLoginPassword] = useState('');
  const [loginLoading, setLoginLoading] = useState(false);
  const [loginError, setLoginError] = useState('');

  // Sync backendUrl from store when modal opens
  useEffect(() => {
    if (isOpen) {
      setBackendUrlInput(backendUrl);
      setBackendUrlError('');
    }
  }, [isOpen, backendUrl]);

  // Register form state
  const [regUsername, setRegUsername] = useState('');
  const [regPassword, setRegPassword] = useState('');
  const [regConfirmPassword, setRegConfirmPassword] = useState('');
  const [regName, setRegName] = useState('');
  const [regEmail, setRegEmail] = useState('');
  const [regLoading, setRegLoading] = useState(false);
  const [regError, setRegError] = useState('');
  const [usernameAvailable, setUsernameAvailable] = useState<boolean | null>(null);
  const [showPassword, setShowPassword] = useState(false);

  const handleLogin = async () => {
    // Validate backend URL first
    const validation = setBackendUrl(backendUrlInput);
    if (!validation.valid) {
      setBackendUrlError(validation.error || '后端地址格式不正确');
      return;
    }
    setBackendUrlError('');

    if (!loginUsername || !loginPassword) {
      setLoginError('请输入用户名和密码');
      return;
    }

    setLoginLoading(true);
    setLoginError('');

    const success = await login(loginUsername, loginPassword);

    setLoginLoading(false);

    if (success) {
      onClose();
      setLoginUsername('');
      setLoginPassword('');
      // 登录成功后跳转到聊天页面
      window.location.hash = '#chat';
    } else {
      setLoginError('用户名或密码错误，或无法连接到后端服务器');
    }
  };

  const handleRegister = async () => {
    // Validate backend URL first
    const validation = setBackendUrl(backendUrlInput);
    if (!validation.valid) {
      setBackendUrlError(validation.error || '后端地址格式不正确');
      return;
    }
    setBackendUrlError('');

    // Validation
    if (!regUsername || !regPassword || !regName) {
      setRegError('请填写所有必填项');
      return;
    }

    if (regUsername.length < 3) {
      setRegError('用户名至少需要3个字符');
      return;
    }

    if (regPassword.length < 6) {
      setRegError('密码至少需要6个字符');
      return;
    }

    if (regPassword !== regConfirmPassword) {
      setRegError('两次输入的密码不一致');
      return;
    }

    if (usernameAvailable === false) {
      setRegError('用户名已被占用');
      return;
    }

    setRegLoading(true);
    setRegError('');

    const success = await register(regUsername, regPassword, regName, regEmail || undefined);
    
    setRegLoading(false);

    if (success) {
      onClose();
      // Reset form
      setRegUsername('');
      setRegPassword('');
      setRegConfirmPassword('');
      setRegName('');
      setRegEmail('');
      setUsernameAvailable(null);
      // 注册成功后跳转到聊天页面
      window.location.hash = '#chat';
    } else {
      setRegError('注册失败，请稍后重试');
    }
  };

  const checkUsernameAvailability = async (username: string) => {
    if (username.length < 3) {
      setUsernameAvailable(null);
      return;
    }
    const available = await checkUsername(username);
    setUsernameAvailable(available);
  };

  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      title={activeTab === 'login' ? '用户登录' : '用户注册'}
      size="md"
    >
      <div className="space-y-4">
        {/* Tab Switcher */}
        <div className="flex rounded-lg bg-[var(--tg-secondary-bg-color)] p-1">
          <button
            onClick={() => {
              setActiveTab('login');
              setLoginError('');
              setRegError('');
            }}
            className={cn(
              'flex-1 flex items-center justify-center gap-2 py-2 rounded-md text-sm font-medium transition-colors',
              activeTab === 'login'
                ? 'bg-[var(--tg-button-color)] text-[var(--tg-button-text-color)]'
                : 'text-[var(--tg-text-color)] hover:bg-black/5 dark:hover:bg-white/5'
            )}
          >
            <LogIn className="w-4 h-4" />
            登录
          </button>
          <button
            onClick={() => {
              setActiveTab('register');
              setLoginError('');
              setRegError('');
            }}
            className={cn(
              'flex-1 flex items-center justify-center gap-2 py-2 rounded-md text-sm font-medium transition-colors',
              activeTab === 'register'
                ? 'bg-[var(--tg-button-color)] text-[var(--tg-button-text-color)]'
                : 'text-[var(--tg-text-color)] hover:bg-black/5 dark:hover:bg-white/5'
            )}
          >
            <UserPlus className="w-4 h-4" />
            注册
          </button>
        </div>

        {activeTab === 'login' ? (
          <div className="space-y-4">
            {/* Backend URL Input */}
            <Input
              label="后端服务器地址"
              placeholder="http://localhost:8080"
              value={backendUrlInput}
              onChange={(e) => {
                setBackendUrlInput(e.target.value);
                setBackendUrlError('');
              }}
              onBlur={() => {
                const result = validateBackendUrl(backendUrlInput);
                if (!result.valid) {
                  setBackendUrlError(result.error || '地址格式不正确');
                } else {
                  setBackendUrlError('');
                }
              }}
            />
            {backendUrlError && (
              <p className="text-xs text-red-500 -mt-2">{backendUrlError}</p>
            )}
            <p className="text-xs text-[var(--tg-hint-color)] -mt-2">
              支持 localhost, 127.0.0.1, 内网IP 和外网地址
            </p>

            <Input
              label="用户名"
              placeholder="请输入用户名"
              value={loginUsername}
              onChange={(e) => setLoginUsername(e.target.value)}
              onKeyDown={(e) => e.key === 'Enter' && handleLogin()}
            />
            <div className="relative">
              <Input
                label="密码"
                type={showPassword ? 'text' : 'password'}
                placeholder="请输入密码"
                value={loginPassword}
                onChange={(e) => setLoginPassword(e.target.value)}
                onKeyDown={(e) => e.key === 'Enter' && handleLogin()}
              />
              <button
                onClick={() => setShowPassword(!showPassword)}
                className="absolute right-3 top-[34px] text-[var(--tg-hint-color)] hover:text-[var(--tg-text-color)]"
              >
                {showPassword ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
              </button>
            </div>

            {loginError && (
              <div className="p-3 bg-red-50 dark:bg-red-900/20 text-red-600 dark:text-red-400 text-sm rounded-lg">
                {loginError}
              </div>
            )}

            <Button
              onClick={handleLogin}
              isLoading={loginLoading}
              className="w-full"
            >
              登录
            </Button>

            <p className="text-xs text-[var(--tg-hint-color)] text-center">
              登录后将保持登录状态，下次访问自动登录
            </p>
          </div>
        ) : (
          <div className="space-y-4">
            {/* Backend URL Input */}
            <Input
              label="后端服务器地址"
              placeholder="http://localhost:8080"
              value={backendUrlInput}
              onChange={(e) => {
                setBackendUrlInput(e.target.value);
                setBackendUrlError('');
              }}
              onBlur={() => {
                const result = validateBackendUrl(backendUrlInput);
                if (!result.valid) {
                  setBackendUrlError(result.error || '地址格式不正确');
                } else {
                  setBackendUrlError('');
                }
              }}
            />
            {backendUrlError && (
              <p className="text-xs text-red-500 -mt-2">{backendUrlError}</p>
            )}
            <p className="text-xs text-[var(--tg-hint-color)] -mt-2">
              支持 localhost, 127.0.0.1, 内网IP 和外网地址
            </p>

            <div className="relative">
              <Input
                label="用户名 *"
                placeholder="至少3个字符"
                value={regUsername}
                onChange={(e) => {
                  setRegUsername(e.target.value);
                  checkUsernameAvailability(e.target.value);
                }}
              />
              {regUsername.length >= 3 && (
                <span className="absolute right-3 top-[34px]">
                  {usernameAvailable === true ? (
                    <Check className="w-4 h-4 text-green-500" />
                  ) : usernameAvailable === false ? (
                    <X className="w-4 h-4 text-red-500" />
                  ) : null}
                </span>
              )}
            </div>
            {usernameAvailable === false && (
              <p className="text-xs text-red-500 -mt-2">用户名已被占用</p>
            )}

            <div className="relative">
              <Input
                label="密码 *"
                type={showPassword ? 'text' : 'password'}
                placeholder="至少6个字符"
                value={regPassword}
                onChange={(e) => setRegPassword(e.target.value)}
              />
              <button
                onClick={() => setShowPassword(!showPassword)}
                className="absolute right-3 top-[34px] text-[var(--tg-hint-color)] hover:text-[var(--tg-text-color)]"
              >
                {showPassword ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
              </button>
            </div>

            <Input
              label="确认密码 *"
              type="password"
              placeholder="再次输入密码"
              value={regConfirmPassword}
              onChange={(e) => setRegConfirmPassword(e.target.value)}
            />
            {regPassword && regConfirmPassword && regPassword !== regConfirmPassword && (
              <p className="text-xs text-red-500 -mt-2">两次输入的密码不一致</p>
            )}

            <Input
              label="姓名 *"
              placeholder="您的真实姓名"
              value={regName}
              onChange={(e) => setRegName(e.target.value)}
            />

            <Input
              label="邮箱"
              type="email"
              placeholder="选填"
              value={regEmail}
              onChange={(e) => setRegEmail(e.target.value)}
            />

            {regError && (
              <div className="p-3 bg-red-50 dark:bg-red-900/20 text-red-600 dark:text-red-400 text-sm rounded-lg">
                {regError}
              </div>
            )}

            <Button
              onClick={handleRegister}
              isLoading={regLoading}
              className="w-full"
            >
              注册
            </Button>

            <p className="text-xs text-[var(--tg-hint-color)] text-center">
              注册后将保持登录状态，下次访问自动登录
            </p>
          </div>
        )}
      </div>
    </Modal>
  );
}
