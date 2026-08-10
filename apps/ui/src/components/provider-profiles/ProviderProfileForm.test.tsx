import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { ProviderProfileForm } from './ProviderProfileForm';

describe('ProviderProfileForm', () => {
  it('submits create payload', async () => {
    const onSubmit = vi.fn().mockResolvedValue(undefined);
    render(<ProviderProfileForm mode="create" onSubmit={onSubmit} />);

    fireEvent.input(screen.getByLabelText('Name'), { target: { value: 'openai' } });
    fireEvent.input(screen.getByLabelText('API key'), { target: { value: 'sk-test' } });
    fireEvent.click(screen.getByRole('button', { name: /create/i }));

    await waitFor(() => expect(onSubmit).toHaveBeenCalled());
    const payload = onSubmit.mock.calls[0][0];
    expect(payload.name).toBe('openai');
    expect(payload.api_key).toBe('sk-test');
  });

  it('shows validation error for empty name', async () => {
    const onSubmit = vi.fn();
    render(<ProviderProfileForm mode="create" onSubmit={onSubmit} />);
    fireEvent.click(screen.getByRole('button', { name: /create/i }));
    await waitFor(() => expect(screen.getByText(/name/i)).toBeInTheDocument());
    expect(onSubmit).not.toHaveBeenCalled();
  });
});
