import { ReactNode, useEffect, HTMLAttributes } from 'react';
import { clsx } from 'clsx';

export interface ModalProps extends HTMLAttributes<HTMLDivElement> {
  open: boolean;
  onClose: () => void;
  title?: string;
  children: ReactNode;
  maxWidth?: string;
}

export function Modal({ open, onClose, title, children, maxWidth = 'max-w-lg', className, ...props }: ModalProps) {
  useEffect(() => {
    if (!open) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    document.addEventListener('keydown', handler);
    document.body.style.overflow = 'hidden';
    return () => {
      document.removeEventListener('keydown', handler);
      document.body.style.overflow = '';
    };
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center p-4"
      role="dialog"
      aria-modal="true"
    >
      <div
        className="absolute inset-0"
        style={{ backgroundColor: 'rgba(0,0,0,0.5)' }}
        onClick={onClose}
        aria-hidden="true"
      />
      <div
        className={clsx(
          'relative bg-white rounded-xl shadow-lg max-h-[90vh] overflow-y-auto w-full p-6',
          maxWidth,
          className
        )}
        {...props}
      >
        {title && (
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-title-md">{title}</h2>
            <button
              type="button"
              onClick={onClose}
              className="material-symbols-outlined text-on-surface-variant hover:bg-surface-container-low rounded-full p-1"
              aria-label="Close"
            >
              close
            </button>
          </div>
        )}
        {children}
      </div>
    </div>
  );
}
