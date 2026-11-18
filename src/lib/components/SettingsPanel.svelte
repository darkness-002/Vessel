<script lang="ts">
  import type { CardDensity, ThemeMode, DashboardViewMode } from '$lib/types';
  import Icon from '$lib/components/Icon.svelte';
  import { openUrl } from '@tauri-apps/plugin-opener';

  interface Props {
    isOpen: boolean;
    theme?: ThemeMode;
    viewMode?: DashboardViewMode;
    density?: CardDensity;
    isDemoMode: boolean;
    globalIdleThresholdMinutes: number;
    muteAllNotifications?: boolean;
    onUpdateTheme?: (theme: ThemeMode) => void;
    onUpdateViewMode?: (viewMode: DashboardViewMode) => void;
    onUpdateDensity?: (density: CardDensity) => void;
    onToggleDemoMode: (enabled: boolean) => void;
    onUpdateIdleThreshold: (minutes: number) => void;
    onToggleMuteAll?: (muted: boolean) => void;
    onClose: () => void;
  }

  let {
    isOpen = false,
    theme = 'monochrome',
    viewMode = 'simple',
    density = 'minimal',
    isDemoMode = false,
    globalIdleThresholdMinutes = 15,
    muteAllNotifications = false,
    onUpdateTheme,
    onUpdateViewMode,
    onUpdateDensity,
    onToggleDemoMode,
    onUpdateIdleThreshold,
    onToggleMuteAll,
    onClose,
  }: Props = $props();

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && isOpen) {
      onClose();
    }
  }

  function selectView(mode: DashboardViewMode) {
    onUpdateViewMode?.(mode);
    onUpdateDensity?.(mode === 'detailed' ? 'detailed' : 'minimal');
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

  const THEMES: { id: ThemeMode; name: string; type: string; bg: string; surface: string; accent: string }[] = [
    {
      id: 'monochrome',
      name: 'Monochrome',
      type: 'Default Dark',
      bg: '#0A0A0A',
      surface: '#141414',
      accent: '#FFFFFF',
    },
    {
      id: 'paper',
      name: 'Paper',
      type: 'Tactile Light',
      bg: '#F6F4EE',
      surface: '#FFFFFF',
      accent: '#1C1A17',
    },
  ];
</script>

<svelte:window onkeydown={handleKeydown} />

{#if isOpen}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/80 animate-in fade-in"
    onclick={(e) => {
      if (e.target === e.currentTarget) onClose();
    }}
    role="presentation"
  >
    <div
      class="bg-[var(--bg-surface)] border border-[var(--border-subtle)] rounded-2xl w-full max-w-lg shadow-2xl overflow-hidden flex flex-col animate-in zoom-in-95 select-none"
      role="dialog"
      aria-modal="true"
      aria-labelledby="settings-panel-title"
    >
      <!-- Header -->
      <div class="px-6 py-4 border-b border-[var(--border-subtle)] flex items-center justify-between">
        <div class="flex items-center gap-2.5">
          <div
            class="w-8 h-8 rounded-lg bg-[var(--bg-canvas)] border border-[var(--border-subtle)] flex items-center justify-center text-[var(--accent-primary)]"
          >
            <Icon name="settings" size={16} />
          </div>
          <div>
            <h2 id="settings-panel-title" class="text-sm font-semibold text-[var(--text-primary)]">
              Dashboard Settings
            </h2>
            <p class="text-[11px] text-[var(--text-muted)] mt-0.5">Themes, layout density & system preferences</p>
          </div>
        </div>

        <button
          type="button"
          onclick={onClose}
          class="inline-flex items-center justify-center min-w-[36px] min-h-[36px] rounded-lg text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-surface-hover)] focus-visible:outline-none transition-colors"
          aria-label="Close settings"
        >
          <Icon name="close" size={16} />
        </button>
      </div>

      <!-- Settings Content -->
      <div class="p-6 space-y-6 max-h-[75vh] overflow-y-auto vessel-scroll">
        <!-- Appearance / Theme Selector -->
        <div class="space-y-3">
          <div>
            <span class="text-xs font-semibold text-[var(--text-primary)] uppercase tracking-wider font-mono">
              Appearance & Theme
            </span>
            <p class="text-xs text-[var(--text-muted)] mt-0.5 leading-relaxed">
              Select your color palette with instant live preview.
            </p>
          </div>

          <div class="grid grid-cols-2 gap-3.5">
            {#each THEMES as t}
              <button
                type="button"
                onclick={() => onUpdateTheme?.(t.id)}
                class="flex flex-col items-start p-3.5 rounded-xl border text-left transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--accent-primary)] relative {theme === t.id
                  ? 'border-[var(--accent-primary)] bg-[var(--bg-surface-hover)] shadow-sm'
                  : 'border-[var(--border-subtle)] bg-[var(--bg-canvas)]/40 hover:border-[var(--border-subtle)]/90 hover:bg-[var(--bg-surface-hover)]/40'}"
              >
                <!-- Color preview dots -->
                <div class="flex items-center gap-1.5 mb-2.5">
                  <span class="w-3.5 h-3.5 rounded-full border border-black/10 shadow-sm" style="background-color: {t.bg};"></span>
                  <span class="w-3.5 h-3.5 rounded-full border border-black/10 shadow-sm" style="background-color: {t.surface};"></span>
                  <span class="w-3.5 h-3.5 rounded-full border border-black/10 shadow-sm" style="background-color: {t.accent};"></span>
                </div>

                <span class="text-xs font-semibold text-[var(--text-primary)] block leading-tight">
                  {t.name}
                </span>
                <span class="text-[10px] text-[var(--text-muted)] block mt-0.5">
                  {t.type}
                </span>

                {#if theme === t.id}
                  <span class="absolute top-2.5 right-2.5 w-2 h-2 rounded-full bg-[var(--accent-primary)]"></span>
                {/if}
              </button>
            {/each}
          </div>
        </div>

        <!-- Dashboard View Mode -->
        <div class="space-y-3 pt-3 border-t border-[var(--border-subtle)]">
          <div>
            <span class="text-xs font-semibold text-[var(--text-primary)] uppercase tracking-wider font-mono">
              Dashboard View Mode
            </span>
            <p class="text-xs text-[var(--text-muted)] mt-0.5 leading-relaxed">
              Switch between card grid and live resource table.
            </p>
          </div>

          <div class="grid grid-cols-2 gap-3">
            <!-- Simple View (Default) -->
            <button
              type="button"
              onclick={() => selectView('simple')}
              class="flex flex-col items-start p-3.5 rounded-xl border text-left transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--accent-primary)] {viewMode === 'simple'
                ? 'border-[var(--accent-primary)] bg-[var(--bg-surface-hover)] text-[var(--text-primary)]'
                : 'border-[var(--border-subtle)] bg-[var(--bg-canvas)]/40 text-[var(--text-muted)] hover:border-[var(--border-subtle)]/90 hover:text-[var(--text-primary)]'}"
            >
              <div class="flex items-center justify-between w-full">
                <span class="text-xs font-semibold text-[var(--text-primary)]">Simple</span>
                {#if viewMode === 'simple'}
                  <span class="text-[10px] font-mono px-1.5 py-0.5 rounded bg-[var(--accent-primary)] text-[var(--accent-fg)] font-semibold">
                    Default
                  </span>
                {/if}
              </div>
              <p class="text-[11px] text-[var(--text-muted)] mt-1.5 leading-snug">
                3-column responsive card grid. App icons, live status indicator, and quick actions.
              </p>
            </button>

            <!-- Detailed View -->
            <button
              type="button"
              onclick={() => selectView('detailed')}
              class="flex flex-col items-start p-3.5 rounded-xl border text-left transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--accent-primary)] {viewMode === 'detailed'
                ? 'border-[var(--accent-primary)] bg-[var(--bg-surface-hover)] text-[var(--text-primary)]'
                : 'border-[var(--border-subtle)] bg-[var(--bg-canvas)]/40 text-[var(--text-muted)] hover:border-[var(--border-subtle)]/90 hover:text-[var(--text-primary)]'}"
            >
              <div class="flex items-center justify-between w-full">
                <span class="text-xs font-semibold text-[var(--text-primary)]">Detailed</span>
              </div>
              <p class="text-[11px] text-[var(--text-muted)] mt-1.5 leading-snug">
                2-column RAM/CPU telemetry KPI cards plus 5-column process resource data table.
              </p>
            </button>
          </div>
        </div>

        <!-- Mute All Notifications Toggle -->
        <div class="pt-3 border-t border-[var(--border-subtle)]">
          <label class="flex items-center justify-between p-3 rounded-xl bg-[var(--bg-canvas)]/50 border border-[var(--border-subtle)] cursor-pointer hover:bg-[var(--bg-surface-hover)] transition-colors">
            <div class="space-y-0.5 pr-4">
              <span class="text-xs font-semibold text-[var(--text-primary)] block">Mute All Notifications</span>
              <span class="text-[11px] text-[var(--text-muted)] block leading-relaxed">
                Globally suppresses container notifications without interrupting background sessions.
              </span>
            </div>
            <input
              type="checkbox"
              checked={muteAllNotifications}
              onchange={(e) => onToggleMuteAll?.(e.currentTarget.checked)}
              class="w-4 h-4 rounded border-[var(--border-subtle)] bg-[var(--bg-canvas)] text-[var(--accent-primary)] focus:ring-[var(--accent-primary)]"
            />
          </label>
        </div>

        <!-- Demo Mode Fixture Toggle -->
        <div class="pt-3 border-t border-[var(--border-subtle)]">
          <label class="flex items-center justify-between p-3 rounded-xl bg-[var(--bg-canvas)]/50 border border-[var(--border-subtle)] cursor-pointer hover:bg-[var(--bg-surface-hover)] transition-colors">
            <div class="space-y-0.5 pr-4">
              <span class="text-xs font-semibold text-[var(--text-primary)] block">Demo Mode Fixture</span>
              <span class="text-[11px] text-[var(--text-muted)] block leading-relaxed">
                Loads a simulated test dataset demonstrating Spotify, Notes, and Mail container states.
              </span>
            </div>
            <input
              type="checkbox"
              checked={isDemoMode}
              onchange={(e) => onToggleDemoMode(e.currentTarget.checked)}
              class="w-4 h-4 rounded border-[var(--border-subtle)] bg-[var(--bg-canvas)] text-[var(--accent-primary)] focus:ring-[var(--accent-primary)]"
            />
          </label>
        </div>

        <!-- Default Inactivity Hibernation -->
        <div class="pt-3 border-t border-[var(--border-subtle)] space-y-2">
          <label for="global-idle-timeout" class="block text-xs font-semibold text-[var(--text-primary)] uppercase tracking-wider font-mono">
            Global Hibernation Threshold
          </label>
          <p class="text-xs text-[var(--text-muted)] leading-relaxed">
            Inactivity duration before unfocused container windows are gracefully hibernated back to the host operating system.
          </p>
          <select
            id="global-idle-timeout"
            value={globalIdleThresholdMinutes}
            onchange={(e) => onUpdateIdleThreshold(Number(e.currentTarget.value))}
            class="w-full px-3 py-2 text-xs bg-[var(--bg-canvas)] border border-[var(--border-subtle)] rounded-xl text-[var(--text-primary)] focus:border-[var(--accent-primary)] focus:ring-1 focus:ring-[var(--accent-primary)] outline-none transition-colors"
          >
            <option value={5}>5 minutes</option>
            <option value={15}>15 minutes (Default & Recommended)</option>
            <option value={30}>30 minutes</option>
            <option value={60}>60 minutes</option>
          </select>
        </div>

        <!-- Documentation & Benchmarks Link Row -->
        <div class="pt-3 border-t border-[var(--border-subtle)] flex items-center justify-between">
          <div>
            <span class="text-xs font-semibold text-[var(--text-primary)] uppercase tracking-wider font-mono">
              Documentation & Benchmarks
            </span>
            <p class="text-xs text-[var(--text-muted)] mt-0.5 leading-relaxed">
              Read architecture specifications, isolation boundaries, and empirical host benchmarks.
            </p>
          </div>
          <button
            type="button"
            onclick={handleOpenDocs}
            class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-xl border border-[var(--border-subtle)] bg-[var(--bg-canvas)] hover:bg-[var(--bg-surface-hover)] text-xs font-medium text-[var(--text-primary)] transition-colors cursor-pointer"
          >
            <span>Open Docs</span>
            <span class="text-[11px] leading-none">↗</span>
          </button>
        </div>
      </div>

      <!-- Footer -->
      <div class="px-6 py-4 bg-[var(--bg-canvas)]/40 border-t border-[var(--border-subtle)] flex items-center justify-end">
        <button
          type="button"
          onclick={onClose}
          class="min-h-[38px] px-5 rounded-xl bg-[var(--accent-primary)] hover:opacity-95 text-[var(--accent-fg)] text-xs font-semibold active:scale-[0.98] transition-colors shadow-sm"
        >
          Done
        </button>
      </div>
    </div>
  </div>
{/if}
