'use client';

import { ReactNode, useEffect } from 'react';
import { clsx } from 'clsx';
import { CloseIcon } from '@/components/icons';

interface DrawerProps {
  open: boolean;
  onClose: () => void;
  title?: string;
  subtitle?: string;
  children: ReactNode;
  header?: ReactNode;
}

export function Drawer({ open, onClose, title, subtitle, children, header }: DrawerProps) {
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
    <>
      <div className={clsx('drawer-overlay', open && 'open')} onClick={onClose} aria-hidden="true" />
      <aside className={clsx('drawer-panel', open && 'open')} role="dialog" aria-modal="true">
        <div className="flex items-center gap-3 px-5 py-4 border-b border-border">
          {header}
          {title && (
            <div>
              <h3 className="text-[15px] font-semibold tracking-[-0.01em]">{title}</h3>
              {subtitle && <div className="font-mono text-[11.5px] text-muted">{subtitle}</div>}
            </div>
          )}
          <button
            type="button"
            className="grid place-items-center w-8 h-8 rounded-lg text-muted border border-border ml-auto hover:bg-bg hover:text-fg"
            onClick={onClose}
            aria-label="Close"
          >
            <CloseIcon width={15} height={15} />
          </button>
        </div>
        <div className="flex-1 overflow-y-auto px-5 py-4">{children}</div>
      </aside>
    </>
  );
}
