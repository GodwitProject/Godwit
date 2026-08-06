import { HTMLAttributes, forwardRef } from 'react';
import { clsx } from 'clsx';

export interface CardProps extends HTMLAttributes<HTMLDivElement> {
  variant?: 'elevated' | 'outlined' | 'filled';
}

export const Card = forwardRef<HTMLDivElement, CardProps>(
  ({ className, variant = 'elevated', ...props }, ref) => {
    return (
      <div
        ref={ref}
        className={clsx(
          'bg-surface-container-lowest rounded-xl p-container-padding',
          {
            'ambient-shadow': variant === 'elevated',
            'hairline-border': variant === 'outlined',
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
