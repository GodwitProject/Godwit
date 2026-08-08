import { HTMLAttributes, forwardRef } from 'react';
import { clsx } from 'clsx';

export interface BadgeProps extends HTMLAttributes<HTMLSpanElement> {
  variant?: 'default' | 'success' | 'warning' | 'error' | 'info';
}

export const Badge = forwardRef<HTMLSpanElement, BadgeProps>(
  ({ className, variant = 'default', ...props }, ref) => {
    return (
      <span
        ref={ref}
        className={clsx(
          'inline-flex items-center px-2 py-0.5 rounded text-[11px] font-medium',
          {
            'bg-bg text-muted border border-border': variant === 'default',
            'text-success bg-[oklch(96%_0.03_155)] border border-[oklch(88%_0.06_155)]': variant === 'success',
            'text-warn bg-[oklch(97%_0.04_80)] border border-[oklch(90%_0.08_80)]': variant === 'warning',
            'text-danger bg-[oklch(97%_0.03_25)] border border-[oklch(90%_0.06_25)]': variant === 'error',
            'text-[oklch(40%_0.14_260)] bg-[oklch(97%_0.02_260)] border border-[oklch(90%_0.05_260)]': variant === 'info',
          },
          className
        )}
        {...props}
      />
    );
  }
);

Badge.displayName = 'Badge';
