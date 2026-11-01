import type { ContainerConfig, ContainerProcessMetrics } from '$lib/types';

export const DEMO_CONTAINERS: ContainerConfig[] = [
  {
    id: 'github',
    name: 'GitHub',
    url: 'https://github.com',
    icon: null,
    createdAt: '2026-10-20T10:00:00Z',
    idleThresholdMinutes: 15,
  },
  {
    id: 'linear',
    name: 'Linear',
    url: 'https://linear.app',
    icon: null,
    createdAt: '2026-10-20T10:15:00Z',
    idleThresholdMinutes: 15,
  },
  {
    id: 'spotify',
    name: 'Spotify',
    url: 'https://open.spotify.com',
    icon: null,
    createdAt: '2026-10-20T10:30:00Z',
    idleThresholdMinutes: 15,
  },
  {
    id: 'slack',
    name: 'Slack',
    url: 'https://slack.com',
    icon: null,
    createdAt: '2026-10-20T10:45:00Z',
    profileLabel: 'Work Workspace',
    idleThresholdMinutes: 15,
  },
  {
    id: 'notion',
    name: 'Notion',
    url: 'https://notion.so',
    icon: null,
    createdAt: '2026-10-20T11:00:00Z',
    idleThresholdMinutes: 20,
  },
];

export const DEMO_METRICS: Record<string, ContainerProcessMetrics> = {
  // 1. Running state
  github: {
    id: 'github',
    name: 'GitHub',
    url: 'https://github.com',
    status: 'running',
    pid: 14208,
    memoryMb: 64.2,
    cpuUsage: 1.4,
    lastActiveAt: new Date(Date.now() - 12 * 1000).toISOString(),
    idleSeconds: 12,
    history: [
      { memoryMb: 58.4, cpuUsage: 3.2 },
      { memoryMb: 60.1, cpuUsage: 2.8 },
      { memoryMb: 62.5, cpuUsage: 1.9 },
      { memoryMb: 61.8, cpuUsage: 1.1 },
      { memoryMb: 63.9, cpuUsage: 2.4 },
      { memoryMb: 64.2, cpuUsage: 1.4 },
    ],
  },
  // 2. Idle state
  linear: {
    id: 'linear',
    name: 'Linear',
    url: 'https://linear.app',
    status: 'idle',
    pid: 19484,
    memoryMb: 48.6,
    cpuUsage: 0.1,
    lastActiveAt: new Date(Date.now() - 480 * 1000).toISOString(),
    idleSeconds: 480, // 8 minutes idle
    history: [
      { memoryMb: 54.0, cpuUsage: 2.1 },
      { memoryMb: 52.2, cpuUsage: 1.0 },
      { memoryMb: 50.1, cpuUsage: 0.4 },
      { memoryMb: 49.3, cpuUsage: 0.2 },
      { memoryMb: 48.8, cpuUsage: 0.1 },
      { memoryMb: 48.6, cpuUsage: 0.1 },
    ],
  },
  // 3. Hibernated state
  spotify: {
    id: 'spotify',
    name: 'Spotify',
    url: 'https://open.spotify.com',
    status: 'hibernated',
    pid: null,
    memoryMb: 0.0,
    cpuUsage: 0.0,
    lastActiveAt: new Date(Date.now() - 1800 * 1000).toISOString(),
    idleSeconds: 1800,
  },
  // 4. Closed state (vault locked)
  slack: {
    id: 'slack',
    name: 'Slack',
    url: 'https://slack.com',
    status: 'closed',
    pid: null,
    memoryMb: 0.0,
    cpuUsage: 0.0,
    lastActiveAt: new Date(Date.now() - 86400 * 1000).toISOString(),
    idleSeconds: 0,
  },
  // 5. Closed state (standard)
  notion: {
    id: 'notion',
    name: 'Notion',
    url: 'https://notion.so',
    status: 'closed',
    pid: null,
    memoryMb: 0.0,
    cpuUsage: 0.0,
    lastActiveAt: new Date(Date.now() - 7200 * 1000).toISOString(),
    idleSeconds: 0,
  },
};

export const DEMO_VAULT_LOCKED: Record<string, boolean> = {
  github: false,
  linear: false,
  spotify: false,
  slack: true, // Vault sealed
  notion: false,
};

export function getDemoState() {
  return {
    containers: JSON.parse(JSON.stringify(DEMO_CONTAINERS)) as ContainerConfig[],
    metricsMap: JSON.parse(JSON.stringify(DEMO_METRICS)) as Record<string, ContainerProcessMetrics>,
    vaultLockedMap: { ...DEMO_VAULT_LOCKED },
  };
}
