import { forwardRef, HTMLAttributes } from 'react';
import { clsx } from '@/lib/utils';

export const Table = forwardRef<HTMLTableElement, HTMLAttributes<HTMLTableElement>>(
  ({ className, ...props }, ref) => (
    <div className="overflow-x-auto">
      <table ref={ref} className={clsx('w-full text-left border-collapse', className)} {...props} />
    </div>
  )
);
Table.displayName = 'Table';

export const TableHead = forwardRef<HTMLTableSectionElement, HTMLAttributes<HTMLTableSectionElement>>(
  ({ className, ...props }, ref) => (
    <thead ref={ref} className={clsx('bg-surface-container-low', className)} {...props} />
  )
);
TableHead.displayName = 'TableHead';

export const TableBody = forwardRef<HTMLTableSectionElement, HTMLAttributes<HTMLTableSectionElement>>(
  ({ className, ...props }, ref) => <tbody ref={ref} className={clsx(className)} {...props} />
);
TableBody.displayName = 'TableBody';

export const TableRow = forwardRef<HTMLTableRowElement, HTMLAttributes<HTMLTableRowElement>>(
  ({ className, ...props }, ref) => (
    <tr
      ref={ref}
      className={clsx('border-b hairline-border hover:bg-surface-container-low transition-colors', className)}
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
        'py-3 px-6 text-left text-caption-xs font-medium text-on-surface-variant uppercase tracking-wider',
        className
      )}
      {...props}
    />
  )
);
TableHeadCell.displayName = 'TableHeadCell';

export const TableCell = forwardRef<HTMLTableCellElement, HTMLAttributes<HTMLTableCellElement>>(
  ({ className, ...props }, ref) => (
    <td ref={ref} className={clsx('py-3 px-6 text-body-base', className)} {...props} />
  )
);
TableCell.displayName = 'TableCell';
