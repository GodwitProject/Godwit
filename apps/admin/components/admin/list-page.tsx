'use client'

import { ColumnDef } from '@tanstack/react-table'
import { PageHeader } from '@/components/ui/page-header'
import { DataTable } from '@/components/ui/data-table'
import { EmptyState } from '@/components/ui/empty-state'

export function ListPage<T>({
  data,
  columns,
  title,
  description,
  isEmpty,
  onCreateClick,
  onRowClick,
  emptyStateMessage,
}: {
  data: T[]
  columns: ColumnDef<T>[]
  title: string
  description?: string
  isEmpty?: boolean
  onCreateClick: () => void
  onRowClick?: (row: T) => void
  emptyStateMessage?: string
}) {
  return (
    <div className="space-y-6">
      <PageHeader
        title={title}
        description={description}
        action={{ label: 'Create', onClick: onCreateClick }}
      />

      {isEmpty ? (
        <EmptyState
          message={emptyStateMessage || 'No items found'}
          action={{ label: 'Create New', onClick: onCreateClick }}
        />
      ) : (
        <DataTable columns={columns} data={data} onRowClick={onRowClick} />
      )}
    </div>
  )
}
