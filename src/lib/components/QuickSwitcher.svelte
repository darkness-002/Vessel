<script lang="ts">
  import { tick } from 'svelte';
  import type { ContainerConfig, ContainerProcessMetrics } from '$lib/types';
  import Icon from '$lib/components/Icon.svelte';
  import AppIcon from '$lib/components/AppIcon.svelte';

  interface Props {
    isOpen: boolean;
    containers: ContainerConfig[];
    metricsMap: Record<string, ContainerProcessMetrics>;
    vaultLockedMap: Record<string, boolean>;
    onSelectContainer: (container: ContainerConfig) => void;
    onLockAll: () => void;
    onHibernateAll: () => void;
    onNewContainer: () => void;
    onClose: () => void;
  }

  let {
    isOpen,
    containers,
    metricsMap,
    vaultLockedMap,
    onSelectContainer,
    onLockAll,
    onHibernateAll,
    onNewContainer,
    onClose,
  }: Props = $props();

  let query = $state('');
  let selectedIndex = $state(0);
  let inputRef = $state<HTMLInputElement | null>(null);

  interface CommandItem {
    id: string;
    type: 'container' | 'action';
    title: string;
    subtitle: string;
    shortcut?: string;
    statusBadge?: { label: string; color: string };
    container?: ContainerConfig;
    action?: () => void;
  }

  let filteredItems = $derived.by<CommandItem[]>(() => {
    const q = query.trim().toLowerCase();
    const items: CommandItem[] = [];

    // 1. Container items
    for (const c of containers) {
      const matchesName = c.name.toLowerCase().includes(q);
      const matchesUrl = c.url.toLowerCase().includes(q);
      const matchesLabel = c.profileLabel?.toLowerCase().includes(q) ?? false;

      if (!q || matchesName || matchesUrl || matchesLabel) {
        let badge = { label: 'Closed', color: 'bg-zinc-800 text-zinc-400' };
        if (vaultLockedMap[c.id]) {
          badge = { label: 'Vault Sealed', color: 'bg-purple-500/20 text-purple-300' };
        } else {
          const status = metricsMap[c.id]?.status;
          if (status === 'running') {
            badge = { label: 'Running', color: 'bg-emerald-500/20 text-emerald-300' };
          } else if (status === 'idle') {
            badge = { label: 'Idle', color: 'bg-amber-500/20 text-amber-300' };
          } else if (status === 'hibernated') {
            badge = { label: 'Hibernated', color: 'bg-sky-500/20 text-sky-300' };
          } else if (status === 'crashed') {
            badge = { label: 'Crashed', color: 'bg-rose-500/20 text-rose-300' };
          }
        }

        items.push({
          id: `container-${c.id}`,
          type: 'container',
          title: c.name,
          subtitle: c.url,
          statusBadge: badge,
          container: c,
        });
      }
    }

    // 2. Global Actions
    const actions: CommandItem[] = [
      {
        id: 'action-lock-all',
        type: 'action',
        title: 'Lock All Containers',
        subtitle: 'Seal all active vaults and clear in-memory encryption keys',
        shortcut: 'Ctrl+Shift+L',
        action: onLockAll,
      },
      {
        id: 'action-hibernate-all',
        type: 'action',
        title: 'Hibernate All Inactive Containers',
        subtitle: 'Free memory by suspending background webview processes',
        action: onHibernateAll,
      },
      {
        id: 'action-new-container',
        type: 'action',
        title: 'Install New Container',
        subtitle: 'Create a sandboxed container with isolated storage',
        shortcut: 'Ctrl+N',
        action: onNewContainer,
      },
    ];

    for (const a of actions) {
      if (!q || a.title.toLowerCase().includes(q) || a.subtitle.toLowerCase().includes(q)) {
        items.push(a);
      }
    }

    return items;
  });

  $effect(() => {
    if (isOpen) {
      query = '';
      selectedIndex = 0;
      tick().then(() => inputRef?.focus());
    }
  });

  $effect(() => {
    // Keep selected index within bounds
    if (selectedIndex >= filteredItems.length && filteredItems.length > 0) {
      selectedIndex = filteredItems.length - 1;
    }
  });

  function handleSelect(item: CommandItem) {
    if (item.type === 'container' && item.container) {
      onSelectContainer(item.container);
      onClose();
    } else if (item.type === 'action' && item.action) {
      item.action();
      onClose();
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (!isOpen) return;

    if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (filteredItems.length > 0) {
        selectedIndex = (selectedIndex + 1) % filteredItems.length;
      }
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (filteredItems.length > 0) {
        selectedIndex = (selectedIndex - 1 + filteredItems.length) % filteredItems.length;
      }
    } else if (e.key === 'Enter') {
      e.preventDefault();
      const current = filteredItems[selectedIndex];
      if (current) {
        handleSelect(current);
      }
    } else if (e.key === 'Escape') {
      e.preventDefault();
      onClose();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if isOpen}
  <div
    class="fixed inset-0 z-50 flex items-start justify-center pt-20 px-4 bg-black/80 animate-in fade-in"
    onclick={(e) => {
      if (e.target === e.currentTarget) onClose();
    }}
    role="presentation"
  >
    <div
      class="bg-[var(--bg-surface)] border border-[var(--border-subtle)] rounded-2xl w-full max-w-xl shadow-2xl overflow-hidden flex flex-col animate-in zoom-in-95 select-none"
      role="dialog"
      aria-modal="true"
      aria-label="Quick Switcher"
    >
      <!-- Search Input -->
      <div class="px-4 py-3 border-b border-[var(--border-subtle)] flex items-center gap-3">
        <Icon name="search" size={18} class="text-[var(--text-muted)] shrink-0" />
        <input
          bind:this={inputRef}
          type="text"
          bind:value={query}
          placeholder="Search containers, URLs, or actions..."
          class="w-full bg-transparent text-sm text-[var(--text-primary)] placeholder:text-[var(--text-muted)] outline-none"
        />
        <kbd class="px-1.5 py-0.5 rounded text-[10px] font-mono bg-[var(--bg-canvas)] text-[var(--text-muted)] border border-[var(--border-subtle)]">
          ESC
        </kbd>
      </div>

      <!-- Results List -->
      <div class="max-h-96 overflow-y-auto vessel-scroll p-2 space-y-1">
        {#if filteredItems.length === 0}
          <div class="py-12 text-center text-xs text-[var(--text-muted)]">
            No matching containers or commands found.
          </div>
        {:else}
          {#each filteredItems as item, idx (item.id)}
            <button
              type="button"
              onclick={() => handleSelect(item)}
              onmouseenter={() => (selectedIndex = idx)}
              class="w-full flex items-center justify-between p-2.5 rounded-xl text-left transition-colors cursor-pointer {selectedIndex === idx
                ? 'bg-[var(--bg-surface-hover)] text-[var(--text-primary)]'
                : 'text-[var(--text-muted)] hover:bg-[var(--bg-surface-hover)]/60'}"
            >
              <div class="flex items-center gap-3 min-w-0">
                {#if item.type === 'action'}
                  <div
                    class="w-8 h-8 rounded-lg flex items-center justify-center shrink-0 bg-[var(--bg-canvas)] border border-[var(--border-subtle)] text-[var(--accent-primary)]"
                  >
                    {#if item.id === 'action-lock-all'}
                      <Icon name="lock" size={16} class="text-[var(--status-locked)]" />
                    {:else if item.id === 'action-hibernate-all'}
                      <Icon name="moon" size={16} class="text-[var(--accent-primary)]" />
                    {:else}
                      <Icon name="add" size={16} class="text-[var(--accent-primary)]" />
                    {/if}
                  </div>
                {:else}
                  <AppIcon
                    name={item.container?.name || item.title}
                    url={item.container?.url || ''}
                    icon={item.container?.icon}
                    size={32}
                    rounded="rounded-lg"
                  />
                {/if}

                <div class="min-w-0">
                  <div class="text-xs font-medium text-[var(--text-primary)] truncate">{item.title}</div>
                  <div class="text-[11px] text-[var(--text-muted)] font-mono truncate">{item.subtitle}</div>
                </div>
              </div>

              <div class="flex items-center gap-2 shrink-0 ml-3">
                {#if item.statusBadge}
                  <span
                    class="px-2 py-0.5 rounded text-[10px] font-medium {item.statusBadge.color}"
                  >
                    {item.statusBadge.label}
                  </span>
                {/if}
                {#if item.shortcut}
                  <kbd class="px-1.5 py-0.5 rounded text-[10px] font-mono bg-[var(--bg-canvas)] text-[var(--text-muted)] border border-[var(--border-subtle)]">
                    {item.shortcut}
                  </kbd>
                {/if}
              </div>
            </button>
          {/each}
        {/if}
      </div>

      <!-- Footer Info -->
      <div class="px-4 py-2 border-t border-[var(--border-subtle)] bg-[var(--bg-canvas)]/40 flex items-center justify-between text-[11px] text-[var(--text-muted)] font-mono">
        <div class="flex items-center gap-2">
          <span>↑↓ to navigate</span>
          <span>·</span>
          <span>↵ to select</span>
        </div>
        <div>{filteredItems.length} result{filteredItems.length === 1 ? '' : 's'}</div>
      </div>
    </div>
  </div>
{/if}
