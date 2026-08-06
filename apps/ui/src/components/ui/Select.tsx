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
          <label htmlFor={selectId} className="text-label-sm font-medium text-on-surface-variant">
            {label}
          </label>
        )}
        <div className="relative">
          <select
            ref={ref}
            id={selectId}
            className={clsx(
              'w-full appearance-none bg-white border border-outline-variant rounded px-3 py-2 pr-8 text-body-base',
              'focus:outline-none focus:ring-2 focus:ring-primary focus:border-transparent',
              className
            )}
            {...props}
          >
            {children}
          </select>
          <span className="material-symbols-outlined pointer-events-none absolute right-2 top-1/2 -translate-y-1/2 text-sm text-on-surface-variant">
            arrow_drop_down
          </span>
        </div>
      </div>
    );
  }
);

Select.displayName = 'Select';
