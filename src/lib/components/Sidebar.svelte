<script lang="ts">
  import type { FilterCategory, CardDensity } from '$lib/types';
  import VesselLogo from '$lib/components/VesselLogo.svelte';
  import Icon from '$lib/components/Icon.svelte';

  interface Props {
    activeFilter: FilterCategory;
    counts: {
      all: number;
      active: number;
      hibernated: number;
      locked: number;
    };
    density: CardDensity;
    isDemoMode: boolean;
    onSelectFilter: (filter: FilterCategory) => void;
    onOpenSettings: () => void;
    onToggleDensity: () => void;
  }

  let {
    activeFilter,
    counts,
    density,
    isDemoMode,
    onSelectFilter,
    onOpenSettings,
    onToggleDensity,
  }: Props = $props();

  const navItems: { id: FilterCategory; label: string; icon: 'grid' | 'activity' | 'moon' | 'lock' }[] = [
    { id: 'all', label: 'All', icon: 'grid' },
    { id: 'active', label: 'Active', icon: 'activity' },
    { id: 'hibernated', label: 'Hibernated', icon: 'moon' },
    { id: 'locked', label: 'Vault sealed', icon: 'lock' },
  ];
</script>

<aside
  class="w-56 shrink-0 h-full flex flex-col justify-between border-r border-zinc-800 bg-[#0c0e12] select-none z-20"
  aria-label="Navigation sidebar"
>
  <!-- Top: Logo & Main Navigation -->
  <div class="flex flex-col p-4 gap-6">
    <!-- Brand -->
    <div class="flex items-center gap-2.5 px-2 pt-1">
      <VesselLogo size={24} />
      <div class="flex flex-col">
        <span class="text-sm font-semibold tracking-tight text-zinc-100">Vessel</span>
        <span class="text-[10px] text-zinc-500 font-mono leading-none">Container Engine</span>
      </div>
    </div>

    <!-- Filter Navigation List -->
    <nav class="flex flex-col gap-1" aria-label="Container filters">
      {#each navItems as item}
        {@const isCurrent = activeFilter === item.id}
        {@const count = counts[item.id]}
        <button
          type="button"
          onclick={() => onSelectFilter(item.id)}
          class="flex items-center justify-between w-full min-h-[44px] px-3 rounded-lg text-xs font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-zinc-400 {isCurrent
            ? 'bg-zinc-800/90 text-zinc-100'
            : 'text-zinc-400 hover:text-zinc-200 hover:bg-zinc-900/60'}"
          aria-current={isCurrent ? 'page' : undefined}
        >
          <div class="flex items-center gap-2.5 min-w-0">
            <Icon
              name={item.icon}
              size={16}
              class={isCurrent
                ? item.id === 'active'
                  ? 'text-emerald-400'
                  : item.id === 'hibernated'
                    ? 'text-sky-400'
                    : item.id === 'locked'
                      ? 'text-purple-400'
                      : 'text-zinc-200'
                : 'text-zinc-500'}
            />
            <span class="truncate">{item.label}</span>
          </div>

          <span
            class="text-[11px] font-mono px-2 py-0.5 rounded-full {isCurrent
              ? 'bg-zinc-700/60 text-zinc-200'
              : 'text-zinc-500 bg-zinc-900/80'}"
          >
            {count}
          </span>
        </button>
      {/each}
    </nav>
  </div>

  <!-- Bottom: Density Switcher, Demo Badge & Settings -->
  <div class="p-4 border-t border-zinc-800/80 flex flex-col gap-3">
    <!-- Density Quick Toggle -->
    <div class="flex items-center justify-between px-1">
      <span class="text-[11px] font-medium text-zinc-400">Density</span>
      <button
        type="button"
        onclick={onToggleDensity}
        class="inline-flex items-center gap-1.5 px-2.5 py-1 min-h-[32px] rounded-md bg-zinc-900 border border-zinc-700/80 text-[11px] font-medium text-zinc-300 hover:text-zinc-100 hover:border-zinc-600 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-zinc-400 transition-colors"
        title="Switch between minimal and detailed view"
      >
        <span class="capitalize">{density}</span>
      </button>
    </div>

    <!-- Demo Mode indicator -->
    {#if isDemoMode}
      <div
        class="flex items-center gap-1.5 px-2.5 py-1.5 rounded-md bg-amber-500/10 border border-amber-500/20 text-amber-400 text-[11px] font-medium"
      >
        <span class="w-1.5 h-1.5 rounded-full bg-amber-400 animate-pulse"></span>
        <span>Demo Fixture Active</span>
      </div>
    {/if}

    <!-- Settings Button -->
    <button
      type="button"
      onclick={onOpenSettings}
      class="flex items-center justify-between w-full min-h-[44px] px-3 rounded-lg text-xs font-medium text-zinc-300 hover:text-zinc-100 hover:bg-zinc-900/90 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-zinc-400 transition-colors"
    >
      <div class="flex items-center gap-2.5">
        <Icon name="settings" size={16} class="text-zinc-400" />
        <span>Settings</span>
      </div>
      <span class="text-[10px] text-zinc-500 font-mono">v0.1.0</span>
    </button>
  </div>
</aside>
