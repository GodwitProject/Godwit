import { HTMLAttributes, forwardRef } from 'react';
import { clsx } from '@/lib/utils';

export interface CardProps extends HTMLAttributes<HTMLDivElement> {
  variant?: 'elevated' | 'outlined' | 'filled';
}

export const Card = forwardRef<HTMLDivElement, CardProps>(
  ({ className, variant = 'elevated', ...props }, ref) => {
    return (
      <div
        ref={ref}
        className={clsx(
          'rounded-xl p-container-padding',
          {
            'bg-surface-container-lowest ambient-shadow': variant === 'elevated',
            'bg-surface-container-lowest hairline-border': variant === 'outlined',
            'bg-surface-container-low': variant === 'filled',
          },
          className
        )}
        {...props}
      />
    );
  }
);

Card.displayName = 'Card';
