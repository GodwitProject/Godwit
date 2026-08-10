import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { PageHeader } from './PageHeader';

describe('PageHeader', () => {
  it('renders title and description', () => {
    render(<PageHeader title="Users" description="Manage users" />);

    expect(screen.getByRole('heading', { name: 'Users' })).toBeInTheDocument();
    expect(screen.getByText('Manage users')).toBeInTheDocument();
  });

  it('renders action', () => {
    render(<PageHeader title="Users" action={<button>Add</button>} />);

    expect(screen.getByRole('button', { name: 'Add' })).toBeInTheDocument();
  });
});
