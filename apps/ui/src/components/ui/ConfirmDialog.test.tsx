import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { ConfirmDialog } from './ConfirmDialog';

const renderOpen = (props: Partial<Parameters<typeof ConfirmDialog>[0]> = {}) =>
  render(
    <ConfirmDialog
      open
      title="Delete?"
      description="Are you sure?"
      onConfirm={() => {}}
      onCancel={() => {}}
      {...props}
    />
  );

describe('ConfirmDialog', () => {
  it('does not render when closed', () => {
    render(
      <ConfirmDialog
        open={false}
        title="Delete?"
        description="Are you sure?"
        onConfirm={() => {}}
        onCancel={() => {}}
      />
    );
    expect(screen.queryByText('Delete?')).not.toBeInTheDocument();
  });

  it('calls onConfirm and onCancel', () => {
    const onConfirm = vi.fn();
    const onCancel = vi.fn();
    renderOpen({ onConfirm, onCancel });
    fireEvent.click(screen.getByText('Confirm'));
    expect(onConfirm).toHaveBeenCalled();
    fireEvent.click(screen.getByText('Cancel'));
    expect(onCancel).toHaveBeenCalled();
  });

  it('calls onCancel when Escape is pressed', () => {
    const onCancel = vi.fn();
    renderOpen({ onCancel });
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(onCancel).toHaveBeenCalled();
  });

  it('has dialog accessibility attributes', () => {
    renderOpen();
    const dialog = screen.getByRole('dialog');
    expect(dialog).toHaveAttribute('aria-modal', 'true');
    const title = screen.getByText('Delete?');
    expect(dialog).toHaveAttribute('aria-labelledby', title.id);
  });
});
