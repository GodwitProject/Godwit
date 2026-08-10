import { forwardRef, ButtonHTMLAttributes } from 'react';
import { clsx } from '@/lib/utils';

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
          'inline-flex items-center justify-center font-medium rounded transition-colors focus:outline-none focus:ring-2 focus:ring-primary focus:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed',
          {
            'bg-primary text-on-primary hover:bg-primary/90': variant === 'primary',
            'bg-surface-container-lowest hairline-border text-on-surface hover:bg-surface-container-low': variant === 'secondary',
            'bg-transparent text-on-surface hover:bg-surface-container-high': variant === 'ghost',
            'bg-error text-on-error hover:bg-error/90': variant === 'danger',
            'text-label-sm px-3 py-1.5': size === 'sm',
            'text-body-base px-4 py-2': size === 'md',
            'text-title-md px-6 py-3': size === 'lg',
          },
          className
        )}
        {...props}
      />
    );
  }
);

Button.displayName = 'Button';
