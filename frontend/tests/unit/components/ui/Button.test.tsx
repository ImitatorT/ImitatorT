import { render, screen } from '@testing-library/react';
import { describe, it, expect } from '@jest/globals';
import Button from '@/components/ui/Button';

describe('Button', () => {
  it('renders correctly with children', () => {
    render(<Button>Click me</Button>);
    expect(screen.getByText('Click me')).toBeInTheDocument();
  });

  it('applies primary variant class', () => {
    render(<Button variant="primary">Primary Button</Button>);
    const button = screen.getByRole('button', { name: /Primary Button/i });
    expect(button).toHaveClass('bg-[var(--tg-button-color)]');
  });

  it('applies secondary variant class', () => {
    render(<Button variant="secondary">Secondary Button</Button>);
    const button = screen.getByRole('button', { name: /Secondary Button/i });
    expect(button).toHaveClass('bg-[var(--tg-secondary-bg-color)]');
  });

  it('shows loading state', () => {
    render(<Button isLoading={true}>Loading Button</Button>);
    const button = screen.getByRole('button', { name: /Loading Button/i });
    expect(button).toBeDisabled();
    // The loader is present when isLoading is true - visually checked in the DOM
    expect(button.querySelector('svg')).toBeInTheDocument();
  });

  it('handles click events', () => {
    const handleClick = jest.fn();
    render(<Button onClick={handleClick}>Clickable Button</Button>);
    const button = screen.getByRole('button', { name: /Clickable Button/i });
    button.click();
    expect(handleClick).toHaveBeenCalledTimes(1);
  });

  it('renders with icon', () => {
    const { container } = render(
      <Button leftIcon={<span data-testid="icon">I</span>}>Button with Icon</Button>
    );
    expect(container.querySelector('[data-testid="icon"]')).toBeInTheDocument();
  });
});