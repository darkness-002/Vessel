export type AppView = 'gallery' | 'webview' | 'settings' | 'add-app';

export type AppConfig = {
  id: string;
  name?: string;
  icon: string;
  url: string;
  features: {
    theme: string;
    adblock: boolean;
    profile: string;
    customCss: string;
    customJs: string;
    injectionAllowlist: string;
    idleSleepSeconds: number;
  };
};

export type VesselNotification = {
  appId: string;
  title: string;
  body: string;
  time: string;
};

export type BrowserTab = {
  id: string;
  appId: string;
  title: string;
  url: string;
};

export type NewTabEvent = {
  appId: string;
  url: string;
  title?: string | null;
};

export type ResourceUsage = {
  cpuPercent: number;
  ramMb: number;
};

export type DiagnosticEvent = {
  level: string;
  category: string;
  appId: string;
  webviewId?: string | null;
  message: string;
  detail?: string | null;
  time: string;
};
