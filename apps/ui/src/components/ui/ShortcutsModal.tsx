'use client';

import { useEffect } from 'react';
import { useT } from '@/hooks/useT';
import { CloseIcon } from '@/components/icons';

interface ShortcutsModalProps {
  open: boolean;
  onClose: () => void;
}

export function ShortcutsModal({ open, onClose }: ShortcutsModalProps) {
  const { t } = useT();

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

  const rows = [
    { label: t('shortcuts.search'), keys: ['/'] },
    { label: t('shortcuts.overview'), keys: ['G', 'D'] },
    { label: t('shortcuts.traffic'), keys: ['G', 'T'] },
    { label: t('shortcuts.keys'), keys: ['G', 'K'] },
    { label: t('shortcuts.close'), keys: ['Esc'] },
  ];

  return (
    <div className="fixed inset-0 z-50 grid place-items-center" role="dialog" aria-modal="true">
      <div className="absolute inset-0 bg-black/40" onClick={onClose} aria-hidden="true" />
      <div className="relative bg-surface border border-border rounded-2xl w-[480px] max-w-[92vw] shadow-drawer overflow-hidden">
        <div className="flex items-center justify-between px-[18px] py-3.5 border-b border-border">
          <h3 className="text-sm font-semibold tracking-[-0.01em]">{t('shortcuts.title')}</h3>
          <button
            type="button"
            className="grid place-items-center w-8 h-8 rounded-lg text-muted border border-border hover:bg-bg hover:text-fg"
            onClick={onClose}
            aria-label="Close"
          >
            <CloseIcon width={15} height={15} />
          </button>
        </div>
        <div className="px-[18px] py-3">
          {rows.map((row, i) => (
            <div
              key={i}
              className="flex items-center justify-between gap-4 py-2 border-b border-bg last:border-b-0"
            >
              <span className="text-[12.5px]">{row.label}</span>
              <span className="flex gap-1">
                {row.keys.map((k, j) => (
                  <kbd key={j} className="font-mono text-[11px] text-muted border border-border rounded px-1.5 py-1 bg-surface">
                    {k}
                  </kbd>
                ))}
              </span>
            </div>
          ))}
          <p className="font-mono text-[11px] text-muted mt-2">{t('shortcuts.hint')}</p>
        </div>
      </div>
    </div>
  );
}
