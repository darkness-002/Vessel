<script lang="ts">
  import type { ContainerConfig, ContainerProcessMetrics, CardDensity, EffectiveCardState } from '$lib/types';
  import Icon from '$lib/components/Icon.svelte';
  import AppIcon from '$lib/components/AppIcon.svelte';

  interface Props {
    container: ContainerConfig;
    metric?: ContainerProcessMetrics;
    isLocked?: boolean;
    density?: CardDensity;
    isLaunching?: boolean;
    isWaking?: boolean;
    isUnlocking?: boolean;
    isActionLoading?: boolean;
    onPrimaryAction: (container: ContainerConfig) => void;
    onDetails?: (container: ContainerConfig) => void;
    onOpenMenu: (container: ContainerConfig) => void;
    onHibernate?: (container: ContainerConfig) => void;
    onLockVault?: (container: ContainerConfig) => void;
  }

  let {
    container,
    metric,
    isLocked = false,
    density = 'minimal',
    isLaunching = false,
    isWaking = false,
    isUnlocking = false,
    isActionLoading = false,
    onPrimaryAction,
    onDetails,
    onOpenMenu,
  }: Props = $props();

  let effectiveState: EffectiveCardState = $derived.by(() => {
    if (isLocked) return 'locked';
    if (metric?.status === 'crashed') return 'crashed';
    if (metric?.status === 'running') return 'running';
    if (metric?.status === 'idle') return 'idle';
    if (metric?.status === 'hibernated') return 'hibernated';
    return 'closed';
  });

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
</script>

<article
  class="flex flex-col justify-between p-5 rounded-2xl border border-[var(--border-subtle)] bg-[var(--bg-surface)] hover:border-[var(--border-subtle)]/80 transition-colors select-none shadow-sm"
  aria-label="Container card for {container.name}"
>
  <!-- Top Row: Icon, Titles & Overflow Menu -->
  <div>
    <div class="flex items-start justify-between gap-3">
      <!-- Icon & Titles -->
      <div class="flex items-start gap-3 min-w-0 flex-1">
        <!-- Container Icon Tile -->
        <AppIcon
          name={container.name}
          url={container.url}
          icon={container.icon}
          size={40}
          rounded="rounded-xl"
        />

        <!-- Name & Domain -->
        <div class="flex-1 min-w-0">
          <div class="flex items-center gap-1.5">
            <h3 class="text-base font-semibold text-[var(--text-primary)] truncate" title={container.name}>
              {container.name}
            </h3>
            {#if container.profileLabel}
              <span
                class="px-1.5 py-0.5 rounded text-[10px] font-mono bg-[var(--bg-canvas)] text-[var(--text-muted)] border border-[var(--border-subtle)] shrink-0"
              >
                {container.profileLabel}
              </span>
            {/if}
          </div>
          <span class="text-xs text-[var(--text-muted)] truncate block mt-0.5 font-normal" title={container.url}>
            {getTargetDomain(container.url)}
          </span>
        </div>
      </div>

      <!-- Overflow Menu Button -->
      <button
        type="button"
        onclick={() => onOpenMenu(container)}
        class="w-8 h-8 rounded-lg flex items-center justify-center text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-surface-hover)] focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-[var(--accent-primary)] transition-colors shrink-0"
        aria-label="Options for {container.name}"
      >
        <svg class="w-4 h-4" viewBox="0 0 24 24" fill="currentColor">
          <circle cx="12" cy="5" r="2"/>
          <circle cx="12" cy="12" r="2"/>
          <circle cx="12" cy="19" r="2"/>
        </svg>
      </button>
    </div>

    <!-- Status Row -->
    <div class="mt-4 mb-5">
      {#if effectiveState === 'running'}
        <div class="flex items-center gap-2">
          <span class="w-2 h-2 rounded-full bg-[var(--status-running)]"></span>
          <span class="text-xs font-medium text-[var(--status-running)]">Running</span>
        </div>
        <p class="text-xs text-[var(--text-muted)] mt-1.5">Ready when you are.</p>
      {:else if effectiveState === 'locked'}
        <div class="flex items-center gap-1.5">
          <svg class="w-3.5 h-3.5 text-[var(--status-locked)]" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <rect x="3" y="11" width="18" height="11" rx="2" ry="2"/>
            <path d="M7 11V7a5 5 0 0 1 10 0v4"/>
          </svg>
          <span class="text-xs font-medium text-[var(--status-locked)]">Locked</span>
        </div>
        <p class="text-xs text-[var(--text-muted)] mt-1.5">Unlock to open your app.</p>
      {:else if effectiveState === 'idle'}
        <div class="flex items-center gap-2">
          <span class="w-2 h-2 rounded-full bg-[var(--accent-primary)] opacity-80"></span>
          <span class="text-xs font-medium text-[var(--accent-primary)]">Idle</span>
        </div>
        <p class="text-xs text-[var(--text-muted)] mt-1.5">Ready when you are.</p>
      {:else if effectiveState === 'hibernated'}
        <div class="flex items-center gap-2">
          <span class="w-2 h-2 rounded-full bg-[var(--text-muted)]"></span>
          <span class="text-xs font-medium text-[var(--text-muted)]">Hibernated</span>
        </div>
        <p class="text-xs text-[var(--text-muted)] mt-1.5">Paused to save RAM.</p>
      {:else}
        <div class="flex items-center gap-2">
          <span class="w-2 h-2 rounded-full bg-[var(--text-muted)]/60"></span>
          <span class="text-xs font-medium text-[var(--text-muted)]">Closed</span>
        </div>
        <p class="text-xs text-[var(--text-muted)] mt-1.5">Click Open to launch.</p>
      {/if}
    </div>
  </div>

  <!-- Bottom Action Row: Primary Action + Details Button -->
  <div class="flex items-center gap-2.5 pt-1">
    {#if effectiveState === 'locked'}
      <button
        type="button"
        onclick={() => onPrimaryAction(container)}
        disabled={isUnlocking}
        class="flex-1 min-h-[40px] px-4 rounded-xl bg-[var(--accent-primary)] hover:opacity-95 text-[var(--accent-fg)] font-semibold text-xs flex items-center justify-center transition-colors active:scale-[0.98] disabled:opacity-60 shadow-sm"
      >
        {#if isUnlocking}
          <Icon name="spinner" size={14} class="mr-1.5" />
          <span>Unlocking...</span>
        {:else}
          <span>Unlock</span>
        {/if}
      </button>
    {:else}
      <button
        type="button"
        onclick={() => onPrimaryAction(container)}
        disabled={isLaunching || isWaking}
        class="flex-1 min-h-[40px] px-4 rounded-xl bg-[var(--accent-primary)] hover:opacity-95 text-[var(--accent-fg)] font-semibold text-xs flex items-center justify-center transition-colors active:scale-[0.98] disabled:opacity-60 shadow-sm"
      >
        {#if isLaunching}
          <Icon name="spinner" size={14} class="mr-1.5" />
          <span>Opening...</span>
        {:else if isWaking}
          <Icon name="spinner" size={14} class="mr-1.5" />
          <span>Waking...</span>
        {:else}
          <span>Open app</span>
        {/if}
      </button>
    {/if}

    <button
      type="button"
      onclick={() => (onDetails ? onDetails(container) : onOpenMenu(container))}
      class="min-h-[40px] px-4 rounded-xl border border-[var(--border-subtle)] bg-[var(--bg-canvas)] hover:bg-[var(--bg-surface-hover)] text-[var(--text-primary)] font-medium text-xs flex items-center justify-center transition-colors active:scale-[0.98]"
    >
      Details
    </button>
  </div>
</article>
