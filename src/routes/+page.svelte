<script lang="ts">
  import { tick } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import type {
    ContainerConfig,
    ContainerProcessMetrics,
    FilterCategory,
    ContainerCustomSettings,
    VaultMode,
  } from '$lib/types';
  import { showToast } from '$lib/stores/toastStore';
  import {
    appSettings,
    updateAppSettings,
    setTheme,
    setViewMode,
    applyTheme,
    syncSettingsFromBackend,
  } from '$lib/stores/appSettings';
  import { getDemoState } from '$lib/fixtures/demoContainers';
  import baselineData from '$lib/data/electron_baseline.json';

  import TopBar from '$lib/components/TopBar.svelte';
  import ContainerCard from '$lib/components/ContainerCard.svelte';
  import VaultUnlockModal from '$lib/components/VaultUnlockModal.svelte';
  import SettingsPanel from '$lib/components/SettingsPanel.svelte';
  import ContainerMenuModal from '$lib/components/ContainerMenuModal.svelte';
  import DocumentationModal from '$lib/components/DocumentationModal.svelte';
  import QuickSwitcher from '$lib/components/QuickSwitcher.svelte';
  import ToastContainer from '$lib/components/ToastContainer.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import AppIcon from '$lib/components/AppIcon.svelte';

  // Local storage key for container custom preferences
  const CONTAINER_SETTINGS_STORAGE_KEY = 'vessel_container_settings_map';

  function loadLocalContainerSettings(): Record<string, ContainerCustomSettings> {
    if (typeof window === 'undefined' || !window.localStorage) return {};
    try {
      const raw = localStorage.getItem(CONTAINER_SETTINGS_STORAGE_KEY);
      return raw ? JSON.parse(raw) : {};
    } catch {
      return {};
    }
  }

  function saveLocalContainerSettings(map: Record<string, ContainerCustomSettings>) {
    if (typeof window === 'undefined' || !window.localStorage) return;
    try {
      localStorage.setItem(CONTAINER_SETTINGS_STORAGE_KEY, JSON.stringify(map));
    } catch {}
  }

  // Navigation tab state: 'apps' (default) or 'manage'
  let activeNavTab = $state<'apps' | 'manage'>('apps');

  // Reactive State
  let containers = $state<ContainerConfig[]>([]);
  let metricsMap = $state<Record<string, ContainerProcessMetrics>>({});
  let vaultLockedMap = $state<Record<string, boolean>>({});
  let containerSettingsMap = $state<Record<string, ContainerCustomSettings>>(
    loadLocalContainerSettings()
  );

  let isLoading = $state(true);
  let searchQuery = $state('');
  let activeFilter = $state<FilterCategory>('all');
  let searchInputRef = $state<HTMLInputElement | null>(null);

  // Manage containers tab search & filter state
  let manageSearchQuery = $state('');
  let manageStatusFilter = $state<'all' | 'running' | 'locked' | 'hibernated'>('all');

  // Modals & Panels
  let isSettingsOpen = $state(false);
  let showDocModal = $state(false);
  let showQuickSwitcher = $state(false);
  let showCreateModal = $state(false);
  let showUnlockModal = $state(false);
  let unlockTarget = $state<ContainerConfig | null>(null);
  let unlockTargetVaultMode = $state<VaultMode>('keyring');
  let isUnlocking = $state(false);
  let unlockError = $state('');

  let showContainerMenuModal = $state(false);
  let selectedContainerForMenu = $state<ContainerConfig | null>(null);

  let confirmDeleteContainer = $state<ContainerConfig | null>(null);
  let isDeleting = $state(false);

  // Action Loading Trackers (for card & table loading states)
  let launchingId = $state<string | null>(null);
  let wakingId = $state<string | null>(null);
  let unlockingCardId = $state<string | null>(null);
  let actionLoadingId = $state<string | null>(null);
  let installingPreset = $state<string | null>(null);

  // New Container Form State
  let newName = $state('');
  let newUrl = $state('');
  let hasUserEditedName = $state(false);
  let isSubmitting = $state(false);
  let modalError = $state('');
  let urlInputRef = $state<HTMLInputElement | null>(null);

  // Common App Presets
  const COMMON_PRESETS = [
    { name: 'GitHub', url: 'https://github.com', category: 'Dev' },
    { name: 'Spotify', url: 'https://open.spotify.com', category: 'Media' },
    { name: 'Slack', url: 'https://slack.com', category: 'Chat' },
    { name: 'Linear', url: 'https://linear.app', category: 'Dev' },
    { name: 'Notion', url: 'https://notion.so', category: 'Productivity' },
  ];

  const knownBrands: Record<string, string> = {
    'chatgpt.com': 'ChatGPT',
    'openai.com': 'ChatGPT',
    'claude.ai': 'Claude',
    'anthropic.com': 'Claude',
    'github.com': 'GitHub',
    'slack.com': 'Slack',
    'discord.com': 'Discord',
    'notion.so': 'Notion',
    'linear.app': 'Linear',
    'spotify.com': 'Spotify',
    'open.spotify.com': 'Spotify',
    'figma.com': 'Figma',
  };

  function extractTitleFromUrl(rawUrl: string): string {
    const trimmed = rawUrl.trim();
    if (!trimmed) return '';
    try {
      const formatted =
        trimmed.startsWith('http://') || trimmed.startsWith('https://')
          ? trimmed
          : 'https://' + trimmed;
      const parsed = new URL(formatted);
      const host = parsed.hostname.toLowerCase().replace(/^www\./, '');

      if (knownBrands[host]) return knownBrands[host];

      const parts = host.split('.');
      let base = parts.length > 1 ? parts[parts.length - 2] : parts[0];
      if (['co', 'com', 'org', 'net', 'io', 'app', 'dev', 'ai'].includes(base) && parts.length > 2) {
        base = parts[parts.length - 3];
      }
      return base.charAt(0).toUpperCase() + base.slice(1);
    } catch {
      return '';
    }
  }

  function getTargetDomain(rawUrl: string): string {
    try {
      const formatted =
        rawUrl.startsWith('http://') || rawUrl.startsWith('https://')
          ? rawUrl
          : 'https://' + rawUrl;
      const parsed = new URL(formatted);
      return parsed.hostname.replace(/^www\./, '');
    } catch {
      return rawUrl.replace(/^https?:\/\//, '').split('/')[0];
    }
  }


  // Combined containers with local customizations
  let enrichedContainers = $derived(
    containers.map((c) => {
      const custom = containerSettingsMap[c.id];
      if (!custom) return c;
      return {
        ...c,
        name: custom.name || c.name,
        profileLabel: custom.profileLabel !== undefined ? custom.profileLabel : c.profileLabel,
        neverHibernate: custom.neverHibernate !== undefined ? custom.neverHibernate : c.neverHibernate,
        idleThresholdMinutes:
          custom.idleThresholdMinutes !== undefined
            ? custom.idleThresholdMinutes
            : c.idleThresholdMinutes,
      };
    })
  );

  // Live filter counts
  let runningCount = $derived(
    enrichedContainers.filter((c) => {
      if (vaultLockedMap[c.id]) return false;
      const s = metricsMap[c.id]?.status;
      return s === 'running';
    }).length
  );

  let lockedCount = $derived(
    enrichedContainers.filter((c) => vaultLockedMap[c.id]).length
  );

  let idleCount = $derived(
    enrichedContainers.filter((c) => {
      if (vaultLockedMap[c.id]) return false;
      const s = metricsMap[c.id]?.status;
      return s === 'idle' || s === 'closed' || !s;
    }).length
  );

  let hibernatedCount = $derived(
    enrichedContainers.filter((c) => !vaultLockedMap[c.id] && metricsMap[c.id]?.status === 'hibernated').length
  );

  // Filtered Containers according to Status filter tab and Search query in "Your apps"
  let filteredContainers = $derived(
    enrichedContainers.filter((c) => {
      // 1. Status Filter
      if (activeFilter === 'running' || activeFilter === 'active') {
        if (vaultLockedMap[c.id]) return false;
        const status = metricsMap[c.id]?.status;
        if (status !== 'running') return false;
      } else if (activeFilter === 'hibernated') {
        if (vaultLockedMap[c.id]) return false;
        if (metricsMap[c.id]?.status !== 'hibernated') return false;
      } else if (activeFilter === 'locked') {
        if (!vaultLockedMap[c.id]) return false;
      }

      // 2. Search Text Filter
      if (searchQuery.trim()) {
        const q = searchQuery.toLowerCase();
        const matchesName = c.name.toLowerCase().includes(q);
        const matchesUrl = c.url.toLowerCase().includes(q);
        const matchesLabel = c.profileLabel?.toLowerCase().includes(q) ?? false;
        if (!matchesName && !matchesUrl && !matchesLabel) return false;
      }

      return true;
    })
  );

  // Filtered Containers for "Manage containers" tab
  let manageFilteredContainers = $derived(
    enrichedContainers.filter((c) => {
      if (manageStatusFilter === 'running') {
        if (vaultLockedMap[c.id]) return false;
        const status = metricsMap[c.id]?.status;
        if (status !== 'running') return false;
      } else if (manageStatusFilter === 'locked') {
        if (!vaultLockedMap[c.id]) return false;
      } else if (manageStatusFilter === 'hibernated') {
        if (vaultLockedMap[c.id]) return false;
        if (metricsMap[c.id]?.status !== 'hibernated') return false;
      }

      if (manageSearchQuery.trim()) {
        const q = manageSearchQuery.toLowerCase();
        const matchesName = c.name.toLowerCase().includes(q);
        const matchesUrl = c.url.toLowerCase().includes(q);
        const matchesLabel = c.profileLabel?.toLowerCase().includes(q) ?? false;
        if (!matchesName && !matchesUrl && !matchesLabel) return false;
      }

      return true;
    })
  );

  // Live Telemetry Aggregates for Detailed View KPIs
  let totalActiveRam = $derived(
    Object.entries(metricsMap)
      .filter(([id, m]) => !vaultLockedMap[id] && m.status === 'running')
      .reduce((sum, [, m]) => sum + (m.memoryMb || 0), 0)
  );

  let totalActiveCpu = $derived(
    Object.entries(metricsMap)
      .filter(([id, m]) => !vaultLockedMap[id] && m.status === 'running')
      .reduce((sum, [, m]) => sum + (m.cpuUsage || 0), 0)
  );

  // Estimated memory saved relative to Electron baseline
  let totalSavedRam = $derived(
    Math.max(
      0,
      (hibernatedCount + lockedCount) * baselineData.states.active.meanMb +
        Math.max(0, runningCount * baselineData.states.active.meanMb - totalActiveRam)
    )
  );

  // Live Footer Status Telemetry Text
  let footerStatusText = $derived.by(() => {
    const total = enrichedContainers.length;
    return `${total} ${total === 1 ? 'app' : 'apps'} · ${runningCount} running · ${idleCount} idle · ${lockedCount} locked`;
  });

  // Data Loading & Polling
  async function loadContainers() {
    if ($appSettings.isDemoMode) {
      const demo = getDemoState();
      containers = demo.containers;
      metricsMap = demo.metricsMap;
      vaultLockedMap = demo.vaultLockedMap;
      isLoading = false;
      return;
    }

    try {
      containers = await invoke<ContainerConfig[]>('list_containers');
    } catch (err) {
      console.warn('Failed to load containers via Tauri invoke, running in web/offline mode:', err);
      if (containers.length === 0) {
        containers = [];
      }
    } finally {
      isLoading = false;
    }
  }

  let isPolling = false;
  let isResizing = false;
  let resizeTimeout: ReturnType<typeof setTimeout> | undefined;

  function handleWindowResize() {
    isResizing = true;
    if (resizeTimeout) clearTimeout(resizeTimeout);
    resizeTimeout = setTimeout(() => {
      isResizing = false;
    }, 250);
  }

  async function pollTelemetry() {
    if ($appSettings.isDemoMode || isPolling || isResizing) {
      return;
    }
    if (typeof document !== 'undefined' && document.hidden) {
      return;
    }

    isPolling = true;
    try {
      const metricsList = await invoke<ContainerProcessMetrics[]>('get_container_metrics');
      const newMap: Record<string, ContainerProcessMetrics> = {};
      for (const m of metricsList) {
        newMap[m.id] = m;
      }
      metricsMap = newMap;

      // Query locked status for each container
      const lockStatuses: Record<string, boolean> = {};
      await Promise.all(
        containers.map(async (c) => {
          try {
            const locked = await invoke<boolean>('is_vault_locked', { id: c.id });
            lockStatuses[c.id] = locked;
          } catch {
            lockStatuses[c.id] = false;
          }
        })
      );
      vaultLockedMap = lockStatuses;
    } catch (err) {
      console.error('Telemetry polling error:', err);
    } finally {
      isPolling = false;
    }
  }

  // 3-Second Polling Effect
  $effect(() => {
    applyTheme($appSettings.theme || 'monochrome');
    syncSettingsFromBackend();
    loadContainers().then(() => {
      pollTelemetry();
    });

    const interval = setInterval(() => {
      pollTelemetry();
    }, 3000);

    let unlistenLockAll: (() => void) | undefined;
    listen('vessel:lock-all', () => {
      showToast('Global lock engaged: All vaults sealed and memory wiped', 'info');
      pollTelemetry();
    })
      .then((unlisten: () => void) => {
        unlistenLockAll = unlisten;
      })
      .catch(() => {});

    return () => {
      clearInterval(interval);
      if (resizeTimeout) clearTimeout(resizeTimeout);
      if (unlistenLockAll) {
        unlistenLockAll();
      }
    };
  });

  // Watch Demo Mode changes
  $effect(() => {
    const isDemo = $appSettings.isDemoMode;
    if (isDemo) {
      const demo = getDemoState();
      containers = demo.containers;
      metricsMap = demo.metricsMap;
      vaultLockedMap = demo.vaultLockedMap;
      isLoading = false;
    } else {
      loadContainers().then(() => pollTelemetry());
    }
  });

  // Container Actions
  async function handlePrimaryAction(container: ContainerConfig) {
    const isLocked = vaultLockedMap[container.id];
    const status = metricsMap[container.id]?.status ?? 'closed';

    if (isLocked) {
      unlockTarget = container;
      unlockError = '';
      try {
        if (!$appSettings.isDemoMode) {
          unlockTargetVaultMode = await invoke<VaultMode>('get_vault_mode', { id: container.id });
        } else {
          unlockTargetVaultMode = 'keyring';
        }
      } catch {
        unlockTargetVaultMode = 'keyring';
      }
      showUnlockModal = true;
      return;
    }

    if (status === 'crashed') {
      handleRecover(container);
      return;
    }

    if (status === 'hibernated') {
      handleWake(container);
      return;
    }

    // Launch or focus
    launchingId = container.id;
    try {
      if ($appSettings.isDemoMode) {
        metricsMap[container.id] = {
          ...(metricsMap[container.id] || {
            id: container.id,
            name: container.name,
            url: container.url,
            pid: 12345,
            memoryMb: 52.0,
            cpuUsage: 1.0,
            lastActiveAt: new Date().toISOString(),
            idleSeconds: 0,
          }),
          status: 'running',
          idleSeconds: 0,
        };
        showToast(`Focused "${container.name}"`, 'success');
      } else {
        await invoke('launch_container', { id: container.id });
        setTimeout(pollTelemetry, 500);
      }
    } catch (err) {
      const msg = String(err);
      if (msg.includes('vault') || msg.includes('passphrase') || msg.includes('locked')) {
        unlockTarget = container;
        unlockError = msg;
        try {
          if (!$appSettings.isDemoMode) {
            unlockTargetVaultMode = await invoke<VaultMode>('get_vault_mode', { id: container.id });
          } else {
            unlockTargetVaultMode = 'keyring';
          }
        } catch {
          unlockTargetVaultMode = 'keyring';
        }
        showUnlockModal = true;
      } else {
        showToast(`Launch failed: ${err}`, 'error');
      }
    } finally {
      launchingId = null;
    }
  }

  async function handleRecover(container: ContainerConfig) {
    actionLoadingId = container.id;
    try {
      if ($appSettings.isDemoMode) {
        metricsMap[container.id] = {
          ...(metricsMap[container.id] || {
            id: container.id,
            name: container.name,
            url: container.url,
            pid: 14200,
            lastActiveAt: new Date().toISOString(),
          }),
          status: 'running',
          memoryMb: 52.0,
          cpuUsage: 1.0,
          idleSeconds: 0,
        };
        showToast(`Recovered "${container.name}"`, 'success');
      } else {
        await invoke('recover_container', { id: container.id });
        showToast(`Recovered "${container.name}"`, 'success');
        setTimeout(pollTelemetry, 500);
      }
    } catch (err) {
      showToast(`Recovery failed: ${err}`, 'error');
    } finally {
      actionLoadingId = null;
    }
  }

  async function handleLockAll() {
    try {
      if ($appSettings.isDemoMode) {
        for (const c of containers) {
          vaultLockedMap[c.id] = true;
          if (metricsMap[c.id]) {
            metricsMap[c.id].status = 'closed';
            metricsMap[c.id].memoryMb = 0.0;
          }
        }
        showToast('All vaults sealed and memory wiped', 'info');
      } else {
        await invoke('lock_all_containers');
        showToast('All vaults sealed and memory wiped', 'info');
        pollTelemetry();
      }
    } catch (err) {
      showToast(`Lock all failed: ${err}`, 'error');
    }
  }

  async function handleHibernateAll() {
    try {
      if ($appSettings.isDemoMode) {
        for (const c of containers) {
          if (!vaultLockedMap[c.id] && metricsMap[c.id]) {
            metricsMap[c.id].status = 'hibernated';
            metricsMap[c.id].memoryMb = 0.0;
            metricsMap[c.id].cpuUsage = 0.0;
          }
        }
        showToast('All active containers hibernated', 'info');
      } else {
        await invoke('hibernate_all_containers');
        showToast('All active containers hibernated', 'info');
        setTimeout(pollTelemetry, 500);
      }
    } catch (err) {
      showToast(`Hibernate all failed: ${err}`, 'error');
    }
  }

  async function handleWake(container: ContainerConfig) {
    wakingId = container.id;
    try {
      if ($appSettings.isDemoMode) {
        metricsMap[container.id] = {
          id: container.id,
          name: container.name,
          url: container.url,
          status: 'running',
          pid: 18400,
          memoryMb: 56.4,
          cpuUsage: 1.2,
          lastActiveAt: new Date().toISOString(),
          idleSeconds: 0,
        };
        showToast(`Woke "${container.name}"`, 'success');
      } else {
        await invoke('wake_container', { id: container.id });
        showToast(`Waking "${container.name}"...`, 'info');
        setTimeout(pollTelemetry, 500);
      }
    } catch (err) {
      showToast(`Wake failed: ${err}`, 'error');
    } finally {
      wakingId = null;
    }
  }

  async function handleForceHibernate(container: ContainerConfig) {
    actionLoadingId = container.id;
    try {
      if ($appSettings.isDemoMode) {
        metricsMap[container.id] = {
          ...(metricsMap[container.id] || {
            id: container.id,
            name: container.name,
            url: container.url,
            pid: null,
            lastActiveAt: new Date().toISOString(),
          }),
          status: 'hibernated',
          memoryMb: 0.0,
          cpuUsage: 0.0,
          idleSeconds: 1800,
        };
      } else {
        await invoke('force_hibernate_container', { id: container.id });
        setTimeout(pollTelemetry, 400);
      }

      showToast(`Hibernated "${container.name}"`, 'info', 6000, {
        label: 'Undo',
        onClick: () => handleWake(container),
      });
    } catch (err) {
      showToast(`Hibernate failed: ${err}`, 'error');
    } finally {
      actionLoadingId = null;
    }
  }

  async function handleLockVault(container: ContainerConfig) {
    actionLoadingId = container.id;
    try {
      if ($appSettings.isDemoMode) {
        vaultLockedMap[container.id] = true;
        if (metricsMap[container.id]) {
          metricsMap[container.id].status = 'closed';
          metricsMap[container.id].memoryMb = 0.0;
        }
      } else {
        await invoke('lock_container_vault', { id: container.id });
        vaultLockedMap[container.id] = true;
        setTimeout(pollTelemetry, 400);
      }

      showToast(`Sealed vault for "${container.name}"`, 'info', 6000, {
        label: 'Undo',
        onClick: () => handleUndoSeal(container),
      });
    } catch (err) {
      showToast(`Vault encryption failed: ${err}`, 'error');
    } finally {
      actionLoadingId = null;
    }
  }

  async function handleUndoSeal(container: ContainerConfig) {
    unlockingCardId = container.id;
    try {
      if ($appSettings.isDemoMode) {
        vaultLockedMap[container.id] = false;
        metricsMap[container.id] = {
          id: container.id,
          name: container.name,
          url: container.url,
          status: 'running',
          pid: 14208,
          memoryMb: 58.2,
          cpuUsage: 1.1,
          lastActiveAt: new Date().toISOString(),
          idleSeconds: 0,
        };
        showToast(`Restored "${container.name}"`, 'success');
      } else {
        await invoke('unlock_container_vault', { id: container.id });
        await invoke('launch_container', { id: container.id });
        vaultLockedMap[container.id] = false;
        showToast(`Restored and launched "${container.name}"`, 'success');
        setTimeout(pollTelemetry, 500);
      }
    } catch (err) {
      showToast(`Undo seal failed: ${err}`, 'error');
    } finally {
      unlockingCardId = null;
    }
  }

  async function handleUnlockVault(passphrase?: string) {
    if (!unlockTarget) return;
    const target = unlockTarget;
    isUnlocking = true;
    unlockError = '';
    try {
      if ($appSettings.isDemoMode) {
        vaultLockedMap[target.id] = false;
        metricsMap[target.id] = {
          id: target.id,
          name: target.name,
          url: target.url,
          status: 'running',
          pid: 15100,
          memoryMb: 52.8,
          cpuUsage: 1.0,
          lastActiveAt: new Date().toISOString(),
          idleSeconds: 0,
        };
        showToast(`Unlocked and launched "${target.name}"`, 'success');
        showUnlockModal = false;
        unlockTarget = null;
      } else {
        await invoke('unlock_container_vault', { id: target.id, passphrase });
        await invoke('launch_container', { id: target.id });
        vaultLockedMap[target.id] = false;
        showToast(`Unlocked and launched "${target.name}"`, 'success');
        showUnlockModal = false;
        unlockTarget = null;
        setTimeout(pollTelemetry, 500);
      }
    } catch (err) {
      unlockError = String(err);
      showToast(`Unlock failed: ${err}`, 'error');
    } finally {
      isUnlocking = false;
    }
  }

  function handleOpenMenu(container: ContainerConfig) {
    selectedContainerForMenu = container;
    showContainerMenuModal = true;
  }

  async function handleSaveContainerSettings(
    containerId: string,
    settings: ContainerCustomSettings
  ) {
    containerSettingsMap = {
      ...containerSettingsMap,
      [containerId]: settings,
    };
    saveLocalContainerSettings(containerSettingsMap);

    try {
      if (!$appSettings.isDemoMode) {
        const updated = await invoke<ContainerConfig>('update_container_policy', {
          id: containerId,
          name: settings.name ?? null,
          neverHibernate: settings.neverHibernate ?? null,
          idleThresholdMinutes: settings.idleThresholdMinutes ?? null,
          profileLabel: settings.profileLabel ?? null,
        });
        containers = containers.map((c) => (c.id === containerId ? updated : c));
      } else {
        containers = containers.map((c) =>
          c.id === containerId
            ? {
                ...c,
                name: settings.name || c.name,
                neverHibernate: settings.neverHibernate,
                idleThresholdMinutes: settings.idleThresholdMinutes,
                profileLabel: settings.profileLabel,
              }
            : c
        );
      }
      showToast('Preferences updated', 'success');
    } catch (err) {
      showToast(`Failed to update policy: ${err}`, 'error');
    }
  }

  async function handlePinDesktop(container: ContainerConfig) {
    try {
      if ($appSettings.isDemoMode) {
        showToast(`Created desktop shortcut for "${container.name}"`, 'success');
      } else {
        const res = await invoke<string>('create_shortcut', { id: container.id });
        showToast(res || `Desktop shortcut pinned for "${container.name}"`, 'success');
      }
    } catch (err) {
      showToast(`Shortcut failed: ${err}`, 'error');
    }
  }

  function promptDelete(container: ContainerConfig) {
    confirmDeleteContainer = container;
  }

  async function confirmDelete() {
    if (!confirmDeleteContainer) return;
    const target = confirmDeleteContainer;
    isDeleting = true;
    try {
      if ($appSettings.isDemoMode) {
        containers = containers.filter((c) => c.id !== target.id);
        const nextMetrics = { ...metricsMap };
        delete nextMetrics[target.id];
        metricsMap = nextMetrics;
        const nextLocked = { ...vaultLockedMap };
        delete nextLocked[target.id];
        vaultLockedMap = nextLocked;
        showToast(`Removed "${target.name}"`, 'info');
      } else {
        await invoke('delete_container', { id: target.id });
        containers = containers.filter((c) => c.id !== target.id);
        showToast(`Removed "${target.name}" and purged vault data`, 'info');
        pollTelemetry();
      }
      confirmDeleteContainer = null;
    } catch (err) {
      showToast(`Delete failed: ${err}`, 'error');
    } finally {
      isDeleting = false;
    }
  }

  // Create Container Functions
  async function openCreateModal(preset?: { name: string; url: string }) {
    modalError = '';
    if (preset) {
      newName = preset.name;
      newUrl = preset.url;
      hasUserEditedName = true;
    } else {
      newName = '';
      newUrl = '';
      hasUserEditedName = false;
    }
    showCreateModal = true;
    await tick();
    urlInputRef?.focus();
  }

  function closeCreateModal() {
    if (isSubmitting) return;
    showCreateModal = false;
    modalError = '';
  }

  async function handleOpenDocs() {
    try {
      await openUrl('https://darkness-002.github.io/Vessel/docs.html');
    } catch {
      if (typeof window !== 'undefined') {
        window.open('https://darkness-002.github.io/Vessel/docs.html', '_blank');
      }
    }
  }

  async function handleCreateContainer(e: SubmitEvent) {
    e.preventDefault();
    modalError = '';
    const cleanName = newName.trim();
    let cleanUrl = newUrl.trim();

    if (!cleanUrl) {
      modalError = 'URL is required.';
      urlInputRef?.focus();
      return;
    }

    if (!cleanUrl.startsWith('http://') && !cleanUrl.startsWith('https://')) {
      cleanUrl = 'https://' + cleanUrl;
    }

    const finalName =
      cleanName || extractTitleFromUrl(cleanUrl) || getTargetDomain(cleanUrl);

    isSubmitting = true;
    try {
      if ($appSettings.isDemoMode) {
        const id = finalName.toLowerCase().replace(/[^a-z0-9]/g, '-');
        const created: ContainerConfig = {
          id,
          name: finalName,
          url: cleanUrl,
          createdAt: new Date().toISOString(),
        };
        containers = [created, ...containers.filter((c) => c.id !== id)];
        metricsMap[id] = {
          id,
          name: finalName,
          url: cleanUrl,
          status: 'closed',
          pid: null,
          memoryMb: 0.0,
          cpuUsage: 0.0,
          lastActiveAt: new Date().toISOString(),
          idleSeconds: 0,
        };
        showToast(`Installed "${created.name}"`, 'success');
        showCreateModal = false;
        newName = '';
        newUrl = '';
      } else {
        const created = await invoke<ContainerConfig>('create_container', {
          name: finalName,
          url: cleanUrl,
        });
        containers = [created, ...containers.filter((c) => c.id !== created.id)];
        showToast(`Installed "${created.name}"`, 'success');
        showCreateModal = false;
        isSubmitting = false;
        newName = '';
        newUrl = '';
        pollTelemetry();
      }
    } catch (err) {
      console.error('Container install error:', err);
      modalError = String(err);
      showToast(`Install failed: ${err}`, 'error');
    } finally {
      isSubmitting = false;
    }
  }

  async function installTemplateDirectly(preset: { name: string; url: string }) {
    if (installingPreset) return;
    installingPreset = preset.name;
    try {
      if ($appSettings.isDemoMode) {
        const id = preset.name.toLowerCase();
        const created: ContainerConfig = {
          id,
          name: preset.name,
          url: preset.url,
          createdAt: new Date().toISOString(),
        };
        containers = [created, ...containers.filter((c) => c.id !== id)];
        showToast(`Installed "${created.name}"`, 'success');
        return;
      }

      const created = await invoke<ContainerConfig>('create_container', {
        name: preset.name,
        url: preset.url,
      });
      containers = [created, ...containers.filter((c) => c.id !== created.id)];
      showToast(`Installed "${created.name}"`, 'success');
      installingPreset = null;
      pollTelemetry();
    } catch (err) {
      console.error('Template direct install error:', err);
      showToast(`Install failed: ${err}`, 'error');
    } finally {
      installingPreset = null;
    }
  }

  // Global Keyboard Shortcuts
  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      if (showDocModal) {
        showDocModal = false;
      } else if (showQuickSwitcher) {
        showQuickSwitcher = false;
      } else if (confirmDeleteContainer && !isDeleting) {
        confirmDeleteContainer = null;
      } else if (showUnlockModal && !isUnlocking) {
        showUnlockModal = false;
      } else if (showContainerMenuModal) {
        showContainerMenuModal = false;
      } else if (isSettingsOpen) {
        isSettingsOpen = false;
      } else if (showCreateModal && !isSubmitting) {
        closeCreateModal();
      }
    } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
      e.preventDefault();
      showQuickSwitcher = !showQuickSwitcher;
    } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'n') {
      e.preventDefault();
      openCreateModal();
    } else if (
      e.key === '/' &&
      !showQuickSwitcher &&
      !showCreateModal &&
      !confirmDeleteContainer &&
      !showUnlockModal &&
      !showContainerMenuModal &&
      !isSettingsOpen &&
      !showDocModal
    ) {
      const activeTag = document.activeElement?.tagName.toLowerCase();
      if (activeTag !== 'input' && activeTag !== 'textarea') {
        e.preventDefault();
        searchInputRef?.focus();
      }
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} onresize={handleWindowResize} />

<div class="min-h-screen w-screen overflow-x-hidden flex flex-col bg-[var(--bg-canvas)] text-[var(--text-primary)] font-sans select-none">
  <!-- Top Navigation Bar: Full-Width Fluid Shell -->
  <TopBar
    activeTab={activeNavTab}
    onSelectTab={(tab) => (activeNavTab = tab)}
    onOpenQuickSwitcher={() => (showQuickSwitcher = true)}
    onOpenDocumentation={handleOpenDocs}
    onOpenSettings={() => (isSettingsOpen = true)}
  />

  <!-- Main Workspace Shell: Full-Width Fluid Margins -->
  <main class="flex-1 flex flex-col w-full">
    <div class="w-full px-6 md:px-10 xl:px-14 py-8 flex-1 flex flex-col justify-between space-y-6">

      <div class="space-y-6">
        {#if activeNavTab === 'apps'}
          <!-- =================== YOUR APPS TAB =================== -->

          <!-- Header Row -->
          <div class="flex items-start justify-between gap-4">
            <div>
              <h1 class="text-2xl lg:text-3xl font-bold tracking-tight text-[var(--text-primary)]">
                Your apps
              </h1>
              <p class="text-sm text-[var(--text-muted)] mt-1 font-normal">
                A calmer home for your apps.
              </p>
            </div>

            <div class="flex flex-col items-end">
              <button
                type="button"
                onclick={() => openCreateModal()}
                class="min-h-[38px] px-5 py-2.5 rounded-xl bg-[var(--accent-primary)] hover:opacity-95 text-[var(--accent-fg)] font-semibold text-xs flex items-center gap-1.5 transition-colors active:scale-[0.98] shadow-sm cursor-pointer"
              >
                <Icon name="add" size={14} />
                <span>Add app</span>
              </button>

              {#if $appSettings.viewMode === 'detailed'}
                <span class="text-[11px] text-[var(--text-muted)] mt-1.5 font-normal">
                  Detailed view · <button type="button" onclick={() => (isSettingsOpen = true)} class="underline hover:text-[var(--text-primary)] transition-colors focus-visible:outline-none cursor-pointer">Change in Settings</button>
                </span>
              {/if}
            </div>
          </div>

          <!-- Filter Toolbar Row (Search Input + Inline Filter Tabs on Left) -->
          <div class="flex flex-col sm:flex-row sm:items-center gap-6 pt-1 border-b border-[var(--border-subtle)] pb-2.5">
            <!-- Search Input -->
            <div class="relative w-full sm:w-80 shrink-0">
              <div class="absolute inset-y-0 left-0 pl-3.5 flex items-center pointer-events-none text-[var(--text-muted)]">
                <Icon name="search" size={15} />
              </div>
              <input
                bind:this={searchInputRef}
                type="text"
                bind:value={searchQuery}
                placeholder="Search apps..."
                class="w-full pl-10 pr-8 py-2 text-xs bg-[var(--bg-surface)] border border-[var(--border-subtle)] rounded-xl text-[var(--text-primary)] placeholder:text-[var(--text-muted)] focus:outline-none focus:ring-1 focus:ring-[var(--accent-primary)] focus:border-[var(--accent-primary)] transition-colors min-h-[38px]"
              />
              {#if searchQuery}
                <button
                  type="button"
                  onclick={() => (searchQuery = '')}
                  class="absolute inset-y-0 right-0 pr-3 flex items-center text-[var(--text-muted)] hover:text-[var(--text-primary)] cursor-pointer"
                  aria-label="Clear search"
                >
                  <Icon name="close" size={13} />
                </button>
              {:else}
                <div class="absolute inset-y-0 right-0 pr-2.5 flex items-center pointer-events-none">
                  <kbd class="px-1.5 py-0.5 rounded bg-[var(--bg-canvas)] border border-[var(--border-subtle)] font-mono text-[10px] text-[var(--text-muted)]">
                    /
                  </kbd>
                </div>
              {/if}
            </div>

            <!-- Inline Status Filter Tabs (clean text with accent underline on active) -->
            <div class="flex items-center gap-6 h-full" role="tablist" aria-label="Status filters">
              <button
                type="button"
                role="tab"
                aria-selected={activeFilter === 'all'}
                onclick={() => (activeFilter = 'all')}
                class="text-xs transition-colors relative pb-2.5 -mb-2.5 border-b-2 focus-visible:outline-none cursor-pointer {activeFilter === 'all'
                  ? 'border-[var(--accent-primary)] text-[var(--text-primary)] font-semibold'
                  : 'border-transparent text-[var(--text-muted)] hover:text-[var(--text-primary)] font-medium'}"
              >
                All apps
              </button>
              <button
                type="button"
                role="tab"
                aria-selected={activeFilter === 'running'}
                onclick={() => (activeFilter = 'running')}
                class="text-xs transition-colors relative pb-2.5 -mb-2.5 border-b-2 focus-visible:outline-none cursor-pointer {activeFilter === 'running' || activeFilter === 'active'
                  ? 'border-[var(--accent-primary)] text-[var(--text-primary)] font-semibold'
                  : 'border-transparent text-[var(--text-muted)] hover:text-[var(--text-primary)] font-medium'}"
              >
                Running
              </button>
              <button
                type="button"
                role="tab"
                aria-selected={activeFilter === 'locked'}
                onclick={() => (activeFilter = 'locked')}
                class="text-xs transition-colors relative pb-2.5 -mb-2.5 border-b-2 focus-visible:outline-none cursor-pointer {activeFilter === 'locked'
                  ? 'border-[var(--accent-primary)] text-[var(--text-primary)] font-semibold'
                  : 'border-transparent text-[var(--text-muted)] hover:text-[var(--text-primary)] font-medium'}"
              >
                Locked
              </button>
            </div>
          </div>

          <!-- Content Area -->
          {#if isLoading}
            <div class="flex flex-col items-center justify-center py-24 gap-3 text-[var(--text-muted)]">
              <Icon name="spinner" size={24} class="text-[var(--accent-primary)]" />
              <span class="text-xs font-mono">Connecting to supervisor...</span>
            </div>

          {:else if enrichedContainers.length === 0}
            <!-- Empty State (No Containers Installed) -->
            <div
              class="flex flex-col items-center justify-center py-16 px-6 text-center rounded-2xl border border-dashed border-[var(--border-subtle)] bg-[var(--bg-surface)]/40"
            >
              <div
                class="w-12 h-12 rounded-xl bg-[var(--bg-canvas)] flex items-center justify-center mb-3 border border-[var(--border-subtle)] text-[var(--accent-primary)]"
              >
                <Icon name="grid" size={24} />
              </div>
              <h2 class="text-sm font-semibold text-[var(--text-primary)]">No web apps installed</h2>
              <p class="text-xs text-[var(--text-muted)] max-w-sm mt-1 mb-6 leading-relaxed">
                Launch web apps in dedicated native windows with isolated cookies, encrypted profile vaults, and idle hibernation.
              </p>

              <button
                type="button"
                onclick={() => openCreateModal()}
                class="inline-flex items-center gap-2 min-h-[40px] px-5 rounded-xl bg-[var(--accent-primary)] hover:opacity-95 text-[var(--accent-fg)] font-semibold text-xs active:scale-[0.98] transition-colors shadow-sm cursor-pointer"
              >
                <Icon name="add" size={14} />
                <span>Add app</span>
              </button>

              <!-- One-Click Templates for Common Apps -->
              <div class="mt-10 w-full max-w-2xl">
                <span class="text-[11px] font-semibold text-[var(--text-muted)] uppercase tracking-wider block mb-3 font-mono">
                  One-Click Common App Templates
                </span>
                <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-5 gap-2.5">
                  {#each COMMON_PRESETS as preset}
                    <button
                      type="button"
                      onclick={() => installTemplateDirectly(preset)}
                      disabled={installingPreset === preset.name}
                      class="flex flex-col items-start p-3 min-h-[44px] rounded-xl border border-[var(--border-subtle)] bg-[var(--bg-surface)] hover:border-[var(--border-subtle)]/80 hover:bg-[var(--bg-surface-hover)] active:scale-[0.98] transition-colors text-left group cursor-pointer disabled:opacity-60"
                    >
                      <div class="mb-2">
                        <AppIcon name={preset.name} url={preset.url} size={28} rounded="rounded-lg" />
                      </div>
                      <span class="text-xs font-semibold text-[var(--text-primary)] group-hover:text-[var(--accent-primary)]">
                        {preset.name}
                      </span>
                      <span class="text-[10px] text-[var(--text-muted)] font-mono truncate w-full mt-0.5">
                        {getTargetDomain(preset.url)}
                      </span>
                      <span class="mt-2 text-[10px] text-[var(--accent-primary)] font-medium flex items-center gap-1">
                        {#if installingPreset === preset.name}
                          <Icon name="spinner" size={12} />
                          <span>Installing...</span>
                        {:else}
                          <Icon name="add" size={12} />
                          <span>Install</span>
                        {/if}
                      </span>
                    </button>
                  {/each}
                </div>
              </div>
            </div>

          {:else if filteredContainers.length === 0}
            <!-- Search Filter Empty State -->
            <div class="flex flex-col items-center justify-center py-20 text-center text-[var(--text-muted)]">
              <Icon name="search" size={28} class="mb-2 text-[var(--text-muted)] opacity-60" />
              <p class="text-xs font-medium text-[var(--text-primary)]">
                No containers match the current filter "{activeFilter}"
                {#if searchQuery}
                  or query "{searchQuery}"
                {/if}
              </p>
              <button
                type="button"
                onclick={() => {
                  searchQuery = '';
                  activeFilter = 'all';
                }}
                class="mt-3 min-h-[38px] px-4 rounded-xl text-xs font-medium text-[var(--text-primary)] bg-[var(--bg-surface)] border border-[var(--border-subtle)] hover:bg-[var(--bg-surface-hover)] transition-colors cursor-pointer"
              >
                Reset Filters
              </button>
            </div>

          {:else if $appSettings.viewMode === 'simple'}
            <!-- SIMPLE VIEW: Fluid Responsive Grid (1 col mobile, 2 col sm, 3 col lg, 4 col xl, 5 col 2xl) -->
            <div
              class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 2xl:grid-cols-5 gap-6"
              role="region"
              aria-label="Containers grid"
            >
              {#each filteredContainers as container (container.id)}
                <ContainerCard
                  {container}
                  metric={metricsMap[container.id]}
                  isLocked={vaultLockedMap[container.id]}
                  density={$appSettings.density}
                  isLaunching={launchingId === container.id}
                  isWaking={wakingId === container.id}
                  isUnlocking={unlockingCardId === container.id}
                  isActionLoading={actionLoadingId === container.id}
                  onPrimaryAction={handlePrimaryAction}
                  onDetails={handleOpenMenu}
                  onOpenMenu={handleOpenMenu}
                  onHibernate={handleForceHibernate}
                  onLockVault={handleLockVault}
                />
              {/each}
            </div>

            <!-- Total app count footer for Simple view -->
            <div class="text-xs text-[var(--text-muted)] pt-2 font-normal">
              {filteredContainers.length} app{filteredContainers.length === 1 ? '' : 's'}
            </div>

          {:else}
            <!-- DETAILED VIEW (`coral-detailed.png`) -->
            <!-- 2-column Live Telemetry Summary Row -->
            <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
              <!-- RAM KPI Card -->
              <div class="p-6 rounded-2xl border border-[var(--border-subtle)] bg-[var(--bg-surface)] shadow-sm">
                <div class="text-xs font-mono uppercase tracking-wider text-[var(--text-muted)] font-medium">
                  RAM · Across running apps
                </div>
                <div class="text-3xl font-bold tracking-tight text-[var(--text-primary)] mt-2">
                  {Math.round(totalActiveRam)} MB
                </div>
                <div class="text-xs text-[var(--text-muted)] mt-1.5">
                  Saved ~{Math.round(totalSavedRam)} MB vs standard Electron instances
                </div>
              </div>

              <!-- CPU KPI Card -->
              <div class="p-6 rounded-2xl border border-[var(--border-subtle)] bg-[var(--bg-surface)] shadow-sm">
                <div class="text-xs font-mono uppercase tracking-wider text-[var(--text-muted)] font-medium">
                  CPU · Across running apps
                </div>
                <div class="text-3xl font-bold tracking-tight text-[var(--text-primary)] mt-2">
                  {totalActiveCpu.toFixed(1)}%
                </div>
                <div class="text-xs text-[var(--text-muted)] mt-1.5">
                  Background throttling active
                </div>
              </div>
            </div>

            <!-- 5-column Data Table (APP, STATUS, RAM, CPU, ACTION) with horizontal scroll safety -->
            <div class="rounded-2xl border border-[var(--border-subtle)] bg-[var(--bg-surface)] overflow-hidden shadow-sm w-full">
              <div class="overflow-x-auto">
                <table class="w-full text-left border-collapse min-w-[650px]">
                  <thead>
                    <tr class="border-b border-[var(--border-subtle)] bg-[var(--bg-canvas)]/40 text-[11px] font-mono uppercase tracking-wider text-[var(--text-muted)]">
                      <th class="py-3 px-5 font-medium">APP</th>
                      <th class="py-3 px-5 font-medium">STATUS</th>
                      <th class="py-3 px-5 font-medium">RAM</th>
                      <th class="py-3 px-5 font-medium">CPU</th>
                      <th class="py-3 px-5 font-medium text-right">ACTION</th>
                    </tr>
                  </thead>
                  <tbody class="divide-y divide-[var(--border-subtle)] text-xs">
                    {#each filteredContainers as container (container.id)}
                      {@const isLocked = Boolean(vaultLockedMap[container.id])}
                      {@const metric = metricsMap[container.id]}
                      {@const status = metric?.status ?? 'closed'}

                      <tr class="hover:bg-[var(--bg-surface-hover)]/60 transition-colors">
                        <!-- 1. APP -->
                        <td class="py-3.5 px-5">
                          <div class="flex items-center gap-3">
                            <AppIcon
                              name={container.name}
                              url={container.url}
                              icon={container.icon}
                              size={36}
                              rounded="rounded-xl"
                            />

                            <div class="min-w-0">
                              <div class="flex items-center gap-1.5">
                                <span class="font-semibold text-[var(--text-primary)] truncate">
                                  {container.name}
                                </span>
                                {#if container.profileLabel}
                                  <span class="px-1.5 py-0.5 rounded text-[10px] font-mono bg-[var(--bg-canvas)] text-[var(--text-muted)] border border-[var(--border-subtle)]">
                                    {container.profileLabel}
                                  </span>
                                {/if}
                              </div>
                              <span class="text-[11px] text-[var(--text-muted)] font-normal block truncate">
                                {getTargetDomain(container.url)}
                              </span>
                            </div>
                          </div>
                        </td>

                        <!-- 2. STATUS -->
                        <td class="py-3.5 px-5">
                          {#if isLocked}
                            <div class="flex items-center gap-1.5 text-[var(--status-locked)]">
                              <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                <rect x="3" y="11" width="18" height="11" rx="2" ry="2"/>
                                <path d="M7 11V7a5 5 0 0 1 10 0v4"/>
                              </svg>
                              <span class="font-medium">Locked</span>
                            </div>
                          {:else if status === 'running'}
                            <div class="flex items-center gap-2 text-[var(--status-running)]">
                              <span class="w-2 h-2 rounded-full bg-[var(--status-running)]"></span>
                              <span class="font-medium">Running</span>
                            </div>
                          {:else if status === 'idle'}
                            <div class="flex items-center gap-2 text-[var(--accent-primary)]">
                              <span class="w-2 h-2 rounded-full bg-[var(--accent-primary)]"></span>
                              <span class="font-medium">Idle</span>
                            </div>
                          {:else if status === 'hibernated'}
                            <div class="flex items-center gap-2 text-[var(--text-muted)]">
                              <span class="w-2 h-2 rounded-full bg-[var(--text-muted)]"></span>
                              <span class="font-medium">Hibernated</span>
                            </div>
                          {:else}
                            <div class="flex items-center gap-2 text-[var(--text-muted)] opacity-70">
                              <span class="w-2 h-2 rounded-full bg-[var(--text-muted)]"></span>
                              <span class="font-medium">Closed</span>
                            </div>
                          {/if}
                        </td>

                        <!-- 3. RAM -->
                        <td class="py-3.5 px-5 font-mono text-xs">
                          {#if !isLocked && status === 'running'}
                            <span class="text-[var(--text-primary)] font-medium">
                              {metric?.memoryMb ? Math.round(metric.memoryMb) + ' MB' : '0 MB'}
                            </span>
                          {:else}
                            <span class="text-[var(--text-muted)]">0 MB</span>
                          {/if}
                        </td>

                        <!-- 4. CPU -->
                        <td class="py-3.5 px-5 font-mono text-xs">
                          {#if !isLocked && status === 'running'}
                            <span class="text-[var(--text-primary)] font-medium">
                              {metric?.cpuUsage ? metric.cpuUsage.toFixed(1) + '%' : '0.0%'}
                            </span>
                          {:else}
                            <span class="text-[var(--text-muted)]">0.0%</span>
                          {/if}
                        </td>

                        <!-- 5. ACTION -->
                        <td class="py-3.5 px-5 text-right">
                          <div class="flex items-center justify-end gap-2">
                            {#if isLocked}
                              <button
                                type="button"
                                onclick={() => handlePrimaryAction(container)}
                                disabled={unlockingCardId === container.id}
                                class="px-4 py-1.5 rounded-lg bg-[var(--accent-primary)] hover:opacity-95 text-[var(--accent-fg)] font-semibold text-xs active:scale-[0.98] transition-colors disabled:opacity-50 shadow-sm cursor-pointer"
                              >
                                {#if unlockingCardId === container.id}
                                  Unlocking...
                                {:else}
                                  Unlock
                                {/if}
                              </button>
                            {:else}
                              <button
                                type="button"
                                onclick={() => handlePrimaryAction(container)}
                                disabled={launchingId === container.id || wakingId === container.id}
                                class="px-4 py-1.5 rounded-lg border border-[var(--border-subtle)] bg-[var(--bg-canvas)] hover:bg-[var(--bg-surface-hover)] text-[var(--text-primary)] font-medium text-xs active:scale-[0.98] transition-colors disabled:opacity-50 cursor-pointer"
                              >
                                {#if launchingId === container.id}
                                  Opening...
                                {:else if wakingId === container.id}
                                  Waking...
                                {:else}
                                  Open app
                                {/if}
                              </button>
                            {/if}

                            <button
                              type="button"
                              onclick={() => handleOpenMenu(container)}
                              class="w-7 h-7 rounded-lg flex items-center justify-center text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-surface-hover)] transition-colors cursor-pointer"
                              aria-label="More options for {container.name}"
                            >
                              <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="currentColor">
                                <circle cx="12" cy="5" r="2"/>
                                <circle cx="12" cy="12" r="2"/>
                                <circle cx="12" cy="19" r="2"/>
                              </svg>
                            </button>
                          </div>
                        </td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
              </div>
            </div>

            <!-- Status Summary Count -->
            <div class="text-xs text-[var(--text-muted)] pt-1 font-normal">
              {filteredContainers.length} {filteredContainers.length === 1 ? 'app' : 'apps'} · {runningCount} running · {idleCount} idle · {lockedCount} locked
            </div>
          {/if}

        {:else}
          <!-- =================== MANAGE CONTAINERS TAB (from .design-handoff) =================== -->

          <!-- Header Row -->
          <div class="flex items-start justify-between gap-4">
            <div>
              <h1 class="text-2xl lg:text-3xl font-bold tracking-tight text-[var(--text-primary)]">
                Manage containers
              </h1>
              <p class="text-sm text-[var(--text-muted)] mt-1.5 font-normal">
                Edit your apps and control their sessions, all in one place.
              </p>
            </div>

            <button
              type="button"
              onclick={() => openCreateModal()}
              class="min-h-[38px] px-5 py-2.5 rounded-xl bg-[var(--accent-primary)] hover:opacity-95 text-[var(--accent-fg)] font-semibold text-xs flex items-center gap-1.5 transition-colors active:scale-[0.98] shadow-sm cursor-pointer"
            >
              <Icon name="add" size={14} />
              <span>Add app</span>
            </button>
          </div>

          <!-- Search & Status Filter Toolbar -->
          <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4 pt-1">
            <!-- Search Containers Input -->
            <div class="relative w-full sm:w-80">
              <div class="absolute inset-y-0 left-0 pl-3.5 flex items-center pointer-events-none text-[var(--text-muted)]">
                <Icon name="search" size={15} />
              </div>
              <input
                type="text"
                bind:value={manageSearchQuery}
                placeholder="Search containers..."
                class="w-full pl-10 pr-8 py-2.5 text-xs bg-[var(--bg-surface)] border border-[var(--border-subtle)] rounded-xl text-[var(--text-primary)] placeholder:text-[var(--text-muted)] focus:outline-none focus:ring-1 focus:ring-[var(--accent-primary)] focus:border-[var(--accent-primary)] transition-colors min-h-[40px]"
              />
              {#if manageSearchQuery}
                <button
                  type="button"
                  onclick={() => (manageSearchQuery = '')}
                  class="absolute inset-y-0 right-0 pr-3 flex items-center text-[var(--text-muted)] hover:text-[var(--text-primary)] cursor-pointer"
                  aria-label="Clear search"
                >
                  <Icon name="close" size={13} />
                </button>
              {/if}
            </div>

            <!-- Status Dropdown Selector -->
            <div class="relative w-full sm:w-44">
              <select
                bind:value={manageStatusFilter}
                class="w-full appearance-none pl-4 pr-10 py-2.5 text-xs bg-[var(--bg-surface)] border border-[var(--border-subtle)] rounded-xl text-[var(--text-primary)] focus:outline-none focus:ring-1 focus:ring-[var(--accent-primary)] transition-colors min-h-[40px] cursor-pointer"
              >
                <option value="all">All statuses</option>
                <option value="running">Running</option>
                <option value="locked">Locked</option>
                <option value="hibernated">Hibernated</option>
              </select>
              <div class="absolute inset-y-0 right-0 pr-3.5 flex items-center pointer-events-none text-[var(--text-muted)]">
                <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="m6 9 6 6 6-6"/>
                </svg>
              </div>
            </div>
          </div>

          <!-- Manage Containers Data Table (APP, WEBSITE, STATUS, ACTIONS) -->
          <div class="rounded-2xl border border-[var(--border-subtle)] bg-[var(--bg-surface)] overflow-hidden shadow-sm w-full">
            <div class="overflow-x-auto">
              <table class="w-full text-left border-collapse min-w-[700px]">
                <thead>
                  <tr class="border-b border-[var(--border-subtle)] bg-[var(--bg-canvas)]/30 text-[11px] font-mono uppercase tracking-wider text-[var(--text-muted)]">
                    <th class="py-3.5 px-6 font-medium w-[32%]">APP</th>
                    <th class="py-3.5 px-6 font-medium w-[28%]">WEBSITE</th>
                    <th class="py-3.5 px-6 font-medium w-[16%]">STATUS</th>
                    <th class="py-3.5 px-6 font-medium text-right w-[24%]">ACTIONS</th>
                  </tr>
                </thead>
                <tbody class="divide-y divide-[var(--border-subtle)] text-xs">
                  {#if manageFilteredContainers.length === 0}
                    <tr>
                      <td colspan="4" class="py-14 text-center text-[var(--text-muted)]">
                        No containers match the current search or filter.
                      </td>
                    </tr>
                  {:else}
                    {#each manageFilteredContainers as container (container.id)}
                      {@const isLocked = Boolean(vaultLockedMap[container.id])}
                      {@const metric = metricsMap[container.id]}
                      {@const status = metric?.status ?? 'closed'}

                      <tr class="hover:bg-[var(--bg-surface-hover)]/60 transition-colors">
                        <!-- 1. APP (Icon Badge + Name) -->
                        <td class="py-4 px-6">
                          <div class="flex items-center gap-3.5">
                            <AppIcon
                              name={container.name}
                              url={container.url}
                              icon={container.icon}
                              size={40}
                              rounded="rounded-xl"
                            />

                            <div class="min-w-0">
                              <div class="flex items-center gap-1.5">
                                <span class="text-sm font-semibold text-[var(--text-primary)] truncate">
                                  {container.name}
                                </span>
                                {#if container.profileLabel}
                                  <span class="px-1.5 py-0.5 rounded text-[10px] font-mono bg-[var(--bg-canvas)] text-[var(--text-muted)] border border-[var(--border-subtle)]">
                                    {container.profileLabel}
                                  </span>
                                {/if}
                              </div>
                            </div>
                          </div>
                        </td>

                        <!-- 2. WEBSITE (Domain) -->
                        <td class="py-4 px-6 text-xs text-[var(--text-muted)] font-normal truncate">
                          {getTargetDomain(container.url)}
                        </td>

                        <!-- 3. STATUS (Dot + Label) -->
                        <td class="py-4 px-6">
                          {#if isLocked}
                            <div class="flex items-center gap-2 text-[var(--text-muted)]">
                              <span class="w-2 h-2 rounded-full bg-[var(--text-muted)]"></span>
                              <span class="font-medium text-xs">Locked</span>
                            </div>
                          {:else if status === 'running' || status === 'idle'}
                            <div class="flex items-center gap-2 text-[var(--status-running)]">
                              <span class="w-2 h-2 rounded-full bg-[var(--status-running)]"></span>
                              <span class="font-medium text-xs">Running</span>
                            </div>
                          {:else if status === 'hibernated'}
                            <div class="flex items-center gap-2 text-[var(--text-muted)]">
                              <span class="w-2 h-2 rounded-full bg-[var(--text-muted)]"></span>
                              <span class="font-medium text-xs">Hibernated</span>
                            </div>
                          {:else}
                            <div class="flex items-center gap-2 text-[var(--text-muted)] opacity-60">
                              <span class="w-2 h-2 rounded-full bg-[var(--text-muted)]"></span>
                              <span class="font-medium text-xs">Closed</span>
                            </div>
                          {/if}
                        </td>

                        <!-- 4. ACTIONS (Edit, Stop/Unlock/Open, ⋮) -->
                        <td class="py-4 px-6 text-right">
                          <div class="flex items-center justify-end gap-2.5">
                            <!-- Edit Button -->
                            <button
                              type="button"
                              onclick={() => handleOpenMenu(container)}
                              class="px-4 py-1.5 rounded-xl border border-[var(--border-subtle)] bg-[var(--bg-canvas)] hover:bg-[var(--bg-surface-hover)] text-[var(--text-primary)] font-medium text-xs transition-colors cursor-pointer"
                            >
                              Edit
                            </button>

                            <!-- Status Action Button (Stop / Unlock / Open) -->
                            {#if isLocked}
                              <button
                                type="button"
                                onclick={() => handlePrimaryAction(container)}
                                disabled={unlockingCardId === container.id}
                                class="px-5 py-1.5 rounded-xl bg-[var(--accent-primary)] hover:opacity-95 text-[var(--accent-fg)] font-semibold text-xs active:scale-[0.98] transition-colors disabled:opacity-50 shadow-sm cursor-pointer"
                              >
                                {#if unlockingCardId === container.id}
                                  Unlocking...
                                {:else}
                                  Unlock
                                {/if}
                              </button>
                            {:else if status === 'running' || status === 'idle'}
                              <button
                                type="button"
                                onclick={() => handleForceHibernate(container)}
                                disabled={actionLoadingId === container.id}
                                class="px-4 py-1.5 rounded-xl border border-[var(--border-subtle)] bg-[var(--bg-canvas)] hover:bg-[var(--bg-surface-hover)] text-[var(--text-primary)] font-medium text-xs active:scale-[0.98] transition-colors disabled:opacity-50 cursor-pointer"
                              >
                                {#if actionLoadingId === container.id}
                                  Stopping...
                                {:else}
                                  Stop
                                {/if}
                              </button>
                            {:else if status === 'hibernated'}
                              <button
                                type="button"
                                onclick={() => handleWake(container)}
                                disabled={wakingId === container.id}
                                class="px-4 py-1.5 rounded-xl border border-[var(--border-subtle)] bg-[var(--bg-canvas)] hover:bg-[var(--bg-surface-hover)] text-[var(--text-primary)] font-medium text-xs active:scale-[0.98] transition-colors disabled:opacity-50 cursor-pointer"
                              >
                                {#if wakingId === container.id}
                                  Waking...
                                {:else}
                                  Wake
                                {/if}
                              </button>
                            {:else}
                              <button
                                type="button"
                                onclick={() => handlePrimaryAction(container)}
                                disabled={launchingId === container.id}
                                class="px-4 py-1.5 rounded-xl border border-[var(--border-subtle)] bg-[var(--bg-canvas)] hover:bg-[var(--bg-surface-hover)] text-[var(--text-primary)] font-medium text-xs active:scale-[0.98] transition-colors disabled:opacity-50 cursor-pointer"
                              >
                                {#if launchingId === container.id}
                                  Opening...
                                {:else}
                                  Open
                                {/if}
                              </button>
                            {/if}

                            <!-- Overflow Menu Button ⋮ -->
                            <button
                              type="button"
                              onclick={() => handleOpenMenu(container)}
                              class="w-8 h-8 rounded-xl border border-[var(--border-subtle)] bg-[var(--bg-canvas)] hover:bg-[var(--bg-surface-hover)] flex items-center justify-center text-[var(--text-muted)] hover:text-[var(--text-primary)] transition-colors cursor-pointer shrink-0"
                              aria-label="More options for {container.name}"
                            >
                              <svg class="w-4 h-4" viewBox="0 0 24 24" fill="currentColor">
                                <circle cx="12" cy="5" r="2"/>
                                <circle cx="12" cy="12" r="2"/>
                                <circle cx="12" cy="19" r="2"/>
                              </svg>
                            </button>
                          </div>
                        </td>
                      </tr>
                    {/each}
                  {/if}
                </tbody>
              </table>
            </div>
          </div>

          <!-- Summary count and Privacy Link matching reference screenshot -->
          <div class="flex flex-col gap-2 pt-1 font-normal">
            <div class="text-xs text-[var(--text-muted)]">
              {manageFilteredContainers.length} {manageFilteredContainers.length === 1 ? 'container' : 'containers'} · {runningCount} running · {idleCount} idle · {lockedCount} locked
            </div>
            <div>
              <button
                type="button"
                onclick={() => (showDocModal = true)}
                class="text-xs text-[var(--text-muted)] hover:text-[var(--text-primary)] transition-colors inline-flex items-center gap-1 focus-visible:outline-none cursor-pointer"
              >
                <span>Container privacy & isolation</span>
                <span class="text-[11px] leading-none">↗</span>
              </button>
            </div>
          </div>
        {/if}
      </div>

      <!-- Sticky Bottom Dynamic Telemetry Footer -->
      <footer class="mt-auto pt-8 pb-4 flex items-center justify-between text-xs text-[var(--text-muted)] border-t border-[var(--border-subtle)] select-none">
        <div class="flex items-center gap-2.5">
          <span class="font-semibold text-[var(--text-primary)]">Vessel</span>
          <span class="text-[var(--border-subtle)]">·</span>
          <span>{footerStatusText}</span>
        </div>
        <div class="flex items-center gap-4">
          <button
            type="button"
            onclick={handleOpenDocs}
            class="hover:text-[var(--text-primary)] transition-colors flex items-center gap-1 focus-visible:outline-none cursor-pointer"
            title="Open online technical documentation & empirical benchmarks"
          >
            <span>Documentation & Benchmarks</span>
            <span class="text-[11px] leading-none">↗</span>
          </button>
          <button
            type="button"
            onclick={() => (showDocModal = true)}
            class="hover:text-[var(--text-primary)] transition-colors flex items-center gap-1 focus-visible:outline-none cursor-pointer"
            title="Shortcuts and architecture summary"
          >
            <span>Help</span>
          </button>
        </div>
      </footer>
    </div>
  </main>
</div>

<!-- =================== MODALS & DIALOGS =================== -->

<!-- New Container Modal -->
{#if showCreateModal}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/80 animate-in fade-in"
    onclick={(e) => {
      if (e.target === e.currentTarget) closeCreateModal();
    }}
    role="presentation"
  >
    <div
      class="bg-[var(--bg-surface)] border border-[var(--border-subtle)] rounded-2xl w-full max-w-md shadow-2xl overflow-hidden flex flex-col animate-in zoom-in-95 select-none"
      role="dialog"
      aria-modal="true"
      aria-labelledby="new-container-modal-title"
    >
      <div class="px-6 py-4 border-b border-[var(--border-subtle)] flex items-center justify-between">
        <div>
          <h2 id="new-container-modal-title" class="text-sm font-semibold text-[var(--text-primary)]">
            Add Web Container
          </h2>
          <p class="text-xs text-[var(--text-muted)] mt-0.5">
            Creates an isolated sandbox environment with encrypted vaults.
          </p>
        </div>
        <button
          type="button"
          onclick={closeCreateModal}
          disabled={isSubmitting}
          class="inline-flex items-center justify-center min-w-[36px] min-h-[36px] rounded-lg text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-surface-hover)] focus-visible:outline-none transition-colors cursor-pointer"
          aria-label="Close dialog"
        >
          <Icon name="close" size={16} />
        </button>
      </div>

      <form onsubmit={handleCreateContainer} class="p-6 space-y-4">
        {#if modalError}
          <div
            class="p-3 rounded-xl bg-rose-500/10 border border-rose-500/20 text-rose-300 text-xs flex items-center gap-2"
            role="alert"
          >
            <Icon name="warning" size={16} class="text-rose-400 shrink-0" />
            <span>{modalError}</span>
          </div>
        {/if}

        <div>
          <label for="app-url" class="block text-xs font-medium text-[var(--text-primary)] mb-1.5">
            URL
          </label>
          <input
            id="app-url"
            bind:this={urlInputRef}
            type="text"
            bind:value={newUrl}
            oninput={() => {
              if (!hasUserEditedName) {
                newName = extractTitleFromUrl(newUrl);
              }
            }}
            placeholder="https://linear.app"
            required
            disabled={isSubmitting}
            class="w-full px-3 py-2 text-xs font-mono bg-[var(--bg-canvas)] border border-[var(--border-subtle)] rounded-xl text-[var(--text-primary)] placeholder:text-[var(--text-muted)] focus:border-[var(--accent-primary)] focus:ring-1 focus:ring-[var(--accent-primary)] outline-none transition-colors min-h-[40px]"
          />
        </div>

        <div>
          <div class="flex items-center justify-between mb-1.5">
            <label for="app-name" class="block text-xs font-medium text-[var(--text-primary)]">
              Container Name
            </label>
            {#if hasUserEditedName}
              <button
                type="button"
                onclick={() => {
                  hasUserEditedName = false;
                  newName = extractTitleFromUrl(newUrl);
                }}
                class="text-[11px] text-[var(--text-muted)] hover:text-[var(--text-primary)] transition-colors cursor-pointer"
              >
                Auto-detect
              </button>
            {/if}
          </div>
          <input
            id="app-name"
            type="text"
            bind:value={newName}
            oninput={() => (hasUserEditedName = true)}
            placeholder="e.g. Linear"
            disabled={isSubmitting}
            class="w-full px-3 py-2 text-xs bg-[var(--bg-canvas)] border border-[var(--border-subtle)] rounded-xl text-[var(--text-primary)] placeholder:text-[var(--text-muted)] focus:border-[var(--accent-primary)] focus:ring-1 focus:ring-[var(--accent-primary)] outline-none transition-colors min-h-[40px]"
          />
        </div>

        <div>
          <span class="text-[11px] font-semibold text-[var(--text-muted)] uppercase tracking-wider block mb-2 font-mono">
            Common App Presets
          </span>
          <div class="flex flex-wrap gap-2">
            {#each COMMON_PRESETS as preset}
              <button
                type="button"
                onclick={() => {
                  newUrl = preset.url;
                  newName = preset.name;
                  hasUserEditedName = true;
                }}
                disabled={isSubmitting}
                class="flex items-center gap-2 px-3 py-1.5 rounded-xl bg-[var(--bg-canvas)] border border-[var(--border-subtle)] hover:bg-[var(--bg-surface-hover)] active:scale-[0.98] text-xs text-[var(--text-muted)] hover:text-[var(--text-primary)] focus-visible:outline-none transition-colors cursor-pointer"
              >
                <AppIcon name={preset.name} url={preset.url} size={18} rounded="rounded-sm" />
                <span>{preset.name}</span>
              </button>
            {/each}
          </div>
        </div>

        <div class="pt-3 flex items-center justify-end gap-2.5">
          <button
            type="button"
            onclick={closeCreateModal}
            disabled={isSubmitting}
            class="min-h-[40px] px-4 rounded-xl text-xs font-medium text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-surface-hover)] focus-visible:outline-none transition-colors cursor-pointer"
          >
            Cancel
          </button>
          <button
            type="submit"
            disabled={isSubmitting}
            class="min-h-[40px] inline-flex items-center gap-2 px-5 rounded-xl bg-[var(--accent-primary)] hover:opacity-95 text-[var(--accent-fg)] text-xs font-semibold active:scale-[0.98] focus-visible:outline-none transition-colors shadow-sm disabled:opacity-50 cursor-pointer"
          >
            {#if isSubmitting}
              <Icon name="spinner" size={14} />
              <span>Installing...</span>
            {:else}
              <Icon name="check" size={14} />
              <span>Install Container</span>
            {/if}
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}

<!-- Per-Container Settings Menu Modal -->
<ContainerMenuModal
  isOpen={showContainerMenuModal}
  container={selectedContainerForMenu}
  onSave={handleSaveContainerSettings}
  onPinDesktop={handlePinDesktop}
  onDelete={promptDelete}
  onClose={() => {
    showContainerMenuModal = false;
    selectedContainerForMenu = null;
  }}
/>

<!-- Vault Unlock Modal -->
<VaultUnlockModal
  isOpen={showUnlockModal}
  container={unlockTarget}
  vaultMode={unlockTargetVaultMode}
  {isUnlocking}
  error={unlockError}
  onUnlock={handleUnlockVault}
  onClose={() => {
    showUnlockModal = false;
    unlockTarget = null;
  }}
/>

<!-- Settings Panel -->
<SettingsPanel
  isOpen={isSettingsOpen}
  theme={$appSettings.theme}
  viewMode={$appSettings.viewMode}
  density={$appSettings.density}
  isDemoMode={$appSettings.isDemoMode}
  globalIdleThresholdMinutes={$appSettings.globalIdleThresholdMinutes}
  muteAllNotifications={$appSettings.muteAllNotifications}
  onUpdateTheme={(t) => setTheme(t)}
  onUpdateViewMode={(vm) => setViewMode(vm)}
  onUpdateDensity={(d) => updateAppSettings({ density: d })}
  onToggleDemoMode={(enabled) => updateAppSettings({ isDemoMode: enabled })}
  onUpdateIdleThreshold={(m) => updateAppSettings({ globalIdleThresholdMinutes: m })}
  onToggleMuteAll={(muted) => updateAppSettings({ muteAllNotifications: muted })}
  onClose={() => (isSettingsOpen = false)}
/>

<!-- Documentation Modal -->
<DocumentationModal
  isOpen={showDocModal}
  onClose={() => (showDocModal = false)}
/>

<!-- Delete Confirmation Modal -->
{#if confirmDeleteContainer}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/80 animate-in fade-in"
    onclick={(e) => {
      if (e.target === e.currentTarget && !isDeleting) confirmDeleteContainer = null;
    }}
    role="presentation"
  >
    <div
      class="bg-[var(--bg-surface)] border border-[var(--border-subtle)] rounded-2xl w-full max-w-sm shadow-2xl p-5 space-y-4 animate-in zoom-in-95 select-none"
      role="dialog"
      aria-modal="true"
      aria-labelledby="confirm-delete-title"
    >
      <div class="flex items-start gap-3">
        <div
          class="w-9 h-9 rounded-xl bg-rose-500/10 border border-rose-500/20 flex items-center justify-center shrink-0 text-rose-400"
        >
          <Icon name="warning" size={18} />
        </div>
        <div>
          <h3 id="confirm-delete-title" class="text-sm font-semibold text-[var(--text-primary)]">
            Delete Container
          </h3>
          <p class="text-xs text-[var(--text-muted)] mt-1 leading-relaxed">
            Are you sure you want to remove <span class="font-semibold text-[var(--text-primary)]">"{confirmDeleteContainer.name}"</span>?
            This will erase its cookies, session storage, and AES-256-GCM vault files permanently.
          </p>
        </div>
      </div>

      <div class="flex items-center justify-end gap-2.5 pt-2">
        <button
          type="button"
          onclick={() => (confirmDeleteContainer = null)}
          disabled={isDeleting}
          class="min-h-[40px] px-4 rounded-xl text-xs font-medium text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-surface-hover)] focus-visible:outline-none transition-colors cursor-pointer"
        >
          Cancel
        </button>
        <button
          type="button"
          onclick={confirmDelete}
          disabled={isDeleting}
          class="min-h-[40px] inline-flex items-center gap-2 px-4 rounded-xl bg-rose-600 hover:bg-rose-500 text-white text-xs font-semibold active:scale-[0.98] focus-visible:outline-none transition-colors disabled:opacity-50 cursor-pointer shadow-sm"
        >
          {#if isDeleting}
            <Icon name="spinner" size={14} />
            <span>Deleting...</span>
          {:else}
            <span>Delete & Wipe</span>
          {/if}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Quick Switcher Command Palette (Ctrl+K) -->
<QuickSwitcher
  isOpen={showQuickSwitcher}
  {containers}
  {metricsMap}
  {vaultLockedMap}
  onSelectContainer={(c) => {
    showQuickSwitcher = false;
    handlePrimaryAction(c);
  }}
  onLockAll={() => {
    showQuickSwitcher = false;
    handleLockAll();
  }}
  onHibernateAll={() => {
    showQuickSwitcher = false;
    handleHibernateAll();
  }}
  onNewContainer={() => {
    showQuickSwitcher = false;
    openCreateModal();
  }}
  onClose={() => (showQuickSwitcher = false)}
/>

<ToastContainer />
