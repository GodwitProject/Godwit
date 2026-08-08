import { ReactNode, useEffect, HTMLAttributes } from 'react';
import { clsx } from 'clsx';
import { CloseIcon } from '@/components/icons';

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
        className="absolute inset-0 bg-black/40"
        onClick={onClose}
        aria-hidden="true"
      />
      <div
        className={clsx(
          'relative bg-surface border border-border rounded-2xl shadow-drawer max-h-[90vh] overflow-y-auto w-full p-6',
          maxWidth,
          className
        )}
        {...props}
      >
        {title && (
          <div className="flex items-center justify-between mb-5 pb-4 border-b border-border">
            <h2 className="text-[15px] font-semibold tracking-[-0.01em]">{title}</h2>
            <button
              type="button"
              onClick={onClose}
              className="grid place-items-center w-8 h-8 rounded-lg text-muted hover:bg-surface-2"
              aria-label="Close"
            >
              <CloseIcon width={15} height={15} />
            </button>
          </div>
        )}
        {children}
      </div>
    </div>
  );
}
