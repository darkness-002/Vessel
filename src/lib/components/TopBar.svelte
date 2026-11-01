<script lang="ts">
  import type { CardDensity } from '$lib/types';
  import Icon from '$lib/components/Icon.svelte';

  interface Props {
    searchQuery: string;
    searchInputRef: HTMLInputElement | null;
    density: CardDensity;
    savedMb: number;
    onNewContainer: () => void;
  }

  let {
    searchQuery = $bindable(),
    searchInputRef = $bindable(),
    density,
    savedMb = 0,
    onNewContainer,
  }: Props = $props();

  function formatMemory(mb: number): string {
    if (mb >= 1024) {
      return `${(mb / 1024).toFixed(2)} GB`;
    }
    return `${mb.toFixed(0)} MB`;
  }
</script>

<header
  class="h-16 border-b border-zinc-800 bg-[#0c0e12]/95 backdrop-blur-md px-6 flex items-center justify-between shrink-0 z-10 select-none gap-4"
  aria-label="Top header"
>
  <!-- Search Filter Input -->
  <div class="relative w-72 sm:w-80">
    <div class="absolute left-3 top-1/2 -translate-y-1/2 pointer-events-none text-zinc-400">
      <Icon name="search" size={16} />
    </div>

    <input
      bind:this={searchInputRef}
      type="text"
      bind:value={searchQuery}
      placeholder="Filter containers..."
      class="w-full bg-zinc-900 border border-zinc-700/80 rounded-lg pl-9 pr-8 py-2 text-xs text-zinc-100 placeholder:text-zinc-500 focus:border-zinc-400 focus:ring-1 focus:ring-zinc-400 outline-none transition-colors min-h-[44px]"
      aria-label="Filter containers"
    />

    {#if searchQuery}
      <button
        type="button"
        onclick={() => (searchQuery = '')}
        class="absolute right-1 top-1/2 -translate-y-1/2 inline-flex items-center justify-center min-w-[36px] min-h-[36px] text-zinc-400 hover:text-zinc-100 rounded focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-zinc-400 transition-colors"
        aria-label="Clear filter"
      >
        <Icon name="close" size={14} />
      </button>
    {:else}
      <kbd
        class="absolute right-3 top-1/2 -translate-y-1/2 text-[10px] text-zinc-400 border border-zinc-700/80 rounded px-1.5 py-0.5 font-mono pointer-events-none bg-zinc-800"
      >
        /
      </kbd>
    {/if}
  </div>

  <!-- Right Actions -->
  <div class="flex items-center gap-3">
    <!-- In minimal mode: single small "memory saved" figure -->
    {#if density === 'minimal' && savedMb > 0}
      <div
        class="hidden sm:inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-emerald-500/10 border border-emerald-500/20 text-xs font-mono text-emerald-400"
        title="Active memory saved relative to equivalent Electron instances"
      >
        <span class="w-1.5 h-1.5 rounded-full bg-emerald-400"></span>
        <span>+{formatMemory(savedMb)} saved</span>
      </div>
    {/if}

    <!-- "New container" button -->
    <button
      type="button"
      onclick={onNewContainer}
      class="inline-flex items-center gap-2 min-h-[44px] px-4 rounded-lg bg-zinc-100 hover:bg-white text-zinc-950 font-semibold text-xs active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-zinc-400 transition-colors shadow-sm"
    >
      <Icon name="add" size={16} />
      <span>New Container</span>
    </button>
  </div>
</header>
