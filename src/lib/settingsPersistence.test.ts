import { describe, expect, it } from 'vitest';
import { deserializeAppsBackup, normalizeFeatures, serializeAppsBackup } from './settingsPersistence';

describe('settings persistence', () => {
  it('keeps profile, customCss and customJs fields', () => {
    const apps = [
      {
        id: 'demo',
        features: {
          profile: 'work',
          customCss: 'body { color: red; }',
          customJs: 'console.log(1);',
          injectionAllowlist: 'example.com'
        }
      }
    ];

    const raw = serializeAppsBackup(apps);
    const restored = deserializeAppsBackup<typeof apps[number]>(raw);

    expect(restored[0].features.profile).toBe('work');
    expect(restored[0].features.customCss).toContain('color: red');
    expect(restored[0].features.customJs).toContain('console.log');
  });

  it('normalizes missing feature fields', () => {
    const normalized = normalizeFeatures({ profile: 'personal' });
    expect(normalized.profile).toBe('personal');
    expect(normalized.customCss).toBe('');
    expect(normalized.customJs).toBe('');
    expect(normalized.injectionAllowlist).toBe('');
  });
});
