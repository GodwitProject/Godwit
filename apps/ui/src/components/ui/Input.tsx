import { InputHTMLAttributes, forwardRef, useId } from 'react';
import { clsx } from 'clsx';

export interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  label?: string;
  error?: string;
}

export const Input = forwardRef<HTMLInputElement, InputProps>(
  ({ className, label, error, id, ...props }, ref) => {
    const generatedId = useId();
    const inputId = id || generatedId;

    return (
      <div className="flex flex-col gap-1">
        {label && (
          <label htmlFor={inputId} className="text-[11px] uppercase tracking-wider text-muted font-medium">
            {label}
          </label>
        )}
        <input
          ref={ref}
          id={inputId}
          className={clsx(
            'bg-surface border border-border rounded-lg px-3 py-2 text-[12.5px] text-fg font-mono',
            'focus:outline-none focus:border-accent focus:ring-2 focus:ring-accent/30',
            'placeholder:text-muted/50',
            error && 'border-danger focus:ring-danger/30',
            className
          )}
          {...props}
        />
        {error && (
          <span className="text-[11px] text-danger">{error}</span>
        )}
      </div>
    );
  }
);

Input.displayName = 'Input';
