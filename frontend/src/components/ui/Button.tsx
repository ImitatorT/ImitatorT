import { cn } from '../../utils/helpers';
import { Loader2 } from 'lucide-react';

interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'ghost' | 'danger' | 'success';
  size?: 'sm' | 'md' | 'lg';
  isLoading?: boolean;
  leftIcon?: React.ReactNode;
  rightIcon?: React.ReactNode;
  icon?: React.ReactNode;
}

const variantClasses = {
  primary: 'bg-[var(--tg-button-color)] text-[var(--tg-button-text-color)] hover:opacity-90 active:opacity-80',
  secondary: 'bg-[var(--tg-secondary-bg-color)] text-[var(--tg-text-color)] hover:bg-[var(--tg-section-bg-color)]',
  ghost: 'bg-transparent text-[var(--tg-text-color)] hover:bg-[var(--tg-secondary-bg-color)]',
  danger: 'bg-red-500 text-white hover:bg-red-600 active:bg-red-700',
  success: 'bg-green-500 text-white hover:bg-green-600 active:bg-green-700',
};

const sizeClasses = {
  sm: 'px-3 py-1.5 text-sm',
  md: 'px-4 py-2 text-base',
  lg: 'px-6 py-3 text-lg',
};

export default function Button({
  children,
  variant = 'primary',
  size = 'md',
  isLoading = false,
  leftIcon,
  rightIcon,
  icon,
  className,
  disabled,
  ...props
}: ButtonProps) {
  return (
    <button
      className={cn(
        'inline-flex items-center justify-center font-medium rounded-lg transition-all',
        'focus:outline-none focus:ring-2 focus:ring-[var(--tg-button-color)] focus:ring-offset-2',
        'disabled:opacity-50 disabled:cursor-not-allowed',
        variantClasses[variant],
        sizeClasses[size],
        className
      )}
      disabled={disabled || isLoading}
      {...props}
    >
      {isLoading && <Loader2 className="w-4 h-4 mr-2 animate-spin" />}
      {!isLoading && (leftIcon || icon) && <span className="mr-2">{leftIcon || icon}</span>}
      {children}
      {!isLoading && rightIcon && <span className="ml-2">{rightIcon}</span>}
    </button>
  );
}
