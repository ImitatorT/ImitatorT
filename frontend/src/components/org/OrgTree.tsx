import { useState, useEffect, useMemo } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { ChevronRight, Users, Building2, Search, ArrowLeft, Loader2, Bot } from 'lucide-react';
import { cn } from '../../utils/helpers';
import { useChatStore } from '../../stores/appStore';
import type { Department, User as UserType } from '../../types';

interface DepartmentNodeProps {
  department: Department;
  level?: number;
  onUserClick?: (user: UserType) => void;
}

function DepartmentNode({ department, level = 0, onUserClick }: DepartmentNodeProps) {
  const [isExpanded, setIsExpanded] = useState(false);
  const hasChildren = department.children && department.children.length > 0;
  const hasUsers = department.users && department.users.length > 0;
  const isExpandable = hasChildren || hasUsers;

  const toggleExpand = () => {
    if (isExpandable) {
      setIsExpanded(!isExpanded);
    }
  };

  return (
    <div className="select-none">
      {/* Department Header */}
      <div
        className={cn(
          'flex items-center gap-3 py-2 px-4 hover:bg-black/5 dark:hover:bg-white/5 transition-colors cursor-pointer',
          level > 0 && `pl-${4 + level * 4}`
        )}
        style={{ paddingLeft: level > 0 ? `${16 + level * 16}px` : undefined }}
        onClick={toggleExpand}
      >
        {isExpandable && (
          <ChevronRight 
            className={cn(
              "w-5 h-5 text-[var(--tg-hint-color)] transition-transform",
              isExpanded && "rotate-90"
            )} 
          />
        )}
        
        <div className="w-10 h-10 bg-[var(--tg-button-color)]/10 rounded-full flex items-center justify-center">
          {hasChildren ? (
            <Building2 className="w-5 h-5 text-[var(--tg-button-color)]" />
          ) : (
            <Users className="w-5 h-5 text-[var(--tg-button-color)]" />
          )}
        </div>
        
        <div className="flex-1 min-w-0">
          <p className="font-medium text-[var(--tg-text-color)] truncate">
            {department.name}
          </p>
          <p className="text-sm text-[var(--tg-hint-color)]">
            {department.memberCount} 人
          </p>
        </div>

        {department.leader && (
          <div className="flex items-center gap-2 text-sm text-[var(--tg-hint-color)]">
            <span className={cn(
              "w-2 h-2 rounded-full",
              department.leader.status === 'online' ? 'bg-[#4ac959]' :
              department.leader.status === 'busy' ? 'bg-[#ff595a]' :
              'bg-[#9e9e9e]'
            )} />
            {department.leader.name}
          </div>
        )}
      </div>

      {/* Expanded Content */}
      <AnimatePresence>
        {isExpanded && (
          <motion.div
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: 'auto', opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ duration: 0.2 }}
            className="overflow-hidden"
          >
            {/* Child Departments */}
            {department.children?.map((child) => (
              <DepartmentNode
                key={child.id}
                department={child}
                level={level + 1}
                onUserClick={onUserClick}
              />
            ))}

            {/* Users */}
            {department.users?.map((user) => (
              <div
                key={user.id}
                className="flex items-center gap-3 py-2 px-4 hover:bg-black/5 dark:hover:bg-white/5 transition-colors cursor-pointer"
                style={{ paddingLeft: `${32 + level * 16}px` }}
                onClick={() => onUserClick?.(user)}
              >
                <div className="w-10 h-10 bg-[var(--tg-secondary-bg-color)] rounded-full flex items-center justify-center">
                  <Bot className="w-5 h-5 text-[var(--tg-hint-color)]" />
                </div>
                <div className="flex-1 min-w-0">
                  <p className="font-medium text-[var(--tg-text-color)] truncate">
                    {user.name}
                  </p>
                  <p className="text-sm text-[var(--tg-hint-color)]">
                    {user.title}
                  </p>
                </div>
                <span
                  className={cn(
                    'w-2.5 h-2.5 rounded-full',
                    user.status === 'online' && 'bg-[#4ac959]',
                    user.status === 'away' && 'bg-[#faa419]',
                    user.status === 'busy' && 'bg-[#ff595a]',
                    user.status === 'offline' && 'bg-[#9e9e9e]'
                  )}
                />
              </div>
            ))}
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}

// 递归搜索部门树
function searchDepartments(departments: Department[], query: string): Department[] {
  const lowerQuery = query.toLowerCase();
  const result: Department[] = [];
  
  for (const dept of departments) {
    const matchesName = dept.name.toLowerCase().includes(lowerQuery);
    const matchesUsers = dept.users?.some(user => 
      user.name.toLowerCase().includes(lowerQuery) ||
      user.title?.toLowerCase().includes(lowerQuery)
    );
    
    // 递归搜索子部门
    const filteredChildren = dept.children 
      ? searchDepartments(dept.children, query)
      : [];
    
    if (matchesName || matchesUsers || filteredChildren.length > 0) {
      result.push({
        ...dept,
        children: filteredChildren.length > 0 ? filteredChildren : dept.children,
      });
    }
  }
  
  return result;
}

// 计算部门总数（递归）
function countDepartments(departments: Department[]): number {
  return departments.reduce((count, dept) => {
    const childrenCount = dept.children ? countDepartments(dept.children) : 0;
    return count + 1 + childrenCount;
  }, 0);
}

export default function OrgTree() {
  const [searchQuery, setSearchQuery] = useState('');
  const { departments, isLoading, fetchDepartments, fetchAgents, agents } = useChatStore();

  // 获取组织架构数据
  useEffect(() => {
    fetchDepartments();
    fetchAgents();
  }, []);

  // 过滤数据
  const filteredData = useMemo(() => {
    if (!searchQuery.trim()) return departments;
    return searchDepartments(departments, searchQuery);
  }, [departments, searchQuery]);

  const handleUserClick = (user: UserType) => {
    console.log('Clicked user:', user);
  };

  // 计算统计数据
  const totalDepartments = useMemo(() => countDepartments(departments), [departments]);
  const onlineMembers = agents.filter(a => a.status === 'online').length;

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
          <span className="text-lg font-semibold">组织架构</span>
        </div>

        {/* Search */}
        <div className="px-4 py-2">
          <div className="relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-[var(--tg-hint-color)]" />
            <input
              type="text"
              placeholder="搜索部门或成员"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="w-full pl-10 pr-4 py-2 bg-[var(--tg-bg-color)] rounded-full text-sm text-[var(--tg-text-color)] placeholder-[var(--tg-hint-color)] focus:outline-none"
            />
          </div>
        </div>

        {/* Tree */}
        <div className="flex-1 overflow-y-auto py-2">
          {isLoading ? (
            <div className="flex flex-col items-center justify-center h-32 text-[var(--tg-hint-color)]">
              <Loader2 className="w-8 h-8 animate-spin mb-2" />
              <span className="text-sm">加载中...</span>
            </div>
          ) : filteredData.length === 0 ? (
            <div className="flex flex-col items-center justify-center h-48 text-[var(--tg-hint-color)] px-4">
              <Users className="w-12 h-12 mb-3 opacity-30" />
              <p className="text-sm text-center">
                {searchQuery ? '未找到匹配的部门或成员' : '暂无组织架构数据'}
              </p>
              {departments.length === 0 && !searchQuery && (
                <p className="text-xs text-center mt-2 opacity-70">
                  等待后端数据...
                </p>
              )}
            </div>
          ) : (
            filteredData.map((dept) => (
              <DepartmentNode
                key={dept.id}
                department={dept}
                onUserClick={handleUserClick}
              />
            ))
          )}
        </div>

        {/* Stats */}
        <div className="px-4 py-3 border-t border-[var(--tg-section-bg-color)]">
          <div className="flex justify-between text-sm text-[var(--tg-hint-color)]">
            <span>部门总数</span>
            <span className="font-medium text-[var(--tg-text-color)]">{totalDepartments}</span>
          </div>
          <div className="flex justify-between text-sm text-[var(--tg-hint-color)] mt-2">
            <span>Agent 总数</span>
            <span className="font-medium text-[var(--tg-text-color)]">{agents.length}</span>
          </div>
          <div className="flex justify-between text-sm text-[var(--tg-hint-color)] mt-2">
            <span>在线 Agent</span>
            <span className="font-medium text-green-600 dark:text-green-400">{onlineMembers}</span>
          </div>
        </div>
      </div>

      {/* Right Side - Empty State */}
      <div className="flex-1 flex flex-col items-center justify-center text-[var(--tg-hint-color)] chat-background">
        <div className="w-40 h-40 mb-8 opacity-50">
          <svg viewBox="0 0 200 200" fill="none" xmlns="http://www.w3.org/2000/svg">
            <circle cx="100" cy="80" r="40" stroke="currentColor" strokeWidth="2" opacity="0.3"/>
            <path d="M60 140C60 117.909 77.909 100 100 100C122.091 100 140 117.909 140 140" stroke="currentColor" strokeWidth="2" opacity="0.3"/>
          </svg>
        </div>
        <h3 className="text-xl font-medium mb-2">查看部门详情</h3>
        <p className="text-sm opacity-70">从左侧选择一个部门或成员</p>
      </div>
    </div>
  );
}
