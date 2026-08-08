import { SelectHTMLAttributes, forwardRef, useId } from 'react';
import { clsx } from 'clsx';

export interface SelectProps extends SelectHTMLAttributes<HTMLSelectElement> {
  label?: string;
}

export const Select = forwardRef<HTMLSelectElement, SelectProps>(
  ({ className, label, id, children, ...props }, ref) => {
    const generatedId = useId();
    const selectId = id || generatedId;

    return (
      <div className="flex flex-col gap-1">
        {label && (
          <label htmlFor={selectId} className="text-[11px] uppercase tracking-wider text-muted font-medium">
            {label}
          </label>
        )}
        <div className="relative">
          <select
            ref={ref}
            id={selectId}
            className={clsx(
              'w-full appearance-none bg-surface border border-border rounded-lg px-3 py-2 pr-8 text-[12.5px] text-fg font-mono',
              'focus:outline-none focus:border-accent focus:ring-2 focus:ring-accent/30',
              className
            )}
            {...props}
          >
            {children}
          </select>
          <span className="pointer-events-none absolute right-2.5 top-1/2 -translate-y-1/2 text-muted text-xs">▾</span>
        </div>
      </div>
    );
  }
);

Select.displayName = 'Select';
