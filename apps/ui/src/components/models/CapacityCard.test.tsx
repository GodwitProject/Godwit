import { render, screen } from '@testing-library/react';
import { CapacityCard } from './CapacityCard';

describe('CapacityCard', () => {
  it('renders capacity bars per model with formatted tokens', () => {
    render(
      <CapacityCard
        tokensPerMinByModel={
          new Map([
            ['gpt-4o', 1234],
            ['llama-3.3-70b', 50],
          ])
        }
      />
    );

    expect(screen.getByText('gpt-4o')).toBeInTheDocument();
    expect(screen.getByText('llama-3.3-70b')).toBeInTheDocument();
    expect(screen.getByText('1.2 k/mi')).toBeInTheDocument();
    expect(screen.getByText('50/mi')).toBeInTheDocument();
  });

  it('sorts by tokens descending and shows an empty state when no data', () => {
    render(<CapacityCard tokensPerMinByModel={new Map()} />);
    expect(screen.getByText('No recent activity to display.')).toBeInTheDocument();
  });
});
