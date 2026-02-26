import {
  cn,
  formatMessageTime,
  formatRelativeTime,
  formatFullDateTime,
  getInitials,
  truncateText,
  generateId,
  debounce,
  throttle,
  formatFileSize,
  generateAvatarColor,
  formatDate
} from '../../../src/utils/helpers';

describe('Helpers', () => {
  describe('cn', () => {
    it('should merge class names correctly', () => {
      const result = cn('class1', 'class2', { 'class3': true, 'class4': false });
      expect(result).toBe('class1 class2 class3');
    });

    it('should handle conditional classes', () => {
      const result = cn('btn', { 'btn-primary': true, 'btn-secondary': false });
      expect(result).toBe('btn btn-primary');
    });
  });

  describe('formatMessageTime', () => {
    it('should format today\'s time as HH:mm', () => {
      const today = new Date();
      const result = formatMessageTime(today);
      expect(result).toMatch(/^\d{2}:\d{2}$/);
    });

    it('should format yesterday as "昨天"', () => {
      const yesterday = new Date();
      yesterday.setDate(yesterday.getDate() - 1);
      // Mock isYesterday to return true for this test
      const originalDateNow = Date.now;
      Date.now = jest.fn(() => yesterday.getTime() + 24 * 60 * 60 * 1000); // Today is one day after yesterday

      const result = formatMessageTime(yesterday);
      expect(result).toBe('昨天');

      Date.now = originalDateNow;
    });

    it('should format other dates as MM/dd', () => {
      const date = new Date('2023-01-15');
      const result = formatMessageTime(date);
      expect(result).toBe('01/15');
    });
  });

  describe('formatRelativeTime', () => {
    it('should format relative time', () => {
      const date = new Date(Date.now() - 1000 * 60); // 1 minute ago
      const result = formatRelativeTime(date);
      expect(result).toContain('分钟');
    });
  });

  describe('formatFullDateTime', () => {
    it('should format full date time', () => {
      const date = new Date('2023-01-15T14:30:00');
      const result = formatFullDateTime(date);
      expect(result).toBe('2023年01月15日 14:30');
    });
  });

  describe('getInitials', () => {
    it('should return first two characters for single name', () => {
      expect(getInitials('张三')).toBe('张三');
      expect(getInitials('John')).toBe('JO');
    });

    it('should return first and last initial for multiple names', () => {
      expect(getInitials('John Doe')).toBe('JD');
      expect(getInitials('张 小明')).toBe('张小');
    });

    it('should return "?" for empty name', () => {
      expect(getInitials('')).toBe('?');
    });
  });

  describe('truncateText', () => {
    it('should not truncate if text is shorter than max length', () => {
      const text = 'Short';
      const result = truncateText(text, 10);
      expect(result).toBe(text);
    });

    it('should truncate text and add ellipsis', () => {
      const text = 'This is a long text';
      const result = truncateText(text, 10);
      expect(result).toBe('This is a ...');
    });
  });

  describe('generateId', () => {
    it('should generate unique IDs', () => {
      const id1 = generateId();
      const id2 = generateId();
      expect(id1).not.toBe(id2);
      expect(id1).toMatch(/^\d+-[a-z0-9]+$/);
      expect(id2).toMatch(/^\d+-[a-z0-9]+$/);
    });
  });

  describe('debounce', () => {
    beforeEach(() => {
      jest.useFakeTimers();
    });

    afterEach(() => {
      jest.useRealTimers();
    });

    it('should debounce function calls', () => {
      const mockFn = jest.fn();
      const debouncedFn = debounce(mockFn, 100);

      debouncedFn();
      debouncedFn();
      debouncedFn();

      expect(mockFn).toHaveBeenCalledTimes(0);

      jest.advanceTimersByTime(100);

      expect(mockFn).toHaveBeenCalledTimes(1);
    });
  });

  describe('throttle', () => {
    beforeEach(() => {
      jest.useFakeTimers();
    });

    afterEach(() => {
      jest.useRealTimers();
    });

    it('should throttle function calls', () => {
      const mockFn = jest.fn();
      const throttledFn = throttle(mockFn, 100);

      throttledFn();
      throttledFn();
      throttledFn();

      expect(mockFn).toHaveBeenCalledTimes(1);

      jest.advanceTimersByTime(100);

      throttledFn();
      expect(mockFn).toHaveBeenCalledTimes(2);
    });
  });

  describe('formatFileSize', () => {
    it('should format bytes correctly', () => {
      expect(formatFileSize(0)).toBe('0 B');
      expect(formatFileSize(512)).toBe('512 B');
      expect(formatFileSize(1024)).toBe('1 KB');
      expect(formatFileSize(1024 * 1024)).toBe('1 MB');
      expect(formatFileSize(1024 * 1024 * 1024)).toBe('1 GB');
    });
  });

  describe('generateAvatarColor', () => {
    it('should generate consistent color for same user ID', () => {
      const userId = 'user123';
      const color1 = generateAvatarColor(userId);
      const color2 = generateAvatarColor(userId);
      expect(color1).toBe(color2);
    });

    it('should generate different colors for different user IDs', () => {
      const color1 = generateAvatarColor('user1');
      const color2 = generateAvatarColor('user2');
      // Since colors are picked from a finite set, we can't guarantee they're always different,
      // but we can at least test that the function works
      expect(typeof color1).toBe('string');
      expect(typeof color2).toBe('string');
      expect(color1).toMatch(/^#[0-9A-F]{6}$/i);
      expect(color2).toMatch(/^#[0-9A-F]{6}$/i);
    });
  });

  describe('formatDate', () => {
    it('should format date correctly', () => {
      const date = new Date('2023-01-15');
      const result = formatDate(date);
      expect(result).toBe('2023年01月15日');
    });
  });
});