<script lang="ts">
  import { tick } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import type {
    ContainerConfig,
    ContainerProcessMetrics,
    FilterCategory,
    ContainerCustomSettings,
    VaultMode,
  } from '$lib/types';
  import { showToast } from '$lib/stores/toastStore';
  import { appSettings, updateAppSettings, toggleDensity } from '$lib/stores/appSettings';
  import { getDemoState } from '$lib/fixtures/demoContainers';
  import baselineData from '$lib/data/electron_baseline.json';

  import Sidebar from '$lib/components/Sidebar.svelte';
  import TopBar from '$lib/components/TopBar.svelte';
  import ContainerCard from '$lib/components/ContainerCard.svelte';
  import MemoryHero from '$lib/components/MemoryHero.svelte';
  import VaultUnlockModal from '$lib/components/VaultUnlockModal.svelte';
  import SettingsPanel from '$lib/components/SettingsPanel.svelte';
  import ContainerMenuModal from '$lib/components/ContainerMenuModal.svelte';
  import ToastContainer from '$lib/components/ToastContainer.svelte';
  import Icon from '$lib/components/Icon.svelte';

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

  // Modals & Panels
  let isSettingsOpen = $state(false);
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

  // Action Loading Trackers (for card loading states)
  let launchingId = $state<string | null>(null);
  let wakingId = $state<string | null>(null);
  let unlockingCardId = $state<string | null>(null);
  let actionLoadingId = $state<string | null>(null);

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

  // Combined containers with local customizations (rename, never hibernate, idle threshold, profile label)
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

  // Counts for Sidebar Navigation
  let filterCounts = $derived.by(() => {
    let active = 0;
    let hibernated = 0;
    let locked = 0;

    for (const c of enrichedContainers) {
      if (vaultLockedMap[c.id]) {
        locked += 1;
      } else {
        const m = metricsMap[c.id];
        if (m?.status === 'running' || m?.status === 'idle') {
          active += 1;
        } else if (m?.status === 'hibernated') {
          hibernated += 1;
        }
      }
    }

    return {
      all: enrichedContainers.length,
      active,
      hibernated,
      locked,
    };
  });

  // Filtered Containers according to Sidebar category and Search query
  let filteredContainers = $derived(
    enrichedContainers.filter((c) => {
      // 1. Sidebar Category Filter
      if (activeFilter === 'active') {
        if (vaultLockedMap[c.id]) return false;
        const status = metricsMap[c.id]?.status;
        if (status !== 'running' && status !== 'idle') return false;
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

  // Live Memory Calculations
  let activeCount = $derived(filterCounts.active);
  let idleCount = $derived(
    enrichedContainers.filter(
      (c) => !vaultLockedMap[c.id] && metricsMap[c.id]?.status === 'idle'
    ).length
  );
  let hibernatedCount = $derived(filterCounts.hibernated);
  let lockedCount = $derived(filterCounts.locked);

  let totalActiveRam = $derived(
    Object.entries(metricsMap)
      .filter(([id, m]) => !vaultLockedMap[id] && (m.status === 'running' || m.status === 'idle'))
      .reduce((sum, [, m]) => sum + (m.memoryMb || 0), 0)
  );

  // Estimated memory saved relative to Electron baseline
  let totalSavedRam = $derived(
    Math.max(
      0,
      (hibernatedCount + lockedCount) * baselineData.states.active.meanMb +
        Math.max(0, activeCount * baselineData.states.active.meanMb - totalActiveRam)
    )
  );

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
      // Fallback to empty list or initial state
      if (containers.length === 0) {
        containers = [];
      }
    } finally {
      isLoading = false;
    }
  }

  async function pollTelemetry() {
    if ($appSettings.isDemoMode) {
      // In demo mode, retain simulated metrics without clobbering
      return;
    }

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
    }
  }

  // 3-Second Polling Effect without layout shift
  $effect(() => {
    loadContainers().then(() => {
      pollTelemetry();
    });

    const interval = setInterval(() => {
      pollTelemetry();
    }, 3000);

    return () => clearInterval(interval);
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

    if (status === 'hibernated') {
      handleWake(container);
      return;
    }

    // Launch or focus
    launchingId = container.id;
    try {
      if ($appSettings.isDemoMode) {
        // Simulate demo focus / launch
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

      // Show Toast with Undo action!
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

      // Show Toast with Undo action!
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

  function handleSaveContainerSettings(
    containerId: string,
    settings: ContainerCustomSettings
  ) {
    containerSettingsMap = {
      ...containerSettingsMap,
      [containerId]: settings,
    };
    saveLocalContainerSettings(containerSettingsMap);
    showToast('Preferences updated', 'success');
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
        newName = '';
        newUrl = '';
        pollTelemetry();
      }
    } catch (err) {
      modalError = String(err);
      showToast(`Install failed: ${err}`, 'error');
    } finally {
      isSubmitting = false;
    }
  }

  // One-click install from template presets
  async function installTemplateDirectly(preset: { name: string; url: string }) {
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

    try {
      const created = await invoke<ContainerConfig>('create_container', {
        name: preset.name,
        url: preset.url,
      });
      containers = [created, ...containers.filter((c) => c.id !== created.id)];
      showToast(`Installed "${created.name}"`, 'success');
      pollTelemetry();
    } catch (err) {
      showToast(`Install failed: ${err}`, 'error');
    }
  }

  // Global Keyboard Shortcuts
  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      if (confirmDeleteContainer && !isDeleting) {
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
    } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'n') {
      e.preventDefault();
      openCreateModal();
    } else if (
      e.key === '/' &&
      !showCreateModal &&
      !confirmDeleteContainer &&
      !showUnlockModal &&
      !showContainerMenuModal &&
      !isSettingsOpen
    ) {
      const activeTag = document.activeElement?.tagName.toLowerCase();
      if (activeTag !== 'input' && activeTag !== 'textarea') {
        e.preventDefault();
        searchInputRef?.focus();
      }
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="h-screen w-screen overflow-hidden select-none flex bg-[#0c0e12] text-zinc-100 font-sans">
  <!-- Left Sidebar -->
  <Sidebar
    {activeFilter}
    counts={filterCounts}
    density={$appSettings.density}
    isDemoMode={$appSettings.isDemoMode}
    onSelectFilter={(f) => (activeFilter = f)}
    onOpenSettings={() => (isSettingsOpen = true)}
    onToggleDensity={toggleDensity}
  />

  <!-- Main Content Column -->
  <div class="flex-1 flex flex-col min-w-0 h-full overflow-hidden">
    <!-- Top Bar -->
    <TopBar
      bind:searchQuery
      bind:searchInputRef
      density={$appSettings.density}
      savedMb={totalSavedRam}
      onNewContainer={() => openCreateModal()}
    />

    <!-- Main Scrollable Area -->
    <main class="flex-1 overflow-y-auto vessel-scroll p-6 lg:p-8 flex flex-col">
      <div class="max-w-7xl w-full mx-auto space-y-6 flex-1">
        <!-- Detailed Mode Hero Card (comparing live memory with Electron baseline) -->
        {#if $appSettings.density === 'detailed'}
          <MemoryHero
            {activeCount}
            {idleCount}
            {hibernatedCount}
            {lockedCount}
            totalContainers={enrichedContainers.length}
            {totalActiveRam}
          />
        {/if}

        <!-- Section Header -->
        <div class="flex items-center justify-between pb-2 border-b border-zinc-800">
          <div>
            <h1 class="text-base font-semibold tracking-tight text-zinc-100 capitalize">
              {#if activeFilter === 'all'}
                All Containers
              {:else if activeFilter === 'active'}
                Active Sandboxes
              {:else if activeFilter === 'hibernated'}
                Hibernated Containers
              {:else if activeFilter === 'locked'}
                Encrypted Vaults
              {/if}
            </h1>
            <p class="text-xs text-zinc-400 mt-0.5">
              Isolated web containers with native process supervision and AES-256-GCM vaults.
            </p>
          </div>

          <div class="text-xs text-zinc-400 font-mono">
            {filteredContainers.length} of {enrichedContainers.length} installed
          </div>
        </div>

        <!-- Loading State -->
        {#if isLoading}
          <div class="flex flex-col items-center justify-center py-24 gap-3 text-zinc-400">
            <Icon name="spinner" size={24} class="text-zinc-300" />
            <span class="text-xs font-mono">Connecting to supervisor...</span>
          </div>

        <!-- Empty State (No Containers Installed) -->
        {:else if enrichedContainers.length === 0}
          <div
            class="flex flex-col items-center justify-center py-16 px-6 text-center rounded-2xl border border-dashed border-zinc-800 bg-[#15181e]/40"
          >
            <div
              class="w-12 h-12 rounded-xl bg-zinc-900 flex items-center justify-center mb-3 border border-zinc-800 text-zinc-400"
            >
              <Icon name="grid" size={24} />
            </div>
            <h2 class="text-sm font-semibold text-zinc-100">No web apps installed</h2>
            <p class="text-xs text-zinc-400 max-w-sm mt-1 mb-6 leading-relaxed">
              Launch web apps in dedicated native windows with isolated cookies, encrypted profile vaults, and idle hibernation.
            </p>

            <button
              type="button"
              onclick={() => openCreateModal()}
              class="inline-flex items-center gap-2 min-h-[44px] px-5 rounded-lg bg-zinc-100 hover:bg-white text-zinc-950 font-semibold text-xs active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-zinc-400 transition-colors shadow-sm"
            >
              <Icon name="add" size={16} />
              <span>Create Custom Container</span>
            </button>

            <!-- One-Click Templates for Common Apps -->
            <div class="mt-10 w-full max-w-2xl">
              <span class="text-[11px] font-semibold text-zinc-400 uppercase tracking-wider block mb-3 font-mono">
                One-Click Common App Templates
              </span>
              <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-5 gap-2.5">
                {#each COMMON_PRESETS as preset}
                  <button
                    type="button"
                    onclick={() => installTemplateDirectly(preset)}
                    class="flex flex-col items-start p-3 min-h-[44px] rounded-lg border border-zinc-800 bg-[#15181e] hover:border-zinc-700 hover:bg-zinc-800/80 active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-zinc-400 transition-colors text-left group"
                  >
                    <span class="text-xs font-semibold text-zinc-200 group-hover:text-white">
                      {preset.name}
                    </span>
                    <span class="text-[10px] text-zinc-400 font-mono truncate w-full mt-0.5">
                      {getTargetDomain(preset.url)}
                    </span>
                    <span class="mt-2 text-[10px] text-emerald-400/90 font-medium flex items-center gap-1">
                      <Icon name="add" size={12} />
                      <span>Install</span>
                    </span>
                  </button>
                {/each}
              </div>
            </div>
          </div>

        <!-- Search / Filter Empty State -->
        {:else if filteredContainers.length === 0}
          <div class="flex flex-col items-center justify-center py-20 text-center text-zinc-400">
            <Icon name="search" size={28} class="mb-2 text-zinc-500" />
            <p class="text-xs font-medium text-zinc-300">
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
              class="mt-3 min-h-[44px] px-4 rounded-lg text-xs font-medium text-zinc-200 hover:text-white bg-zinc-900 border border-zinc-800 hover:border-zinc-700 transition-colors"
            >
              Reset Filters
            </button>
          </div>

        <!-- Responsive Card Grid: auto-fill columns with a 300px minimum -->
        {:else}
          <div
            class="grid grid-cols-[repeat(auto-fill,minmax(300px,1fr))] gap-4 auto-rows-fr"
            role="region"
            aria-label="Containers grid"
          >
            {#each filteredContainers as container (container.id)}
              <ContainerCard
                {container}
                metric={metricsMap[container.id]}
                isLocked={vaultLockedMap[container.id]}
                density={$appSettings.density}
                globalIdleThresholdMinutes={$appSettings.globalIdleThresholdMinutes}
                isLaunching={launchingId === container.id}
                isWaking={wakingId === container.id}
                isUnlocking={unlockingCardId === container.id}
                isActionLoading={actionLoadingId === container.id}
                onPrimaryAction={handlePrimaryAction}
                onHibernate={handleForceHibernate}
                onLockVault={handleLockVault}
                onOpenMenu={handleOpenMenu}
              />
            {/each}
          </div>
        {/if}
      </div>
    </main>
  </div>
</div>

<!-- New Container Modal -->
{#if showCreateModal}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-sm animate-in fade-in"
    onclick={(e) => {
      if (e.target === e.currentTarget) closeCreateModal();
    }}
    role="presentation"
  >
    <div
      class="bg-[#15181e] border border-zinc-800 rounded-xl w-full max-w-md shadow-2xl overflow-hidden flex flex-col animate-in zoom-in-95 select-none"
      role="dialog"
      aria-modal="true"
      aria-labelledby="new-container-modal-title"
    >
      <div class="px-6 py-4 border-b border-zinc-800 flex items-center justify-between">
        <div>
          <h2 id="new-container-modal-title" class="text-sm font-semibold text-zinc-100">
            Install Web Container
          </h2>
          <p class="text-xs text-zinc-400 mt-0.5">
            Creates an isolated sandbox environment with encrypted vaults.
          </p>
        </div>
        <button
          type="button"
          onclick={closeCreateModal}
          disabled={isSubmitting}
          class="inline-flex items-center justify-center min-w-[44px] min-h-[44px] rounded-lg text-zinc-400 hover:text-zinc-100 hover:bg-zinc-800 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-zinc-400 transition-colors"
          aria-label="Close dialog"
        >
          <Icon name="close" size={18} />
        </button>
      </div>

      <form onsubmit={handleCreateContainer} class="p-6 space-y-4">
        {#if modalError}
          <div
            class="p-3 rounded-lg bg-rose-500/10 border border-rose-500/20 text-rose-300 text-xs flex items-center gap-2"
            role="alert"
          >
            <Icon name="warning" size={16} class="text-rose-400 shrink-0" />
            <span>{modalError}</span>
          </div>
        {/if}

        <div>
          <label for="app-url" class="block text-xs font-medium text-zinc-300 mb-1.5">
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
            class="w-full px-3 py-2 text-xs font-mono bg-zinc-900 border border-zinc-700/80 rounded-lg text-zinc-100 placeholder:text-zinc-500 focus:border-zinc-400 focus:ring-1 focus:ring-zinc-400 outline-none transition-colors min-h-[44px]"
          />
        </div>

        <div>
          <div class="flex items-center justify-between mb-1.5">
            <label for="app-name" class="block text-xs font-medium text-zinc-300">
              Container Name
            </label>
            {#if hasUserEditedName}
              <button
                type="button"
                onclick={() => {
                  hasUserEditedName = false;
                  newName = extractTitleFromUrl(newUrl);
                }}
                class="text-[11px] text-zinc-400 hover:text-zinc-200 transition-colors"
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
            class="w-full px-3 py-2 text-xs bg-zinc-900 border border-zinc-700/80 rounded-lg text-zinc-100 placeholder:text-zinc-500 focus:border-zinc-400 focus:ring-1 focus:ring-zinc-400 outline-none transition-colors min-h-[44px]"
          />
        </div>

        <div>
          <span class="text-[11px] font-semibold text-zinc-400 uppercase tracking-wider block mb-2 font-mono">
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
                class="px-3 py-1.5 rounded-lg bg-zinc-900 border border-zinc-800 hover:border-zinc-700 active:scale-[0.98] text-xs text-zinc-300 hover:text-zinc-100 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-zinc-400 transition-colors"
              >
                {preset.name}
              </button>
            {/each}
          </div>
        </div>

        <div class="pt-3 flex items-center justify-end gap-2.5">
          <button
            type="button"
            onclick={closeCreateModal}
            disabled={isSubmitting}
            class="min-h-[44px] px-4 rounded-lg text-xs font-medium text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-zinc-400 transition-colors"
          >
            Cancel
          </button>
          <button
            type="submit"
            disabled={isSubmitting}
            class="min-h-[44px] inline-flex items-center gap-2 px-5 rounded-lg bg-zinc-100 hover:bg-white text-zinc-950 text-xs font-semibold active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-zinc-400 transition-colors shadow-sm disabled:opacity-50"
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
  density={$appSettings.density}
  isDemoMode={$appSettings.isDemoMode}
  globalIdleThresholdMinutes={$appSettings.globalIdleThresholdMinutes}
  onUpdateDensity={(d) => updateAppSettings({ density: d })}
  onToggleDemoMode={(enabled) => updateAppSettings({ isDemoMode: enabled })}
  onUpdateIdleThreshold={(m) => updateAppSettings({ globalIdleThresholdMinutes: m })}
  onClose={() => (isSettingsOpen = false)}
/>

<!-- Delete Confirmation Modal -->
{#if confirmDeleteContainer}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-sm animate-in fade-in"
    onclick={(e) => {
      if (e.target === e.currentTarget && !isDeleting) confirmDeleteContainer = null;
    }}
    role="presentation"
  >
    <div
      class="bg-[#15181e] border border-zinc-800 rounded-xl w-full max-w-sm shadow-2xl p-5 space-y-4 animate-in zoom-in-95 select-none"
      role="dialog"
      aria-modal="true"
      aria-labelledby="confirm-delete-title"
    >
      <div class="flex items-start gap-3">
        <div
          class="w-9 h-9 rounded-lg bg-rose-500/10 border border-rose-500/20 flex items-center justify-center shrink-0 text-rose-400"
        >
          <Icon name="warning" size={18} />
        </div>
        <div>
          <h3 id="confirm-delete-title" class="text-sm font-semibold text-zinc-100">
            Delete Container
          </h3>
          <p class="text-xs text-zinc-400 mt-1 leading-relaxed">
            Are you sure you want to remove <span class="font-medium text-zinc-200">"{confirmDeleteContainer.name}"</span>?
            This will erase its cookies, session storage, and AES-256-GCM vault files permanently.
          </p>
        </div>
      </div>

      <div class="flex items-center justify-end gap-2.5 pt-2">
        <button
          type="button"
          onclick={() => (confirmDeleteContainer = null)}
          disabled={isDeleting}
          class="min-h-[44px] px-4 rounded-lg text-xs font-medium text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-zinc-400 transition-colors"
        >
          Cancel
        </button>
        <button
          type="button"
          onclick={confirmDelete}
          disabled={isDeleting}
          class="min-h-[44px] inline-flex items-center gap-2 px-4 rounded-lg bg-rose-600 hover:bg-rose-500 text-white text-xs font-semibold active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-rose-400 transition-colors disabled:opacity-50"
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

<ToastContainer />
