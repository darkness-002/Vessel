export type ThemeMode = 'monochrome' | 'paper';

export type DashboardViewMode = 'simple' | 'detailed';

export type ContainerStatus = 'running' | 'idle' | 'hibernated' | 'closed' | 'crashed';

export type EffectiveCardState = 'running' | 'idle' | 'hibernated' | 'locked' | 'closed' | 'crashed';

export type CardDensity = 'minimal' | 'detailed';

export type FilterCategory = 'all' | 'running' | 'locked' | 'active' | 'hibernated';

export type VaultMode = 'keyring' | 'passphrase';

export interface PowerStatus {
  isOnBattery: boolean;
  batteryPercentage?: number;
}

export interface ContainerProcessMetrics {
  id: string;
  name: string;
  url: string;
  status: ContainerStatus;
  pid: number | null;
  memoryMb: number;
  cpuUsage: number;
  lastActiveAt: string;
  idleSeconds: number;
  estimated?: boolean;
  history?: { memoryMb: number; cpuUsage: number }[];
}

export interface ContainerConfig {
  id: string;
  name: string;
  url: string;
  icon?: string | null;
  iconPath?: string | null;
  createdAt: string;
  profileLabel?: string;
  neverHibernate?: boolean;
  idleThresholdMinutes?: number;
  ssoAllowlist?: string[];
}

export interface ContainerCustomSettings {
  name?: string;
  profileLabel?: string;
  neverHibernate?: boolean;
  idleThresholdMinutes?: number;
  ssoAllowlist?: string[];
}

export interface AppSettings {
  theme: ThemeMode;
  viewMode: DashboardViewMode;
  density: CardDensity;
  isDemoMode: boolean;
  globalIdleThresholdMinutes: number;
  muteAllNotifications: boolean;
}

export interface MemoryComparison {
  vesselActiveMb: number;
  electronEquivalentMb: number;
  savedMb: number;
  savedPercentage: number;
  activeCount: number;
  hibernatedCount: number;
  lockedCount: number;
  totalCount: number;
}

export interface ElectronStateBaseline {
  meanMb: number;
  stddevMb: number;
}

export interface ElectronBaseline {
  source: 'assumed' | 'measured';
  hardware: string;
  os: string;
  date: string;
  runs: number;
  states: {
    active: ElectronStateBaseline;
    idle: ElectronStateBaseline;
    hibernated: ElectronStateBaseline;
  };
}

