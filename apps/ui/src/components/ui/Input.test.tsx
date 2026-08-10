import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Input } from './Input';

describe('Input', () => {
  it('renders a label', () => {
    render(<Input label="Email" />);
    expect(screen.getByLabelText('Email')).toBeInTheDocument();
  });

  it('shows an error message', () => {
    render(<Input label="Email" error="Required" />);
    expect(screen.getByText('Required')).toBeInTheDocument();
    expect(screen.getByLabelText('Email')).toHaveClass('border-error');
  });

  it('updates value on change', async () => {
    const onChange = vi.fn();
    render(<Input onChange={onChange} />);

    await userEvent.type(screen.getByRole('textbox'), 'hello');

    expect(onChange).toHaveBeenCalledTimes(5);
  });
});
