import { forwardRef, useId } from 'react';
import { clsx } from '@/lib/utils';

interface SelectOption {
  value: string;
  label: string;
}

export interface SelectProps {
  id?: string;
  label: string;
  value: string;
  options: SelectOption[];
  onChange: (value: string) => void;
  error?: string;
  placeholder?: string;
}

export const Select = forwardRef<HTMLSelectElement, SelectProps>(
  ({ id, label, value, options, onChange, error, placeholder }, ref) => {
    const generatedId = useId();
    const selectId = id || generatedId;

    return (
      <div className="flex flex-col gap-1">
        <label htmlFor={selectId} className="text-body-md text-on-surface">
          {label}
        </label>
        <select
          ref={ref}
          id={selectId}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          className={clsx(
            'rounded-lg border bg-surface px-3 py-2 text-body-md text-on-surface outline-none focus:border-primary focus:ring-1 focus:ring-primary',
            error ? 'border-danger' : 'border-outline'
          )}
        >
          {(placeholder || value === '') && (
            <option value="" disabled>
              {placeholder ?? ''}
            </option>
          )}
          {options.map((opt) => (
            <option key={opt.value} value={opt.value}>
              {opt.label}
            </option>
          ))}
        </select>
        {error && <span className="text-body-sm text-danger">{error}</span>}
      </div>
    );
  }
);

Select.displayName = 'Select';
