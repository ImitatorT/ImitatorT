import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect } from '@jest/globals';
import Input from '@/components/ui/Input';

describe('Input', () => {
  it('renders correctly with label and placeholder', () => {
    render(<Input label="Username" placeholder="Enter your username" />);
    expect(screen.getByText('Username')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('Enter your username')).toBeInTheDocument();
  });

  it('displays error state', () => {
    render(<Input label="Email" error="Invalid email format" />);
    expect(screen.getByText('Email')).toBeInTheDocument();
    expect(screen.getByText('Invalid email format')).toBeInTheDocument();
  });

  it('handles text input changes', () => {
    const handleChange = jest.fn();
    render(<Input label="Name" onChange={handleChange} />);

    const input = screen.getByRole('textbox');
    fireEvent.change(input, { target: { value: 'John Doe' } });

    expect(handleChange).toHaveBeenCalledTimes(1);
    expect(input).toHaveValue('John Doe');
  });

  it('handles blur events', () => {
    const handleBlur = jest.fn();
    render(<Input label="City" onBlur={handleBlur} />);

    const input = screen.getByRole('textbox');
    fireEvent.blur(input);

    expect(handleBlur).toHaveBeenCalledTimes(1);
  });

  it('can be disabled', () => {
    render(<Input label="Disabled Input" disabled={true} />);

    const input = screen.getByRole('textbox');
    expect(input).toBeDisabled();
  });

  it('renders with different types', () => {
    const { container: container1 } = render(<Input label="Text Input" type="text" />);
    const textInput = container1.querySelector('input[type="text"]');
    expect(textInput).toBeInTheDocument();
    expect(textInput).toHaveAttribute('type', 'text');

    const { container: container2 } = render(<Input label="Password Input" type="password" />);
    const passwordInput = container2.querySelector('input[type="password"]');
    expect(passwordInput).toBeInTheDocument();
    expect(passwordInput).toHaveAttribute('type', 'password');

    const { container: container3 } = render(<Input label="Email Input" type="email" />);
    const emailInput = container3.querySelector('input[type="email"]');
    expect(emailInput).toBeInTheDocument();
    expect(emailInput).toHaveAttribute('type', 'email');
  });
});