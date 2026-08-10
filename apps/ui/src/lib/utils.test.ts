import { describe, it, expect } from 'vitest';
import { clsx } from './utils';

describe('clsx', () => {
  it('joins strings', () => {
    expect(clsx('a', 'b')).toBe('a b');
  });

  it('flattens arrays', () => {
    expect(clsx(['a', 'b'])).toBe('a b');
  });

  it('includes object keys with truthy values', () => {
    expect(clsx({ active: true, disabled: false })).toBe('active');
  });

  it('handles mixed inputs', () => {
    expect(clsx('base', ['mod'], { active: true }, null, undefined, false, 0)).toBe('base mod active');
  });

  it('ignores falsy values', () => {
    expect(clsx(null, undefined, false, '', 0, NaN)).toBe('');
  });

  it('includes numbers', () => {
    expect(clsx('col', 12)).toBe('col 12');
  });
});
