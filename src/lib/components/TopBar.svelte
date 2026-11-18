<script lang="ts">
  import VesselLogo from '$lib/components/VesselLogo.svelte';

  interface Props {
    activeTab?: 'apps' | 'manage';
    onSelectTab?: (tab: 'apps' | 'manage') => void;
    onOpenQuickSwitcher?: () => void;
    onOpenDocumentation?: () => void;
    onOpenSettings?: () => void;
  }

  let {
    activeTab = 'apps',
    onSelectTab,
    onOpenQuickSwitcher,
    onOpenDocumentation,
    onOpenSettings,
  }: Props = $props();
</script>

<header
  data-tauri-drag-region
  class="h-14 border-b border-[var(--border-subtle)] bg-[var(--bg-surface)] shrink-0 z-20 select-none"
  aria-label="Top navigation"
>
  <div class="w-full px-6 md:px-10 xl:px-14 h-full flex items-center justify-between">
    <!-- Left: Brand / Logo + Navigation Tabs -->
    <div class="flex items-center gap-8 h-full">
      <!-- Brand -->
      <div class="flex items-center gap-2.5">
        <VesselLogo size={22} />
        <span class="font-semibold text-base tracking-tight text-[var(--text-primary)]">
          Vessel
        </span>
      </div>

      <!-- Center-left Navigation Tabs -->
      <nav class="flex items-center gap-6 h-full" aria-label="Main tabs">
        <button
          type="button"
          onclick={() => onSelectTab?.('apps')}
          class="h-full flex items-center text-xs transition-colors relative border-b-2 focus-visible:outline-none cursor-pointer {activeTab === 'apps'
            ? 'border-[var(--accent-primary)] text-[var(--text-primary)] font-semibold'
            : 'border-transparent text-[var(--text-muted)] hover:text-[var(--text-primary)] font-normal'}"
        >
          Your apps
        </button>

        <button
          type="button"
          onclick={() => onSelectTab?.('manage')}
          class="h-full flex items-center text-xs transition-colors relative border-b-2 focus-visible:outline-none cursor-pointer {activeTab === 'manage'
            ? 'border-[var(--accent-primary)] text-[var(--text-primary)] font-semibold'
            : 'border-transparent text-[var(--text-muted)] hover:text-[var(--text-primary)] font-normal'}"
        >
          Manage containers
        </button>
      </nav>
    </div>

    <!-- Right Utilities: Quick Switcher, Documentation & Settings -->
    <div class="flex items-center gap-3 sm:gap-4">
      <!-- Quick Switcher Trigger (Ctrl+K) -->
      <button
        type="button"
        onclick={() => onOpenQuickSwitcher?.()}
        class="text-xs text-[var(--text-muted)] hover:text-[var(--text-primary)] transition-colors flex items-center gap-2 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-[var(--accent-primary)] rounded-lg px-2.5 py-1.5 bg-[var(--bg-canvas)] border border-[var(--border-subtle)] hover:bg-[var(--bg-surface-hover)] cursor-pointer shadow-sm"
        title="Open Quick Switcher (Ctrl+K)"
      >
        <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="11" cy="11" r="8"/>
          <path d="m21 21-4.3-4.3"/>
        </svg>
        <span class="hidden sm:inline">Quick Switcher</span>
        <kbd class="px-1.5 py-0.5 rounded bg-[var(--bg-surface)] border border-[var(--border-subtle)] font-mono text-[10px] text-[var(--text-muted)]">
          Ctrl+K
        </kbd>
      </button>

      <!-- Documentation & Benchmarks Link -->
      <button
        type="button"
        onclick={() => onOpenDocumentation?.()}
        class="text-xs text-[var(--text-muted)] hover:text-[var(--text-primary)] transition-colors flex items-center gap-1 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-[var(--accent-primary)] rounded px-1.5 py-1 cursor-pointer"
        title="Open Documentation & Benchmarks"
      >
        <span>Documentation & Benchmarks</span>
        <span class="text-[11px] leading-none">↗</span>
      </button>

      <!-- Settings Button -->
      <button
        type="button"
        onclick={() => onOpenSettings?.()}
        class="text-xs text-[var(--text-muted)] hover:text-[var(--text-primary)] transition-colors flex items-center gap-1.5 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-[var(--accent-primary)] rounded px-1.5 py-1 cursor-pointer"
      >
        <span>Settings</span>
      </button>
    </div>
  </div>
</header>
