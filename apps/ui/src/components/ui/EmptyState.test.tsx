import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { EmptyState } from './EmptyState';

describe('EmptyState', () => {
  it('renders default title and message', () => {
    render(<EmptyState />);

    expect(screen.getByRole('heading', { name: 'Nothing here' })).toBeInTheDocument();
    expect(screen.getByText('No items to display.')).toBeInTheDocument();
  });

  it('renders custom title and message', () => {
    render(<EmptyState title="No users" message="Invite a user to get started." />);

    expect(screen.getByRole('heading', { name: 'No users' })).toBeInTheDocument();
    expect(screen.getByText('Invite a user to get started.')).toBeInTheDocument();
  });

  it('renders action', () => {
    render(<EmptyState action={<button>Invite</button>} />);

    expect(screen.getByRole('button', { name: 'Invite' })).toBeInTheDocument();
  });
});
