export type ContainerStatus = 'running' | 'idle' | 'hibernated' | 'closed';

export type EffectiveCardState = 'running' | 'idle' | 'hibernated' | 'locked' | 'closed';

export type CardDensity = 'minimal' | 'detailed';

export type FilterCategory = 'all' | 'active' | 'hibernated' | 'locked';

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
}

export interface ContainerCustomSettings {
  name?: string;
  profileLabel?: string;
  neverHibernate?: boolean;
  idleThresholdMinutes?: number;
}

export interface AppSettings {
  density: CardDensity;
  isDemoMode: boolean;
  globalIdleThresholdMinutes: number;
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

