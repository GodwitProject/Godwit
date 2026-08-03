import { render, screen } from '@testing-library/react'
import { ColumnDef } from '@tanstack/react-table'
import { DataTable } from '../data-table'

interface TestRow {
  id: string
  name: string
}

describe('DataTable', () => {
  const columns: ColumnDef<TestRow>[] = [
    {
      accessorKey: 'name',
      header: 'Name',
    },
  ]

  const data: TestRow[] = [
    { id: '1', name: 'Row 1' },
    { id: '2', name: 'Row 2' },
  ]

  it('renders table with data', () => {
    render(<DataTable columns={columns} data={data} />)
    expect(screen.getByText('Row 1')).toBeInTheDocument()
    expect(screen.getByText('Row 2')).toBeInTheDocument()
  })

  it('renders "No data" when data is empty', () => {
    render(<DataTable columns={columns} data={[]} />)
    expect(screen.getByText('No data')).toBeInTheDocument()
  })

  it('renders column headers', () => {
    render(<DataTable columns={columns} data={data} />)
    expect(screen.getByText('Name')).toBeInTheDocument()
  })
})
