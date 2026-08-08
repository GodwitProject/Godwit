import { ButtonHTMLAttributes, forwardRef } from 'react';
import { clsx } from 'clsx';

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'ghost' | 'danger';
  size?: 'sm' | 'md' | 'lg';
}

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant = 'primary', size = 'md', ...props }, ref) => {
    return (
      <button
        ref={ref}
        className={clsx(
          'inline-flex items-center justify-center gap-1.5 rounded-lg font-medium transition-colors',
          {
            'bg-accent text-on-accent hover:bg-accent-strong': variant === 'primary',
            'bg-surface border border-border text-fg hover:bg-bg': variant === 'secondary',
            'bg-transparent text-fg hover:bg-surface-2': variant === 'ghost',
            'bg-danger text-white hover:opacity-90': variant === 'danger',
            'text-[12.5px] px-2.5 py-1.5': size === 'sm',
            'text-[12.5px] px-3 py-[7px]': size === 'md',
            'text-sm px-5 py-2.5': size === 'lg',
          },
          className
        )}
        {...props}
      />
    );
  }
);

Button.displayName = 'Button';
