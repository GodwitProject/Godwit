import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { Checkbox } from './Checkbox';

describe('Checkbox', () => {
  it('renders label', () => {
    render(<Checkbox label="Enable" checked={false} onChange={() => {}} />);
    expect(screen.getByLabelText('Enable')).toBeInTheDocument();
  });

  it('calls onChange when clicked', () => {
    const onChange = vi.fn();
    render(<Checkbox label="Enable" checked={false} onChange={onChange} />);
    fireEvent.click(screen.getByLabelText('Enable'));
    expect(onChange).toHaveBeenCalledWith(true);
  });

  it('displays error message', () => {
    render(<Checkbox label="Enable" checked={false} onChange={() => {}} error="Required" />);
    expect(screen.getByText('Required')).toBeInTheDocument();
  });
});
