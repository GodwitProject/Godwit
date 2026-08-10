import { clsx } from '@/lib/utils';

interface CheckboxProps {
  id?: string;
  label: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
  error?: string;
  disabled?: boolean;
}

export function Checkbox({ id, label, checked, onChange, error, disabled }: CheckboxProps) {
  const inputId = id ?? label;
  return (
    <div className="flex flex-col gap-1">
      <label htmlFor={inputId} className="flex items-center gap-2 text-body-md text-on-surface">
        <input
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
