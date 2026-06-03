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
