import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { Table, TableHead, TableBody, TableRow, TableHeadCell, TableCell } from './Table';

describe('Table', () => {
  it('renders head, body, rows, and cells', () => {
    render(
      <Table>
        <TableHead>
          <TableRow>
            <TableHeadCell>Name</TableHeadCell>
            <TableHeadCell>Role</TableHeadCell>
          </TableRow>
        </TableHead>
        <TableBody>
          <TableRow>
            <TableCell>Ada</TableCell>
            <TableCell>Admin</TableCell>
          </TableRow>
        </TableBody>
      </Table>
    );

    expect(screen.getByRole('table')).toBeInTheDocument();
    expect(screen.getAllByRole('columnheader')).toHaveLength(2);
    expect(screen.getByRole('cell', { name: 'Ada' })).toBeInTheDocument();
    expect(screen.getByRole('cell', { name: 'Admin' })).toBeInTheDocument();
    expect(screen.getAllByRole('row')).toHaveLength(2);
  });
});
