export type AppFeaturesSnapshot = {
  profile: string;
  customCss: string;
  customJs: string;
  injectionAllowlist: string;
};

export function normalizeFeatures(input: Partial<AppFeaturesSnapshot> | undefined): AppFeaturesSnapshot {
  return {
    profile: input?.profile || 'default',
    customCss: input?.customCss || '',
    customJs: input?.customJs || '',
    injectionAllowlist: input?.injectionAllowlist || ''
  };
}

export function serializeAppsBackup<T>(apps: T[]): string {
  return JSON.stringify(apps);
}

export function deserializeAppsBackup<T>(raw: string | null): T[] {
  if (!raw) return [];
  try {
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? (parsed as T[]) : [];
  } catch {
    return [];
  }
}
