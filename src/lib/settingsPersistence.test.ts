import { describe, expect, it } from 'vitest';
import { normalizeFeatures } from './settingsPersistence';

describe('settings persistence', () => {
  it('normalizes missing feature fields', () => {
    const normalized = normalizeFeatures({ profile: 'personal' });
    expect(normalized.profile).toBe('personal');
    expect(normalized.customCss).toBe('');
    expect(normalized.customJs).toBe('');
    expect(normalized.injectionAllowlist).toBe('');
  });

  it('provides default values for undefined input', () => {
    const normalized = normalizeFeatures(undefined);
    expect(normalized.profile).toBe('default');
    expect(normalized.customCss).toBe('');
    expect(normalized.customJs).toBe('');
    expect(normalized.injectionAllowlist).toBe('');
  });
});
