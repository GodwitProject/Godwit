import { HTMLAttributes, forwardRef } from 'react';
import { clsx } from 'clsx';

export const Table = forwardRef<HTMLTableElement, HTMLAttributes<HTMLTableElement>>(
  ({ className, ...props }, ref) => (
    <div className="overflow-x-auto">
      <table ref={ref} className={clsx('w-full text-left border-collapse text-[12.5px]', className)} {...props} />
    </div>
  )
);
Table.displayName = 'Table';

export const TableHead = forwardRef<HTMLTableSectionElement, HTMLAttributes<HTMLTableSectionElement>>(
  ({ className, ...props }, ref) => (
    <thead ref={ref} className={clsx('bg-surface', className)} {...props} />
  )
);
TableHead.displayName = 'TableHead';

export const TableBody = forwardRef<HTMLTableSectionElement, HTMLAttributes<HTMLTableSectionElement>>(
  ({ className, ...props }, ref) => (
    <tbody ref={ref} className={clsx(className)} {...props} />
  )
);
TableBody.displayName = 'TableBody';

export const TableRow = forwardRef<HTMLTableRowElement, HTMLAttributes<HTMLTableRowElement>>(
  ({ className, ...props }, ref) => (
    <tr
      ref={ref}
      className={clsx('border-b border-bg hover:bg-surface-2_5 transition-colors', className)}
      {...props}
    />
  )
);
TableRow.displayName = 'TableRow';

export const TableHeadCell = forwardRef<HTMLTableCellElement, HTMLAttributes<HTMLTableCellElement>>(
  ({ className, ...props }, ref) => (
    <th
      ref={ref}
      className={clsx(
        'py-2.5 px-4 text-[11px] font-medium text-muted uppercase tracking-wider whitespace-nowrap text-left border-b border-border sticky top-0 bg-surface',
        className
      )}
      {...props}
    />
  )
);
TableHeadCell.displayName = 'TableHeadCell';

export const TableCell = forwardRef<HTMLTableCellElement, HTMLAttributes<HTMLTableCellElement> & { colSpan?: number }>(
  ({ className, colSpan, ...props }, ref) => (
    <td
      ref={ref}
      colSpan={colSpan}
      className={clsx('py-2.5 px-4 whitespace-nowrap align-middle', className)}
      {...props}
    />
  )
);
TableCell.displayName = 'TableCell';
