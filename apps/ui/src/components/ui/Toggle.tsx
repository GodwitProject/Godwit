import { InputHTMLAttributes, forwardRef, useId } from 'react';
import { clsx } from 'clsx';

export interface ToggleProps extends Omit<InputHTMLAttributes<HTMLInputElement>, 'type'> {
  label?: string;
}

export const Toggle = forwardRef<HTMLInputElement, ToggleProps>(
  ({ className, label, id, ...props }, ref) => {
    const generatedId = useId();
    const toggleId = id || generatedId;

    return (
      <label
        htmlFor={toggleId}
        className={clsx('inline-flex items-center gap-2 cursor-pointer', className)}
      >
        <input
          ref={ref}
          id={toggleId}
          type="checkbox"
          className="sr-only peer"
          {...props}
        />
        <span className="relative inline-flex h-5 w-9 items-center rounded-full bg-surface-container-high transition-colors peer-checked:bg-primary">
          <span className="inline-block h-3.5 w-3.5 transform rounded-full bg-white transition-transform peer-checked:translate-x-4" />
        </span>
        {label && <span className="text-body-base">{label}</span>}
      </label>
    );
  }
);

Toggle.displayName = 'Toggle';
