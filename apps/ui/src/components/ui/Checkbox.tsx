import { InputHTMLAttributes, forwardRef, useId } from 'react';
import { clsx } from 'clsx';

export interface CheckboxProps extends Omit<InputHTMLAttributes<HTMLInputElement>, 'type'> {
  label?: string;
}

export const Checkbox = forwardRef<HTMLInputElement, CheckboxProps>(
  ({ className, label, id, ...props }, ref) => {
    const generatedId = useId();
    const checkboxId = id || generatedId;

    return (
      <label
        htmlFor={checkboxId}
        className={clsx('inline-flex items-center gap-2 cursor-pointer', className)}
      >
        <input
          ref={ref}
          id={checkboxId}
          type="checkbox"
          className="h-4 w-4 rounded border-border accent-accent focus:ring-2 focus:ring-accent"
          {...props}
        />
        {label && <span className="text-[12.5px] text-fg">{label}</span>}
      </label>
    );
  }
);

Checkbox.displayName = 'Checkbox';
