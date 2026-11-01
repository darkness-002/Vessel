<script lang="ts">
  import type { ContainerConfig, ContainerProcessMetrics, CardDensity, EffectiveCardState } from '$lib/types';
  import Icon from '$lib/components/Icon.svelte';

  interface Props {
    container: ContainerConfig;
    metric?: ContainerProcessMetrics;
    isLocked?: boolean;
    density: CardDensity;
    globalIdleThresholdMinutes?: number;
    isLaunching?: boolean;
    isWaking?: boolean;
    isUnlocking?: boolean;
    isActionLoading?: boolean;
    onPrimaryAction: (container: ContainerConfig) => void;
    onHibernate: (container: ContainerConfig) => void;
    onLockVault: (container: ContainerConfig) => void;
    onOpenMenu: (container: ContainerConfig) => void;
  }

  let {
    container,
    metric,
    isLocked = false,
    density = 'minimal',
    globalIdleThresholdMinutes = 15,
    isLaunching = false,
    isWaking = false,
    isUnlocking = false,
    isActionLoading = false,
    onPrimaryAction,
    onHibernate,
    onLockVault,
    onOpenMenu,
  }: Props = $props();

  let imgFailed = $state(false);

  let effectiveState: EffectiveCardState = $derived.by(() => {
    if (isLocked) return 'locked';
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

  function formatIdleTime(seconds: number): string {
    if (seconds < 60) return `${seconds}s`;
    const mins = Math.floor(seconds / 60);
    if (mins < 60) return `${mins}m`;
    const hours = Math.floor(mins / 60);
    return `${hours}h ${mins % 60}m`;
  }

  // Idle countdown bar computation
  let idleThresholdMinutes = $derived(
    container.idleThresholdMinutes || globalIdleThresholdMinutes || 15
  );
  let idleThresholdSeconds = $derived(idleThresholdMinutes * 60);
  let idleSeconds = $derived(metric?.idleSeconds || 0);
  let idleProgressPercent = $derived(
    Math.min(100, Math.round((idleSeconds / idleThresholdSeconds) * 100))
  );
  let idleSecondsRemaining = $derived(Math.max(0, idleThresholdSeconds - idleSeconds));

  // Sparkline points generator
  let sparklinePoints = $derived.by(() => {
    const history = metric?.history;
    if (!history || history.length < 2) {
      // Subtle baseline points if no history recorded yet
      const base = metric?.memoryMb || 50;
      return `0,14 12,12 24,13 36,10 48,11 60,9`;
    }
    const values = history.map(h => h.memoryMb);
    const min = Math.min(...values) * 0.95;
    const max = Math.max(...values, min + 1) * 1.05;
    const width = 64;
    const height = 18;
    const step = width / (values.length - 1);

    return values
      .map((val, idx) => {
        const x = Math.round(idx * step);
        const y = Math.round(height - ((val - min) / (max - min)) * height);
        return `${x},${Math.max(2, Math.min(height - 2, y))}`;
      })
      .join(' ');
  });
</script>

<article
  class="flex flex-col justify-between p-4 rounded-xl border border-zinc-800 bg-[#15181e] hover:border-zinc-700 focus-within:border-zinc-600 transition-all shadow-sm select-none"
  aria-label="Container card for {container.name}"
>
  <!-- Card Header: Icon, Titles & Options Menu -->
  <div>
    <div class="flex items-start justify-between gap-3">
      <!-- Icon & Titles -->
      <div class="flex items-start gap-3 min-w-0">
        <!-- Cached Favicon or Monogram Tile -->
        <div
          class="w-10 h-10 rounded-lg flex items-center justify-center bg-zinc-900 border border-zinc-800 shrink-0 overflow-hidden"
        >
          {#if container.icon && !imgFailed}
            <img
              src={container.icon}
              alt=""
              class="w-6 h-6 object-contain"
              onerror={() => {
                imgFailed = true;
              }}
            />
          {:else}
            <span class="text-sm font-semibold text-zinc-300 uppercase">
              {container.name.slice(0, 1)}
            </span>
          {/if}
        </div>

        <!-- Name & Domain -->
        <div class="flex-1 min-w-0">
          <div class="flex items-center gap-1.5">
            <h3 class="text-sm font-semibold text-zinc-100 truncate" title={container.name}>
              {container.name}
            </h3>
            {#if container.profileLabel}
              <span
                class="px-1.5 py-0.2 rounded text-[10px] font-mono bg-zinc-800 text-zinc-300 border border-zinc-700/60 shrink-0"
              >
                {container.profileLabel}
              </span>
            {/if}
          </div>
          <span class="text-xs text-zinc-400 font-mono truncate block mt-0.5" title={container.url}>
            {getTargetDomain(container.url)}
          </span>
        </div>
      </div>

      <!-- Per-container Menu Button -->
      <button
        type="button"
        onclick={() => onOpenMenu(container)}
        class="inline-flex items-center justify-center min-w-[44px] min-h-[44px] -mr-2 -mt-2 rounded-lg text-zinc-400 hover:text-zinc-100 hover:bg-zinc-800/80 active:scale-95 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-zinc-400 transition-colors"
        aria-label="Options for {container.name}"
      >
        <Icon name="more_vert" size={18} />
      </button>
    </div>

    <!-- Status Badge (differs by label AND icon, not color alone!) -->
    <div class="mt-3 flex items-center gap-2">
      {#if effectiveState === 'running'}
        <div
          class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-md bg-emerald-500/10 border border-emerald-500/20 text-emerald-400 text-xs font-medium"
        >
          <span class="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse"></span>
          <Icon name="activity" size={13} class="text-emerald-400" />
          <span>Running</span>
        </div>
      {:else if effectiveState === 'idle'}
        <div
          class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-md bg-amber-500/10 border border-amber-500/20 text-amber-400 text-xs font-medium"
        >
          <span class="w-1.5 h-1.5 rounded-full bg-amber-400"></span>
          <Icon name="clock" size={13} class="text-amber-400" />
          <span>Idle {formatIdleTime(metric?.idleSeconds || 0)}</span>
        </div>
      {:else if effectiveState === 'hibernated'}
        <div
          class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-md bg-sky-500/10 border border-sky-500/20 text-sky-400 text-xs font-medium"
        >
          <span class="w-1.5 h-1.5 rounded-full bg-sky-400"></span>
          <Icon name="moon" size={13} class="text-sky-400" />
          <span>Hibernated</span>
        </div>
      {:else if effectiveState === 'locked'}
        <div
          class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-md bg-purple-500/10 border border-purple-500/20 text-purple-400 text-xs font-medium"
        >
          <span class="w-1.5 h-1.5 rounded-full bg-purple-400"></span>
          <Icon name="lock" size={13} class="text-purple-400" />
          <span>Vault sealed</span>
        </div>
      {:else}
        <div
          class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-md bg-zinc-800 border border-zinc-700/60 text-zinc-400 text-xs font-medium"
        >
          <span class="w-1.5 h-1.5 rounded-full bg-zinc-600"></span>
          <Icon name="power" size={13} class="text-zinc-400" />
          <span>Closed</span>
        </div>
      {/if}
    </div>

    <!-- Detailed Density Additional Telemetry -->
    {#if density === 'detailed'}
      <div class="mt-3 pt-3 border-t border-zinc-800/80 space-y-2.5">
        {#if effectiveState === 'running'}
          <!-- RAM & CPU with Mini Sparkline -->
          <div class="flex items-center justify-between gap-3 text-xs font-mono">
            <div class="space-y-0.5">
              <div class="text-zinc-200">
                RAM: <span class="font-semibold text-emerald-400">{metric?.memoryMb.toFixed(1)} MB</span>
              </div>
              <div class="text-zinc-400 text-[11px]">
                CPU: <span class="text-zinc-300">{metric?.cpuUsage.toFixed(1)}%</span>
              </div>
            </div>

            <!-- Mini Sparkline -->
            <div class="flex flex-col items-end">
              <svg
                width="64"
                height="18"
                viewBox="0 0 64 18"
                class="overflow-visible"
                aria-label="Process memory sparkline"
              >
                <polyline
                  fill="none"
                  stroke="#34d399"
                  stroke-width="1.75"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  points={sparklinePoints}
                />
              </svg>
              <span class="text-[9px] text-zinc-500 uppercase tracking-wider font-mono mt-0.5">RSS Activity</span>
            </div>
          </div>
        {:else if effectiveState === 'idle'}
          <!-- Idle Countdown Bar -->
          <div class="space-y-1.5">
            <div class="flex items-center justify-between text-[11px] font-mono">
              <span class="text-zinc-400">RAM: {metric?.memoryMb.toFixed(1)} MB</span>
              {#if container.neverHibernate}
                <span class="text-zinc-500 font-sans">Never hibernate active</span>
              {:else}
                <span class="text-amber-400">
                  {formatIdleTime(idleSecondsRemaining)} to hibernate
                </span>
              {/if}
            </div>

            {#if !container.neverHibernate}
              <!-- Countdown bar -->
              <div class="h-1.5 w-full rounded-full bg-zinc-800 overflow-hidden">
                <div
                  class="h-full bg-amber-400 rounded-full transition-all duration-300"
                  style="width: {idleProgressPercent}%"
                ></div>
              </div>
            {/if}
          </div>
        {:else if effectiveState === 'hibernated'}
          <!-- 0 MB Reclaimed Note -->
          <div class="flex items-center gap-2 p-2 rounded-md bg-sky-500/5 border border-sky-500/15 text-xs text-sky-400 font-mono">
            <Icon name="moon" size={13} class="text-sky-400 shrink-0" />
            <span>0 MB RAM · Process released to OS</span>
          </div>
        {:else if effectiveState === 'locked'}
          <!-- Encryption Note -->
          <div class="flex items-center gap-2 p-2 rounded-md bg-purple-500/5 border border-purple-500/15 text-xs text-purple-300 font-mono">
            <Icon name="shield" size={13} class="text-purple-400 shrink-0" />
            <span>AES-256-GCM · Keyring protected</span>
          </div>
        {:else}
          <!-- Closed Note -->
          <div class="text-xs text-zinc-500 font-mono">
            0 MB RAM · Ready to launch
          </div>
        {/if}
      </div>
    {/if}
  </div>

  <!-- Card Footer: Actions -->
  <div class="mt-4 pt-3 border-t border-zinc-800 flex flex-col gap-2">
    <!-- Primary Action (Always visible, min 44px hit target) -->
    <div>
      {#if effectiveState === 'locked'}
        <button
          type="button"
          onclick={() => onPrimaryAction(container)}
          disabled={isUnlocking}
          class="w-full flex items-center justify-center gap-2 min-h-[44px] px-3.5 py-2 rounded-lg bg-purple-600 hover:bg-purple-500 text-white text-xs font-semibold active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-purple-400 transition-colors shadow-sm disabled:opacity-50"
        >
          {#if isUnlocking}
            <Icon name="spinner" size={14} />
            <span>Unlocking...</span>
          {:else}
            <Icon name="lock_open" size={14} />
            <span>Unlock</span>
          {/if}
        </button>
      {:else if effectiveState === 'hibernated'}
        <button
          type="button"
          onclick={() => onPrimaryAction(container)}
          disabled={isWaking}
          class="w-full flex items-center justify-center gap-2 min-h-[44px] px-3.5 py-2 rounded-lg bg-sky-600 hover:bg-sky-500 text-white text-xs font-semibold active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-sky-400 transition-colors shadow-sm disabled:opacity-50"
        >
          {#if isWaking}
            <Icon name="spinner" size={14} />
            <span>Waking...</span>
          {:else}
            <Icon name="moon" size={14} />
            <span>Wake</span>
          {/if}
        </button>
      {:else if effectiveState === 'running' || effectiveState === 'idle'}
        <button
          type="button"
          onclick={() => onPrimaryAction(container)}
          disabled={isLaunching}
          class="w-full flex items-center justify-center gap-2 min-h-[44px] px-3.5 py-2 rounded-lg bg-zinc-800 hover:bg-zinc-700 text-zinc-100 text-xs font-semibold active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-zinc-400 transition-colors border border-zinc-700/80 disabled:opacity-50"
        >
          {#if isLaunching}
            <Icon name="spinner" size={14} />
            <span>Focusing...</span>
          {:else}
            <Icon name="focus" size={14} />
            <span>Focus window</span>
          {/if}
        </button>
      {:else}
        <button
          type="button"
          onclick={() => onPrimaryAction(container)}
          disabled={isLaunching}
          class="w-full flex items-center justify-center gap-2 min-h-[44px] px-3.5 py-2 rounded-lg bg-zinc-100 hover:bg-white text-zinc-950 text-xs font-semibold active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-zinc-400 transition-colors shadow-sm disabled:opacity-50"
        >
          {#if isLaunching}
            <Icon name="spinner" size={14} />
            <span>Launching...</span>
          {:else}
            <Icon name="play" size={14} />
            <span>Launch</span>
          {/if}
        </button>
      {/if}
    </div>

    <!-- Detailed Mode Secondary Icon Buttons (WITH labels!) -->
    {#if density === 'detailed'}
      <div class="flex items-center gap-2 pt-1">
        {#if effectiveState === 'running' || effectiveState === 'idle'}
          <!-- Secondary: Hibernate Button with label -->
          <button
            type="button"
            onclick={() => onHibernate(container)}
            disabled={isActionLoading}
            class="flex-1 flex items-center justify-center gap-1.5 min-h-[44px] px-2.5 py-2 rounded-lg bg-zinc-900 border border-zinc-800 hover:border-sky-500/40 text-xs font-medium text-sky-400 hover:text-sky-300 hover:bg-sky-500/10 active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-sky-400 transition-colors disabled:opacity-50"
          >
            <Icon name="moon" size={14} />
            <span>Hibernate</span>
          </button>
        {/if}

        {#if effectiveState !== 'locked'}
          <!-- Secondary: Seal Vault Button with label -->
          <button
            type="button"
            onclick={() => onLockVault(container)}
            disabled={isActionLoading}
            class="flex-1 flex items-center justify-center gap-1.5 min-h-[44px] px-2.5 py-2 rounded-lg bg-zinc-900 border border-zinc-800 hover:border-purple-500/40 text-xs font-medium text-purple-400 hover:text-purple-300 hover:bg-purple-500/10 active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-purple-400 transition-colors disabled:opacity-50"
          >
            <Icon name="lock" size={14} />
            <span>Seal vault</span>
          </button>
        {/if}
      </div>
    {/if}
  </div>
</article>
