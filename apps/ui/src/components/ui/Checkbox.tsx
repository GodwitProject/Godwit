import { forwardRef, useId } from 'react';
import { clsx } from '@/lib/utils';

export interface CheckboxProps {
  id?: string;
  label: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
  error?: string;
  disabled?: boolean;
}

export const Checkbox = forwardRef<HTMLInputElement, CheckboxProps>(
  ({ id, label, checked, onChange, error, disabled }, ref) => {
    const generatedId = useId();
    const inputId = id || generatedId;

    return (
      <div className="flex flex-col gap-1">
        <label htmlFor={inputId} className="flex items-center gap-2 text-body-md text-on-surface">
          <input
            ref={ref}
            id={inputId}
            type="checkbox"
            checked={checked}
            disabled={disabled}
            onChange={(e) => onChange(e.target.checked)}
            className={clsx(
              'h-4 w-4 rounded border-outline bg-surface text-primary focus:ring-primary',
              error && 'border-danger'
            )}
          />
          {label}
        </label>
        {error && <span className="text-body-sm text-danger">{error}</span>}
      </div>
    );
  }
);

Checkbox.displayName = 'Checkbox';
