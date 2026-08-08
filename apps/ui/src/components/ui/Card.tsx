import { HTMLAttributes, forwardRef } from 'react';
import { clsx } from 'clsx';

export interface CardProps extends HTMLAttributes<HTMLDivElement> {
  variant?: 'elevated' | 'outlined' | 'filled';
}

export const Card = forwardRef<HTMLDivElement, CardProps>(
  ({ className, variant = 'outlined', ...props }, ref) => {
    return (
      <div
        ref={ref}
        className={clsx(
          'bg-surface rounded-xl',
          {
            'shadow-ambient': variant === 'elevated',
            'border border-border': variant === 'outlined',
            'bg-surface-2': variant === 'filled',
          },
          className
        )}
        {...props}
      />
    );
  }
);

Card.displayName = 'Card';
