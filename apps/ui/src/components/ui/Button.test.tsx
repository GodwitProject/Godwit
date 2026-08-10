import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { Button } from './Button';

describe('Button', () => {
  it('renders primary variant by default', () => {
    render(<Button>Click</Button>);
    const button = screen.getByRole('button');
    expect(button).toHaveClass('bg-primary');
  });

  it('applies secondary variant', () => {
    render(<Button variant="secondary">Click</Button>);
    const button = screen.getByRole('button');
    expect(button).toHaveClass('hairline-border');
  });
});
