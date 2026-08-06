import { render, screen, fireEvent } from '@testing-library/react';
import { KeyForm, type KeyFormProps } from './KeyForm';
import type { CreatedKey } from '../../lib/keys';

const models = ['gpt-4', 'claude-3-opus'];

function baseProps(overrides: Partial<KeyFormProps> = {}): KeyFormProps {
  return {
    open: true,
    availableModels: models,
    onClose: () => {},
    onSubmit: async () => {},
    ...overrides,
  };
}

describe('KeyForm', () => {
  it('renders create form fields and scopes checkboxes', () => {
    render(<KeyForm {...baseProps()} />);

    expect(screen.getByLabelText('Name')).toBeInTheDocument();
    expect(screen.getByLabelText('read')).toBeInTheDocument();
    expect(screen.getByLabelText('write')).toBeInTheDocument();
    expect(screen.getByLabelText('admin')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Create Key' })).toBeInTheDocument();
  });

  it('submits a create request with selected fields', async () => {
    const onSubmit = vi.fn().mockResolvedValue(undefined);
    render(<KeyForm {...baseProps({ onSubmit })} />);

    fireEvent.change(screen.getByLabelText('Name'), { target: { value: 'Prod Key' } });
    fireEvent.click(screen.getAllByLabelText('write')[0]);
    fireEvent.change(screen.getByLabelText('Rate Limit RPM (optional)'), { target: { value: '1000' } });

    fireEvent.click(screen.getByRole('button', { name: 'Create Key' }));

    expect(onSubmit).toHaveBeenCalledTimes(1);
    const req = onSubmit.mock.calls[0][0];
    expect(req.name).toBe('Prod Key');
    expect(req.scopes).toContain('write');
    expect(req.rate_limit_requests_per_minute).toBe(1000);
  });

  it('shows the full key once with warning after creation', async () => {
    const created: CreatedKey = {
      id: 'key-1',
      key: 'sk_live_fullsecret123',
      name: 'Prod Key',
    };
    const onSubmit = vi.fn().mockResolvedValue(created);
    render(<KeyForm {...baseProps({ onSubmit })} />);

    fireEvent.change(screen.getByLabelText('Name'), { target: { value: 'Prod Key' } });
    fireEvent.click(screen.getByRole('button', { name: 'Create Key' }));

    await screen.findByText('sk_live_fullsecret123');
    expect(screen.getByText("Copy this key now. You won't see it again.")).toBeInTheDocument();
  });
});
