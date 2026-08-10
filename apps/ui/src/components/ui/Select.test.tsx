import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { Select } from './Select';

const options = [
  { value: 'openai', label: 'OpenAI' },
  { value: 'anthropic', label: 'Anthropic' },
];

describe('Select', () => {
  it('renders options', () => {
    render(<Select label="Protocol" value="" options={options} onChange={() => {}} />);
    expect(screen.getByRole('combobox')).toHaveValue('');
    expect(screen.getByText('OpenAI')).toBeInTheDocument();
  });

  it('calls onChange with selected value', () => {
    const onChange = vi.fn();
    render(<Select label="Protocol" value="" options={options} onChange={onChange} />);
    fireEvent.change(screen.getByRole('combobox'), { target: { value: 'anthropic' } });
    expect(onChange).toHaveBeenCalledWith('anthropic');
  });

  it('shows error', () => {
    render(<Select label="Protocol" value="" options={options} onChange={() => {}} error="Required" />);
    expect(screen.getByText('Required')).toBeInTheDocument();
  });
});
