export type ContainerStatus = 'running' | 'idle' | 'hibernated' | 'closed';

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
}

export interface ContainerConfig {
  id: string;
  name: string;
  url: string;
  icon?: string | null;
  iconPath?: string | null;
  createdAt: string;
}
