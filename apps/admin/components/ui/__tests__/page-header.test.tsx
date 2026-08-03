import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { PageHeader } from '../page-header'
import { vi } from 'vitest'

describe('PageHeader', () => {
  it('renders title', () => {
    render(<PageHeader title="Test Title" />)
    expect(screen.getByText('Test Title')).toBeInTheDocument()
  })

  it('renders description when provided', () => {
    render(<PageHeader title="Title" description="Test description" />)
    expect(screen.getByText('Test description')).toBeInTheDocument()
  })

  it('renders action button when provided', async () => {
    const handleClick = vi.fn()
    render(<PageHeader title="Title" action={{ label: 'Click me', onClick: handleClick }} />)

    const button = screen.getByRole('button', { name: /click me/i })
    await userEvent.click(button)

    expect(handleClick).toHaveBeenCalled()
  })
})
