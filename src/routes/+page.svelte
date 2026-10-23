<script lang="ts">
  import { tick } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import type { ContainerConfig, ContainerProcessMetrics } from '$lib/types';
  import { showToast } from '$lib/stores/toastStore';
  import ToastContainer from '$lib/components/ToastContainer.svelte';
  import VesselLogo from '$lib/components/VesselLogo.svelte';
  import PerformanceRibbon from '$lib/components/PerformanceRibbon.svelte';
  import VaultUnlockModal from '$lib/components/VaultUnlockModal.svelte';

  // Svelte 5 Runes State
  let containers = $state<ContainerConfig[]>([]);
  let metricsMap = $state<Record<string, ContainerProcessMetrics>>({});
  let vaultLockedMap = $state<Record<string, boolean>>({});
  let isLoading = $state(true);
  let searchQuery = $state('');
  let searchInputRef = $state<HTMLInputElement | null>(null);
  let urlInputRef = $state<HTMLInputElement | null>(null);

  // Create Modal State
  let showCreateModal = $state(false);
  let newName = $state('');
  let newUrl = $state('');
  let hasUserEditedName = $state(false);
  let isSubmitting = $state(false);
  let modalError = $state('');

  // Vault Unlock Modal State
  let showUnlockModal = $state(false);
  let unlockTarget = $state<ContainerConfig | null>(null);
  let isUnlocking = $state(false);
  let unlockError = $state('');

  // Container Action States
  let launchingId = $state<string | null>(null);
  let actionLoadingId = $state<string | null>(null);
  let openMenuId = $state<string | null>(null);
  let confirmDeleteContainer = $state<ContainerConfig | null>(null);
  let isDeleting = $state(false);
  let failedIcons = $state(new Set<string>());

  // Presets
  const presets = [
    { name: 'ChatGPT', url: 'https://chatgpt.com', category: 'AI' },
    { name: 'Claude', url: 'https://claude.ai', category: 'AI' },
    { name: 'Linear', url: 'https://linear.app', category: 'Dev' },
    { name: 'GitHub', url: 'https://github.com', category: 'Dev' },
    { name: 'Slack', url: 'https://slack.com', category: 'Chat' },
    { name: 'Discord', url: 'https://discord.com/app', category: 'Chat' },
    { name: 'Notion', url: 'https://notion.so', category: 'Productivity' },
    { name: 'Spotify', url: 'https://open.spotify.com', category: 'Media' }
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
    'twitter.com': 'X / Twitter',
    'x.com': 'X',
    'youtube.com': 'YouTube',
    'reddit.com': 'Reddit',
    'gmail.com': 'Gmail',
    'mail.google.com': 'Gmail',
    'calendar.google.com': 'Google Calendar',
    'web.whatsapp.com': 'WhatsApp',
    'whatsapp.com': 'WhatsApp',
    'web.telegram.org': 'Telegram',
    'telegram.org': 'Telegram',
  };

  function extractTitleFromUrl(rawUrl: string): string {
    const trimmed = rawUrl.trim();
    if (!trimmed) return '';
    try {
      const formatted = trimmed.startsWith('http://') || trimmed.startsWith('https://')
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
      const formatted = rawUrl.startsWith('http://') || rawUrl.startsWith('https://')
        ? rawUrl
        : 'https://' + rawUrl;
      const parsed = new URL(formatted);
      return parsed.hostname.replace(/^www\./, '');
    } catch {
      return rawUrl.replace(/^https?:\/\//, '').split('/')[0];
    }
  }

  function formatIdleTime(seconds: number): string {
    if (seconds < 60) return `${seconds}s`;
    const mins = Math.floor(seconds / 60);
    if (mins < 60) return `${mins}m`;
    const hours = Math.floor(mins / 60);
    return `${hours}h ${mins % 60}m`;
  }

  function handleUrlInput() {
    if (!hasUserEditedName) {
      newName = extractTitleFromUrl(newUrl);
    }
  }

  // Derived Performance Metrics
  let filteredContainers = $derived(
    containers.filter(c => {
      if (!searchQuery.trim()) return true;
      const q = searchQuery.toLowerCase();
      return c.name.toLowerCase().includes(q) || c.url.toLowerCase().includes(q) || c.id.toLowerCase().includes(q);
    })
  );

  let activeCount = $derived(
    Object.values(metricsMap).filter(m => m.status === 'running' || m.status === 'idle').length
  );

  let hibernatedCount = $derived(
    Object.values(metricsMap).filter(m => m.status === 'hibernated').length
  );

  let lockedCount = $derived(
    Object.values(vaultLockedMap).filter(Boolean).length
  );

  let totalActiveRam = $derived(
    Object.values(metricsMap)
      .filter(m => m.status === 'running' || m.status === 'idle')
      .reduce((sum, m) => sum + (m.memoryMb || 0), 0)
  );

  // Savings benchmarked against ~350 MB typical Electron instance
  let totalSavedRam = $derived(
    Math.max(0, (hibernatedCount + lockedCount) * 350 + Math.max(0, activeCount * 350 - totalActiveRam))
  );

  async function loadContainers() {
    try {
      containers = await invoke<ContainerConfig[]>('list_containers');
    } catch (err) {
      console.error('Failed to load containers:', err);
      showToast(String(err), 'error');
    } finally {
      isLoading = false;
    }
  }

  async function pollTelemetry() {
    try {
      const metricsList = await invoke<ContainerProcessMetrics[]>('get_container_metrics');
      const newMap: Record<string, ContainerProcessMetrics> = {};
      for (const m of metricsList) {
        newMap[m.id] = m;
      }
      metricsMap = newMap;

      // Query locked status
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

  // Real-Time Polling Effect
  $effect(() => {
    loadContainers().then(() => {
      pollTelemetry();
    });

    const interval = setInterval(() => {
      pollTelemetry();
    }, 3000);

    return () => clearInterval(interval);
  });

  async function openCreateModal(preset?: { name: string; url: string }) {
    openMenuId = null;
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

    const finalName = cleanName || extractTitleFromUrl(cleanUrl) || getTargetDomain(cleanUrl);

    isSubmitting = true;
    try {
      const created = await invoke<ContainerConfig>('create_container', {
        name: finalName,
        url: cleanUrl
      });
      containers = [created, ...containers.filter(c => c.id !== created.id)];
      showToast(`Installed "${created.name}"`, 'success');
      showCreateModal = false;
      newName = '';
      newUrl = '';
      pollTelemetry();
    } catch (err) {
      modalError = String(err);
      showToast(`Install failed: ${err}`, 'error');
    } finally {
      isSubmitting = false;
    }
  }

  async function handleLaunchOrWake(container: ContainerConfig) {
    openMenuId = null;
    if (launchingId) return;

    // If vault is locked, prompt unlock modal or attempt launch
    if (vaultLockedMap[container.id]) {
      unlockTarget = container;
      unlockError = '';
      showUnlockModal = true;
      return;
    }

    launchingId = container.id;
    try {
      await invoke('launch_container', { id: container.id });
      setTimeout(pollTelemetry, 600);
    } catch (err) {
      const msg = String(err);
      if (msg.includes('vault') || msg.includes('passphrase') || msg.includes('locked')) {
        unlockTarget = container;
        unlockError = msg;
        showUnlockModal = true;
      } else {
        console.error('Launch failed:', err);
        showToast(`Launch failed: ${err}`, 'error');
      }
    } finally {
      launchingId = null;
    }
  }

  async function handleWake(container: ContainerConfig) {
    openMenuId = null;
    if (launchingId) return;
    launchingId = container.id;
    try {
      await invoke('wake_container', { id: container.id });
      showToast(`Waking "${container.name}"...`, 'info');
      setTimeout(pollTelemetry, 500);
    } catch (err) {
      showToast(`Wake failed: ${err}`, 'error');
    } finally {
      launchingId = null;
    }
  }

  async function handleForceHibernate(container: ContainerConfig) {
    openMenuId = null;
    actionLoadingId = container.id;
    try {
      const res = await invoke<string>('force_hibernate_container', { id: container.id });
      showToast(res || `Hibernated "${container.name}"`, 'info');
      setTimeout(pollTelemetry, 400);
    } catch (err) {
      showToast(`Hibernate failed: ${err}`, 'error');
    } finally {
      actionLoadingId = null;
    }
  }

  async function handleLockVault(container: ContainerConfig) {
    openMenuId = null;
    actionLoadingId = container.id;
    try {
      const res = await invoke<string>('lock_container_vault', { id: container.id });
      showToast(res || `Vault for "${container.name}" locked and encrypted`, 'success');
      setTimeout(pollTelemetry, 400);
    } catch (err) {
      showToast(`Vault encryption failed: ${err}`, 'error');
    } finally {
      actionLoadingId = null;
    }
  }

  async function handleUnlockVault(passphrase?: string) {
    if (!unlockTarget) return;
    isUnlocking = true;
    unlockError = '';
    try {
      await invoke('unlock_container_vault', { id: unlockTarget.id, passphrase });
      await invoke('launch_container', { id: unlockTarget.id });
      showToast(`Unlocked and launched "${unlockTarget.name}"`, 'success');
      showUnlockModal = false;
      setTimeout(pollTelemetry, 500);
    } catch (err) {
      unlockError = String(err);
      showToast(`Unlock failed: ${err}`, 'error');
    } finally {
      isUnlocking = false;
    }
  }

  async function handleCreateShortcut(container: ContainerConfig) {
    openMenuId = null;
    try {
      const res = await invoke<string>('create_shortcut', { id: container.id });
      showToast(res || `Desktop shortcut pinned for "${container.name}"`, 'success');
    } catch (err) {
      console.error('Shortcut failed:', err);
      showToast(`Shortcut failed: ${err}`, 'error');
    }
  }

  function promptDelete(container: ContainerConfig) {
    openMenuId = null;
    confirmDeleteContainer = container;
  }

  async function confirmDelete() {
    if (!confirmDeleteContainer) return;
    const target = confirmDeleteContainer;
    isDeleting = true;
    try {
      await invoke('delete_container', { id: target.id });
      containers = containers.filter(c => c.id !== target.id);
      showToast(`Removed "${target.name}" and purged vault data`, 'info');
      confirmDeleteContainer = null;
      pollTelemetry();
    } catch (err) {
      console.error('Delete failed:', err);
      showToast(`Delete failed: ${err}`, 'error');
    } finally {
      isDeleting = false;
    }
  }

  function toggleMenu(id: string, e: MouseEvent) {
    e.stopPropagation();
    openMenuId = openMenuId === id ? null : id;
  }

  function handleWindowClick(e: MouseEvent) {
    const target = e.target as HTMLElement | null;
    if (!target?.closest('.three-dot-menu-container')) {
      openMenuId = null;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      if (confirmDeleteContainer) {
        confirmDeleteContainer = null;
      } else if (showUnlockModal && !isUnlocking) {
        showUnlockModal = false;
      } else if (showCreateModal && !isSubmitting) {
        closeCreateModal();
      } else if (openMenuId) {
        openMenuId = null;
      }
    } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'n') {
      e.preventDefault();
      openCreateModal();
    } else if (e.key === '/' && !showCreateModal && !confirmDeleteContainer && !showUnlockModal) {
      const activeTag = document.activeElement?.tagName.toLowerCase();
      if (activeTag !== 'input' && activeTag !== 'textarea') {
        e.preventDefault();
        searchInputRef?.focus();
      }
    }
  }
</script>

<svelte:window onclick={handleWindowClick} onkeydown={handleKeydown} />

<div class="h-screen w-screen overflow-hidden select-none flex flex-col bg-vessel-bg text-zinc-100 font-sans">
  
  <!-- Minimalist Desktop Top Bar -->
  <header class="h-14 border-b border-zinc-800 bg-vessel-bg/95 backdrop-blur-md px-6 flex items-center justify-between shrink-0 z-20">
    <!-- Brand Title -->
    <div class="flex items-center gap-3">
      <VesselLogo size={26} />
      <span class="text-sm font-semibold tracking-tight text-zinc-100">Vessel</span>
    </div>

    <!-- Quick Search Filter & [ New App ] Button -->
    <div class="flex items-center gap-3">
      <div class="relative w-64">
        <span class="material-symbols-outlined absolute left-2.5 top-1/2 -translate-y-1/2 text-zinc-500 text-base pointer-events-none">
          search
        </span>
        <input
          bind:this={searchInputRef}
          type="text"
          bind:value={searchQuery}
          placeholder="Filter containers..."
          class="w-full bg-zinc-900 border border-zinc-700/80 rounded-md pl-8 pr-7 py-1.5 text-xs text-zinc-100 placeholder:text-zinc-500 focus:border-zinc-500 focus:ring-1 focus:ring-zinc-500 outline-none transition-all duration-150 ease-out"
        />
        {#if searchQuery}
          <button
            type="button"
            onclick={() => searchQuery = ''}
            class="absolute right-2 top-1/2 -translate-y-1/2 text-zinc-500 hover:text-zinc-200 p-0.5 rounded transition-colors duration-150"
            aria-label="Clear filter"
          >
            <span class="material-symbols-outlined text-xs">close</span>
          </button>
        {:else}
          <kbd class="absolute right-2.5 top-1/2 -translate-y-1/2 text-[10px] text-zinc-500 border border-zinc-700/60 rounded px-1 font-mono pointer-events-none">
            /
          </kbd>
        {/if}
      </div>

      <button
        type="button"
        onclick={() => openCreateModal()}
        class="flex items-center gap-1.5 px-3 py-1.5 rounded-md bg-zinc-100 text-zinc-950 font-semibold text-xs hover:bg-white active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-zinc-400 transition-all duration-150 ease-out shadow-sm"
      >
        <span class="material-symbols-outlined text-sm leading-none font-bold">add</span>
        <span>New Container</span>
      </button>
    </div>
  </header>

  <!-- Confined Scroll Area -->
  <main class="flex-1 overflow-y-auto vessel-scroll p-6 lg:p-8 flex flex-col">
    <div class="max-w-7xl w-full mx-auto space-y-6 flex-1">

      <!-- Global Performance Ribbon -->
      <PerformanceRibbon
        {activeCount}
        {hibernatedCount}
        {lockedCount}
        {totalActiveRam}
        {totalSavedRam}
      />

      <!-- Section Header -->
      <div class="flex items-center justify-between pb-2 border-b border-zinc-800/80">
        <div>
          <h1 class="text-lg font-semibold tracking-tight text-zinc-100">Containers</h1>
          <p class="text-xs text-zinc-400 mt-0.5">
            Isolated browser sandboxes with AES-256-GCM vaults and intelligent resource hibernation.
          </p>
        </div>

        <div class="text-xs text-zinc-500 font-mono">
          {containers.length} {containers.length === 1 ? 'container' : 'containers'}
        </div>
      </div>

      <!-- Loading State -->
      {#if isLoading}
        <div class="flex flex-col items-center justify-center py-24 gap-3 text-zinc-500">
          <span class="material-symbols-outlined animate-spin text-2xl text-zinc-400">progress_activity</span>
          <span class="text-xs font-medium font-mono">Querying supervisor telemetry...</span>
        </div>

      <!-- Empty State -->
      {:else if containers.length === 0}
        <div class="flex flex-col items-center justify-center py-20 px-4 text-center rounded-2xl border border-dashed border-zinc-800 bg-vessel-card/40">
          <div class="w-12 h-12 rounded-xl bg-vessel-card flex items-center justify-center mb-3 border border-zinc-800 text-zinc-400">
            <span class="material-symbols-outlined text-2xl">widgets</span>
          </div>
          <h2 class="text-sm font-semibold text-zinc-200">No web apps installed</h2>
          <p class="text-xs text-zinc-400 max-w-sm mt-1 mb-5 leading-relaxed">
            Install web apps as standalone, isolated containers. Each container maintains independent sessions, cookies, and local cache with native memory supervision.
          </p>

          <button
            type="button"
            onclick={() => openCreateModal()}
            class="flex items-center gap-1.5 px-3.5 py-1.5 rounded-md bg-zinc-100 text-zinc-950 font-semibold text-xs hover:bg-white active:scale-[0.98] transition-all duration-150 ease-out shadow-sm"
          >
            <span class="material-symbols-outlined text-sm font-bold">add</span>
            <span>New Container</span>
          </button>

          <!-- Quick Templates -->
          <div class="mt-10 w-full max-w-xl">
            <p class="text-[10px] font-semibold text-zinc-500 uppercase tracking-wider mb-2.5">
              Popular Presets
            </p>
            <div class="grid grid-cols-2 sm:grid-cols-4 gap-2">
              {#each presets as preset}
                <button
                  type="button"
                  onclick={() => openCreateModal(preset)}
                  class="flex flex-col items-start p-2.5 rounded-lg border border-zinc-800 bg-vessel-card hover:bg-vessel-card-hover hover:border-zinc-700 active:scale-[0.98] transition-all duration-150 ease-out text-left group"
                >
                  <span class="text-xs font-medium text-zinc-200 group-hover:text-white transition-colors duration-150">
                    {preset.name}
                  </span>
                  <span class="text-[10px] text-zinc-500 truncate w-full mt-0.5 font-mono">
                    {getTargetDomain(preset.url)}
                  </span>
                </button>
              {/each}
            </div>
          </div>
        </div>

      <!-- Search Filter Empty -->
      {:else if filteredContainers.length === 0}
        <div class="flex flex-col items-center justify-center py-20 text-center text-zinc-500">
          <span class="material-symbols-outlined text-3xl mb-2 text-zinc-600">search_off</span>
          <p class="text-xs font-medium text-zinc-400">No containers matching "{searchQuery}"</p>
          <button
            type="button"
            onclick={() => searchQuery = ''}
            class="mt-2 text-xs text-zinc-300 hover:text-white hover:underline transition-colors duration-150"
          >
            Clear filter
          </button>
        </div>

      <!-- Container Cards Grid -->
      {:else}
        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 auto-rows-fr">
          {#each filteredContainers as container (container.id)}
            {@const metric = metricsMap[container.id]}
            {@const isLocked = vaultLockedMap[container.id]}
            {@const status = isLocked ? 'locked' : (metric?.status ?? 'closed')}
            
            <div
              class="h-full flex flex-col justify-between p-4 rounded-xl border border-zinc-800 bg-vessel-card hover:border-zinc-700 transition-all duration-150 ease-out shadow-sm relative group {openMenuId === container.id ? 'z-30' : 'z-0'}"
            >
              <!-- Card Top: Icon, Titles & Dynamic Telemetry Badge -->
              <div>
                <div class="flex items-start justify-between gap-3">
                  <div class="flex items-start gap-3 min-w-0">
                    <!-- Icon Container with Fallback -->
                    <div class="w-10 h-10 rounded-lg flex items-center justify-center bg-zinc-800/60 border border-zinc-700/50 shrink-0 overflow-hidden">
                      {#if container.icon && !failedIcons.has(container.id)}
                        <img
                          src={container.icon}
                          alt={container.name}
                          class="w-6 h-6 object-contain"
                          onerror={() => {
                            failedIcons.add(container.id);
                            failedIcons = new Set(failedIcons);
                          }}
                        />
                      {:else}
                        <span class="text-sm font-semibold text-zinc-300 uppercase">
                          {container.name.slice(0, 1)}
                        </span>
                      {/if}
                    </div>

                    <!-- Titles -->
                    <div class="flex-1 min-w-0">
                      <h3 class="text-sm font-medium text-zinc-100 truncate group-hover:text-white transition-colors duration-150" title={container.name}>
                        {container.name}
                      </h3>
                      <span class="text-xs text-zinc-400 font-mono truncate block mt-0.5" title={container.url}>
                        {getTargetDomain(container.url)}
                      </span>
                    </div>
                  </div>

                  <!-- Context Menu Toggle -->
                  <div class="relative three-dot-menu-container">
                    <button
                      type="button"
                      onclick={(e) => toggleMenu(container.id, e)}
                      class="p-1 rounded-md text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800 transition-all duration-150"
                      title="More actions"
                      aria-label="More actions"
                    >
                      <span class="material-symbols-outlined text-base">more_vert</span>
                    </button>

                    <!-- Dropdown Menu -->
                    {#if openMenuId === container.id}
                      <div class="absolute right-0 top-full mt-1.5 w-48 bg-zinc-900 border border-zinc-800 rounded-xl shadow-2xl py-1 z-50 animate-in fade-in zoom-in-95 duration-150">
                        <button
                          type="button"
                          onclick={() => handleCreateShortcut(container)}
                          class="w-full flex items-center gap-2 px-3 py-2 text-xs text-zinc-300 hover:text-white hover:bg-zinc-800/70 transition-colors text-left"
                        >
                          <span class="material-symbols-outlined text-sm text-zinc-400">push_pin</span>
                          <span>Pin to Desktop</span>
                        </button>

                        {#if status === 'running' || status === 'idle'}
                          <button
                            type="button"
                            onclick={() => handleForceHibernate(container)}
                            class="w-full flex items-center gap-2 px-3 py-2 text-xs text-sky-400 hover:text-sky-300 hover:bg-sky-500/10 transition-colors text-left"
                          >
                            <span class="material-symbols-outlined text-sm text-sky-400">bedtime</span>
                            <span>Hibernate Process</span>
                          </button>
                        {/if}

                        {#if !isLocked}
                          <button
                            type="button"
                            onclick={() => handleLockVault(container)}
                            class="w-full flex items-center gap-2 px-3 py-2 text-xs text-purple-400 hover:text-purple-300 hover:bg-purple-500/10 transition-colors text-left"
                          >
                            <span class="material-symbols-outlined text-sm text-purple-400">lock</span>
                            <span>Lock in Vault</span>
                          </button>
                        {:else}
                          <button
                            type="button"
                            onclick={() => {
                              unlockTarget = container;
                              unlockError = '';
                              showUnlockModal = true;
                              openMenuId = null;
                            }}
                            class="w-full flex items-center gap-2 px-3 py-2 text-xs text-purple-300 hover:text-white hover:bg-purple-500/20 transition-colors text-left"
                          >
                            <span class="material-symbols-outlined text-sm text-purple-300">lock_open</span>
                            <span>Unlock Vault</span>
                          </button>
                        {/if}

                        <div class="my-1 border-t border-zinc-800"></div>

                        <button
                          type="button"
                          onclick={() => promptDelete(container)}
                          class="w-full flex items-center gap-2 px-3 py-2 text-xs text-rose-400 hover:text-rose-300 hover:bg-rose-500/10 transition-colors text-left"
                        >
                          <span class="material-symbols-outlined text-sm text-rose-400">delete</span>
                          <span>Delete App & Vault</span>
                        </button>
                      </div>
                    {/if}
                  </div>
                </div>

                <!-- Dynamic Telemetry Status Bar -->
                <div class="mt-3 flex items-center gap-2 font-mono text-[11px]">
                  {#if isLocked}
                    <div class="flex items-center gap-1.5 px-2 py-0.5 rounded-md bg-purple-500/10 border border-purple-500/25 text-purple-400 font-sans font-medium">
                      <span class="material-symbols-outlined text-[12px] leading-none">lock</span>
                      <span>Vault Secured</span>
                    </div>
                  {:else if metric?.status === 'running'}
                    <div class="flex items-center gap-1.5 px-2 py-0.5 rounded-md bg-emerald-500/10 border border-emerald-500/25 text-emerald-400">
                      <span class="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse"></span>
                      <span>Running</span>
                      <span class="text-zinc-500">·</span>
                      <span>{metric.memoryMb.toFixed(1)} MB</span>
                      <span class="text-zinc-500">·</span>
                      <span>{metric.cpuUsage.toFixed(1)}%</span>
                    </div>
                  {:else if metric?.status === 'idle'}
                    <div class="flex items-center gap-1.5 px-2 py-0.5 rounded-md bg-amber-500/10 border border-amber-500/25 text-amber-400">
                      <span class="w-1.5 h-1.5 rounded-full bg-amber-400"></span>
                      <span>Idle {formatIdleTime(metric.idleSeconds)}</span>
                      <span class="text-zinc-500">·</span>
                      <span>{metric.memoryMb.toFixed(1)} MB</span>
                    </div>
                  {:else if metric?.status === 'hibernated'}
                    <div class="flex items-center gap-1.5 px-2 py-0.5 rounded-md bg-sky-500/10 border border-sky-500/25 text-sky-400 font-sans">
                      <span class="material-symbols-outlined text-[12px] leading-none">bedtime</span>
                      <span>Hibernated · 0 MB</span>
                    </div>
                  {:else}
                    <div class="flex items-center gap-1.5 px-2 py-0.5 rounded-md bg-zinc-800/80 border border-zinc-700/40 text-zinc-400 font-sans">
                      <span class="w-1.5 h-1.5 rounded-full bg-zinc-600"></span>
                      <span>Closed</span>
                    </div>
                  {/if}
                </div>
              </div>

              <!-- Card Bottom Actions -->
              <div class="mt-4 pt-3 border-t border-zinc-800/80 flex items-center justify-between gap-2">
                <!-- Primary Action Button -->
                <div>
                  {#if isLocked}
                    <button
                      type="button"
                      onclick={() => handleLaunchOrWake(container)}
                      disabled={launchingId === container.id}
                      class="flex items-center gap-1.5 px-3 py-1.5 rounded-md bg-purple-500/15 hover:bg-purple-500/25 text-purple-300 hover:text-white border border-purple-500/30 text-xs font-semibold active:scale-[0.98] transition-all disabled:opacity-50"
                    >
                      <span class="material-symbols-outlined text-xs">lock_open</span>
                      <span>Unlock & Launch</span>
                    </button>
                  {:else if metric?.status === 'hibernated'}
                    <button
                      type="button"
                      onclick={() => handleWake(container)}
                      disabled={launchingId === container.id}
                      class="flex items-center gap-1.5 px-3 py-1.5 rounded-md bg-sky-500/15 hover:bg-sky-500/25 text-sky-300 hover:text-white border border-sky-500/30 text-xs font-semibold active:scale-[0.98] transition-all disabled:opacity-50"
                    >
                      {#if launchingId === container.id}
                        <span class="material-symbols-outlined text-xs animate-spin">progress_activity</span>
                        <span>Waking...</span>
                      {:else}
                        <span class="material-symbols-outlined text-xs">bedtime</span>
                        <span>Wake Process</span>
                      {/if}
                    </button>
                  {:else if metric?.status === 'running' || metric?.status === 'idle'}
                    <button
                      type="button"
                      onclick={() => handleLaunchOrWake(container)}
                      disabled={launchingId === container.id}
                      class="flex items-center gap-1.5 px-3 py-1.5 rounded-md bg-emerald-500/15 hover:bg-emerald-500/25 text-emerald-300 hover:text-white border border-emerald-500/30 text-xs font-semibold active:scale-[0.98] transition-all disabled:opacity-50"
                    >
                      <span class="material-symbols-outlined text-xs">open_in_new</span>
                      <span>Focus Window</span>
                    </button>
                  {:else}
                    <button
                      type="button"
                      onclick={() => handleLaunchOrWake(container)}
                      disabled={launchingId === container.id}
                      class="flex items-center gap-1.5 px-3 py-1.5 rounded-md bg-zinc-800/80 hover:bg-zinc-700 text-zinc-200 hover:text-white border border-zinc-700/50 text-xs font-medium active:scale-[0.98] transition-all disabled:opacity-50"
                    >
                      {#if launchingId === container.id}
                        <span class="material-symbols-outlined text-xs animate-spin">progress_activity</span>
                        <span>Opening...</span>
                      {:else}
                        <span class="material-symbols-outlined text-xs">play_arrow</span>
                        <span>Launch</span>
                      {/if}
                    </button>
                  {/if}
                </div>

                <!-- Quick Action Buttons -->
                <div class="flex items-center gap-1.5">
                  {#if status === 'running' || status === 'idle'}
                    <!-- Quick Hibernate Button -->
                    <button
                      type="button"
                      onclick={() => handleForceHibernate(container)}
                      disabled={actionLoadingId === container.id}
                      class="p-1.5 rounded-md text-sky-400 hover:text-sky-200 hover:bg-sky-500/10 border border-sky-500/25 active:scale-95 transition-all text-xs flex items-center justify-center"
                      title="Hibernate Process (Reclaim RAM)"
                      aria-label="Hibernate"
                    >
                      {#if actionLoadingId === container.id}
                        <span class="material-symbols-outlined text-xs animate-spin">progress_activity</span>
                      {:else}
                        <span class="material-symbols-outlined text-xs">bedtime</span>
                      {/if}
                    </button>
                  {/if}

                  {#if !isLocked}
                    <!-- Quick Lock Vault Button -->
                    <button
                      type="button"
                      onclick={() => handleLockVault(container)}
                      disabled={actionLoadingId === container.id}
                      class="p-1.5 rounded-md text-purple-400 hover:text-purple-200 hover:bg-purple-500/10 border border-purple-500/25 active:scale-95 transition-all text-xs flex items-center justify-center"
                      title="Lock & Encrypt Vault at Rest"
                      aria-label="Lock Vault"
                    >
                      {#if actionLoadingId === container.id}
                        <span class="material-symbols-outlined text-xs animate-spin">progress_activity</span>
                      {:else}
                        <span class="material-symbols-outlined text-xs">lock</span>
                      {/if}
                    </button>
                  {/if}
                </div>
              </div>

            </div>
          {/each}
        </div>
      {/if}

    </div>
  </main>
</div>

<!-- New App Modal -->
{#if showCreateModal}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-sm transition-all duration-150 ease-out animate-in fade-in"
    onclick={(e) => { if (e.target === e.currentTarget) closeCreateModal(); }}
    role="presentation"
  >
    <div
      class="bg-vessel-card border border-zinc-800 rounded-xl w-full max-w-md shadow-2xl overflow-hidden flex flex-col transition-all duration-150 ease-out animate-in zoom-in-95"
      role="dialog"
      aria-modal="true"
    >
      <div class="px-6 py-4 border-b border-zinc-800 flex items-center justify-between">
        <div>
          <h2 class="text-sm font-semibold text-zinc-100">Install Web Container</h2>
          <p class="text-xs text-zinc-400 mt-0.5">Creates an isolated sandbox environment with encrypted vaults.</p>
        </div>
        <button
          type="button"
          onclick={closeCreateModal}
          disabled={isSubmitting}
          class="text-zinc-500 hover:text-zinc-200 p-1 rounded-md transition-colors duration-150"
          aria-label="Close dialog"
        >
          <span class="material-symbols-outlined text-lg">close</span>
        </button>
      </div>

      <form onsubmit={handleCreateContainer} class="p-6 space-y-4">
        {#if modalError}
          <div class="p-2.5 rounded-md bg-rose-500/10 border border-rose-500/20 text-rose-400 text-xs flex items-center gap-2">
            <span class="material-symbols-outlined text-sm shrink-0">error</span>
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
            oninput={handleUrlInput}
            placeholder="https://linear.app"
            required
            disabled={isSubmitting}
            class="px-3 py-2 text-sm bg-zinc-900 border border-zinc-700/80 rounded-md focus:border-zinc-500 focus:ring-1 focus:ring-zinc-500 outline-none text-zinc-100 placeholder:text-zinc-500 transition-all duration-150 ease-out w-full font-mono text-xs"
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
                onclick={() => { hasUserEditedName = false; newName = extractTitleFromUrl(newUrl); }}
                class="text-[10px] text-zinc-500 hover:text-zinc-300 transition-colors"
              >
                Auto-detect
              </button>
            {/if}
          </div>
          <input
            id="app-name"
            type="text"
            bind:value={newName}
            oninput={() => hasUserEditedName = true}
            placeholder="e.g. Linear"
            disabled={isSubmitting}
            class="px-3 py-2 text-sm bg-zinc-900 border border-zinc-700/80 rounded-md focus:border-zinc-500 focus:ring-1 focus:ring-zinc-500 outline-none text-zinc-100 placeholder:text-zinc-500 transition-all duration-150 ease-out w-full"
          />
        </div>

        <div>
          <span class="text-[10px] uppercase font-semibold text-zinc-500 block mb-1.5">
            Popular Presets
          </span>
          <div class="flex flex-wrap gap-1.5">
            {#each presets as preset}
              <button
                type="button"
                onclick={() => {
                  newUrl = preset.url;
                  newName = preset.name;
                  hasUserEditedName = true;
                }}
                disabled={isSubmitting}
                class="px-2.5 py-1 rounded-md bg-zinc-900 border border-zinc-800 hover:border-zinc-700 active:scale-[0.98] text-[11px] text-zinc-400 hover:text-zinc-200 transition-all duration-150 ease-out"
              >
                {preset.name}
              </button>
            {/each}
          </div>
        </div>

        <div class="pt-2 flex items-center justify-end gap-2.5">
          <button
            type="button"
            onclick={closeCreateModal}
            disabled={isSubmitting}
            class="px-3.5 py-1.5 rounded-md text-xs font-medium text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800 active:scale-[0.98] transition-all duration-150"
          >
            Cancel
          </button>
          <button
            type="submit"
            disabled={isSubmitting}
            class="flex items-center gap-1.5 px-4 py-1.5 rounded-md bg-zinc-100 text-zinc-950 text-xs font-semibold hover:bg-white active:scale-[0.98] transition-all duration-150 shadow-sm disabled:opacity-50"
          >
            {#if isSubmitting}
              <span class="material-symbols-outlined text-xs animate-spin">progress_activity</span>
              <span>Installing...</span>
            {:else}
              <span class="material-symbols-outlined text-xs font-bold">check</span>
              <span>Install Container</span>
            {/if}
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}

<!-- Vault Unlock Modal -->
<VaultUnlockModal
  isOpen={showUnlockModal}
  container={unlockTarget}
  {isUnlocking}
  error={unlockError}
  onUnlock={handleUnlockVault}
  onClose={() => { showUnlockModal = false; unlockTarget = null; }}
/>

<!-- Confirm Delete Modal -->
{#if confirmDeleteContainer}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-sm transition-all duration-150 ease-out animate-in fade-in"
    onclick={(e) => { if (e.target === e.currentTarget && !isDeleting) confirmDeleteContainer = null; }}
    role="presentation"
  >
    <div
      class="bg-vessel-card border border-zinc-800 rounded-xl w-full max-w-sm shadow-2xl p-5 space-y-4 transition-all duration-150 ease-out animate-in zoom-in-95"
      role="dialog"
      aria-modal="true"
    >
      <div class="flex items-start gap-3">
        <div class="w-9 h-9 rounded-lg bg-rose-500/10 border border-rose-500/20 flex items-center justify-center shrink-0">
          <span class="material-symbols-outlined text-lg text-rose-400">warning</span>
        </div>
        <div>
          <h3 class="text-sm font-semibold text-zinc-100">Delete Container</h3>
          <p class="text-xs text-zinc-400 mt-1 leading-relaxed">
            Are you sure you want to remove <span class="font-medium text-zinc-200">"{confirmDeleteContainer.name}"</span>?
            This will permanently erase all cookies, session vaults, and local data.
          </p>
        </div>
      </div>

      <div class="flex items-center justify-end gap-2 pt-1">
        <button
          type="button"
          onclick={() => confirmDeleteContainer = null}
          disabled={isDeleting}
          class="px-3 py-1.5 rounded-md text-xs font-medium text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800 active:scale-[0.98] transition-all duration-150"
        >
          Cancel
        </button>
        <button
          type="button"
          onclick={confirmDelete}
          disabled={isDeleting}
          class="flex items-center gap-1.5 px-3.5 py-1.5 rounded-md bg-rose-500/90 hover:bg-rose-500 text-white text-xs font-semibold active:scale-[0.98] transition-all duration-150 disabled:opacity-50"
        >
          {#if isDeleting}
            <span class="material-symbols-outlined text-xs animate-spin">progress_activity</span>
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
