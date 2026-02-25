import { cn } from '../../utils/helpers';
import { getInitials, generateAvatarColor } from '../../utils/helpers';

interface AvatarProps {
  src?: string;
  name: string;
  userId?: string;
  size?: 'xs' | 'sm' | 'md' | 'lg' | 'xl';
  status?: 'online' | 'away' | 'busy' | 'offline';
  className?: string;
  onClick?: () => void;
}

const sizeClasses = {
  xs: 'w-6 h-6 text-[10px]',
  sm: 'w-8 h-8 text-xs',
  md: 'w-10 h-10 text-sm',
  lg: 'w-12 h-12 text-base',
  xl: 'w-16 h-16 text-lg',
};

const statusClasses = {
  online: 'bg-green-500',
  away: 'bg-yellow-500',
  busy: 'bg-red-500',
  offline: 'bg-gray-400',
};

export default function Avatar({
  src,
  name,
  userId,
  size = 'md',
  status,
  className,
  onClick,
}: AvatarProps) {
  const bgColor = userId ? generateAvatarColor(userId) : '#0088cc';
  const initials = getInitials(name);

  return (
    <div
      className={cn(
        'relative inline-flex items-center justify-center rounded-full overflow-hidden shrink-0',
        sizeClasses[size],
        onClick && 'cursor-pointer hover:opacity-80 transition-opacity',
        className
      )}
      onClick={onClick}
    >
      {src ? (
        <img
          src={src}
          alt={name}
          className="w-full h-full object-cover"
          onError={(e) => {
            (e.target as HTMLImageElement).style.display = 'none';
          }}
        />
      ) : (
        <div
          className="w-full h-full flex items-center justify-center text-white font-medium"
          style={{ backgroundColor: bgColor }}
        >
          {initials}
        </div>
      )}
      
      {status && (
        <span
          className={cn(
            'absolute bottom-0 right-0 rounded-full border-2 border-white dark:border-gray-800',
            size === 'xs' ? 'w-2 h-2' : 'w-3 h-3',
            statusClasses[status]
          )}
        />
      )}
    </div>
  );
}
