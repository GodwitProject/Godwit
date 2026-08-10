import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { Badge } from './Badge';

describe('Badge', () => {
  it('renders default variant', () => {
    render(<Badge>default</Badge>);
    const badge = screen.getByText('default');
    expect(badge).toHaveClass('bg-surface-container-high');
  });

  it('renders success variant', () => {
    render(<Badge variant="success">success</Badge>);
    expect(screen.getByText('success')).toHaveClass('bg-success/10', 'text-success');
  });

  it('renders warning variant', () => {
    render(<Badge variant="warning">warning</Badge>);
    expect(screen.getByText('warning')).toHaveClass('bg-warning/10', 'text-warning');
  });

  it('renders error variant', () => {
    render(<Badge variant="error">error</Badge>);
    expect(screen.getByText('error')).toHaveClass('bg-error/10', 'text-error');
  });

  it('renders info variant', () => {
    render(<Badge variant="info">info</Badge>);
    expect(screen.getByText('info')).toHaveClass('bg-info/10', 'text-info');
  });
});
