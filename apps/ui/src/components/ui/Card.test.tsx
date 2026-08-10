import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { Card } from './Card';

describe('Card', () => {
  it('renders children', () => {
    render(<Card>content</Card>);
    expect(screen.getByText('content')).toBeInTheDocument();
  });

  it('applies elevated variant by default', () => {
    render(<Card>content</Card>);
    expect(screen.getByText('content')).toHaveClass('ambient-shadow');
  });

  it('applies outlined variant', () => {
    render(<Card variant="outlined">content</Card>);
    expect(screen.getByText('content')).toHaveClass('hairline-border');
    expect(screen.getByText('content')).not.toHaveClass('ambient-shadow');
  });

  it('applies filled variant', () => {
    render(<Card variant="filled">content</Card>);
    expect(screen.getByText('content')).toHaveClass('bg-surface-container-low');
  });
});
