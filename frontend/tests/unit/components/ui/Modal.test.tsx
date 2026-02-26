import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect } from '@jest/globals';
import Modal from '@/components/ui/Modal';

describe('Modal', () => {
  const mockOnClose = jest.fn();

  beforeEach(() => {
    mockOnClose.mockClear();
  });

  it('renders modal with title and children', () => {
    render(
      <Modal isOpen={true} onClose={mockOnClose} title="Test Modal">
        <p>Modal content</p>
      </Modal>
    );

    expect(screen.getByText('Test Modal')).toBeInTheDocument();
    expect(screen.getByText('Modal content')).toBeInTheDocument();
  });

  it('does not render when not open', () => {
    render(
      <Modal isOpen={false} onClose={mockOnClose} title="Test Modal">
        <p>Modal content</p>
      </Modal>
    );

    expect(screen.queryByText('Test Modal')).not.toBeInTheDocument();
    expect(screen.queryByText('Modal content')).not.toBeInTheDocument();
  });

  it('calls onClose when close button is clicked', () => {
    render(
      <Modal isOpen={true} onClose={mockOnClose} title="Test Modal">
        <p>Modal content</p>
      </Modal>
    );

    const closeButton = screen.getByRole('button'); // The close button in the header
    fireEvent.click(closeButton);

    expect(mockOnClose).toHaveBeenCalledTimes(1);
  });

  it('calls onClose when backdrop is clicked', () => {
    const { container } = render(
      <Modal isOpen={true} onClose={mockOnClose} title="Test Modal">
        <p>Modal content</p>
      </Modal>
    );

    // Find the backdrop element (overlay) - it has class "fixed inset-0 bg-black/50 z-40"
    const backdrop = container.querySelector('.fixed.inset-0.bg-black\\/50.z-40');
    if (backdrop) {
      fireEvent.click(backdrop);
    }

    expect(mockOnClose).toHaveBeenCalledTimes(1);
  });

  it('does not close when clicking inside modal content', () => {
    render(
      <Modal isOpen={true} onClose={mockOnClose} title="Test Modal">
        <div data-testid="modal-content">
          <p>Modal content</p>
          <button>Inside button</button>
        </div>
      </Modal>
    );

    const content = screen.getByTestId('modal-content');
    fireEvent.click(content);

    expect(mockOnClose).not.toHaveBeenCalled();
  });

  it('renders with different sizes', () => {
    const { container: container1 } = render(
      <Modal isOpen={true} onClose={mockOnClose} title="Test Modal" size="sm">
        <p>Small modal content</p>
      </Modal>
    );

    // Check for small modal size class - find the modal div by looking for the size class
    const modalSm = container1.querySelector('.max-w-sm');
    expect(modalSm).toBeInTheDocument();

    const { container: container2 } = render(
      <Modal isOpen={true} onClose={mockOnClose} title="Test Modal" size="lg">
        <p>Large modal content</p>
      </Modal>
    );

    // Check for large modal size class
    const modalLg = container2.querySelector('.max-w-lg');
    expect(modalLg).toBeInTheDocument();
  });
});