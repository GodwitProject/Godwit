import { render, screen, fireEvent } from '@testing-library/react';
import { KeyForm, type KeyFormProps } from './KeyForm';
import type { CreatedKey } from '../../lib/keys';

const owners = ['Platform Team', 'Growth', 'Data Science'];
const models = ['gpt-4', 'claude-3-opus'];

function baseProps(overrides: Partial<KeyFormProps> = {}): KeyFormProps {
  return {
    open: true,
    owners,
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
    expect(screen.getByLabelText('Owner')).toBeInTheDocument();
    expect(screen.getByText('read')).toBeInTheDocument();
    expect(screen.getByText('write')).toBeInTheDocument();
    expect(screen.getByText('admin')).toBeInTheDocument();
    expect(screen.getByText('Budget (USD, optional)')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Create Key' })).toBeInTheDocument();
  });

  it('submits a create request with selected fields', async () => {
    const onSubmit = vi.fn().mockResolvedValue(undefined);
    render(<KeyForm {...baseProps({ onSubmit })} />);

    fireEvent.change(screen.getByLabelText('Name'), { target: { value: 'Prod Key' } });
    fireEvent.click(screen.getByLabelText('write'));
    fireEvent.change(screen.getByLabelText('Owner'), { target: { value: 'Growth' } });
    fireEvent.change(screen.getByLabelText('Budget (USD, optional)'), { target: { value: '50' } });

    fireEvent.click(screen.getByRole('button', { name: 'Create Key' }));

    expect(onSubmit).toHaveBeenCalledTimes(1);
    const req = onSubmit.mock.calls[0][0];
    expect(req.name).toBe('Prod Key');
    expect(req.owner).toBe('Growth');
    expect(req.scopes).toContain('write');
    expect(req.budget).toBe(50);
  });

  it('shows the full key once with warning after creation', async () => {
    const created: CreatedKey = {
      key: {} as CreatedKey['key'],
      fullKey: 'sk_live_fullsecret123',
    };
    const onSubmit = vi.fn().mockResolvedValue(created);
    render(<KeyForm {...baseProps({ onSubmit })} />);

    fireEvent.change(screen.getByLabelText('Name'), { target: { value: 'Prod Key' } });
    fireEvent.click(screen.getByRole('button', { name: 'Create Key' }));

    await screen.findByText('sk_live_fullsecret123');
    expect(screen.getByText("Copy this key now. You won't see it again.")).toBeInTheDocument();
  });
});
