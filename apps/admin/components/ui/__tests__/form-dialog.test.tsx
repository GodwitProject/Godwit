import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { FormDialog } from '../form-dialog'
import { vi } from 'vitest'

describe('FormDialog', () => {
  it('does not render when isOpen is false', () => {
    const handleSubmit = vi.fn()
    const handleClose = vi.fn()

    const { container } = render(
      <FormDialog
        isOpen={false}
        title="Test Dialog"
        onSubmit={handleSubmit}
        onClose={handleClose}
      >
        <input name="test" />
      </FormDialog>
    )

    expect(container.firstChild).toBeNull()
  })

  it('renders when isOpen is true', () => {
    const handleSubmit = vi.fn()
    const handleClose = vi.fn()

    render(
      <FormDialog
        isOpen={true}
        title="Test Dialog"
        onSubmit={handleSubmit}
        onClose={handleClose}
      >
        <input name="test" />
      </FormDialog>
    )

    expect(screen.getByText('Test Dialog')).toBeInTheDocument()
  })

  it('calls onClose when cancel button is clicked', async () => {
    const handleSubmit = vi.fn()
    const handleClose = vi.fn()

    render(
      <FormDialog
        isOpen={true}
        title="Test Dialog"
        onSubmit={handleSubmit}
        onClose={handleClose}
      >
        <input name="test" />
      </FormDialog>
    )

    const cancelButton = screen.getByRole('button', { name: /cancel/i })
    await userEvent.click(cancelButton)

    expect(handleClose).toHaveBeenCalled()
  })
})
