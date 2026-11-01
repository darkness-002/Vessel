<script lang="ts">
  import type { CardDensity } from '$lib/types';
  import Icon from '$lib/components/Icon.svelte';

  interface Props {
    isOpen: boolean;
    density: CardDensity;
    isDemoMode: boolean;
    globalIdleThresholdMinutes: number;
    onUpdateDensity: (density: CardDensity) => void;
    onToggleDemoMode: (enabled: boolean) => void;
    onUpdateIdleThreshold: (minutes: number) => void;
    onClose: () => void;
  }

  let {
    isOpen = false,
    density = 'minimal',
    isDemoMode = false,
    globalIdleThresholdMinutes = 15,
    onUpdateDensity,
    onToggleDemoMode,
    onUpdateIdleThreshold,
    onClose,
  }: Props = $props();

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && isOpen) {
      onClose();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if isOpen}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-sm animate-in fade-in"
    onclick={(e) => {
      if (e.target === e.currentTarget) onClose();
    }}
    role="presentation"
  >
    <div
      class="bg-[#15181e] border border-zinc-800 rounded-xl w-full max-w-lg shadow-2xl overflow-hidden flex flex-col animate-in zoom-in-95 select-none"
      role="dialog"
      aria-modal="true"
      aria-labelledby="settings-panel-title"
    >
      <!-- Header -->
      <div class="px-6 py-4 border-b border-zinc-800 flex items-center justify-between">
        <div class="flex items-center gap-2.5">
          <div
            class="w-8 h-8 rounded-lg bg-zinc-800 border border-zinc-700/60 flex items-center justify-center text-zinc-300"
          >
            <Icon name="settings" size={16} />
          </div>
          <div>
            <h2 id="settings-panel-title" class="text-sm font-semibold text-zinc-100">
              Dashboard Settings
            </h2>
            <p class="text-[11px] text-zinc-400 font-mono mt-0.5">Preferences & Display Density</p>
          </div>
        </div>

        <button
          type="button"
          onclick={onClose}
          class="inline-flex items-center justify-center min-w-[44px] min-h-[44px] rounded-lg text-zinc-400 hover:text-zinc-100 hover:bg-zinc-800 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-zinc-400 transition-colors"
          aria-label="Close settings"
        >
          <Icon name="close" size={18} />
        </button>
      </div>

      <!-- Settings Content -->
      <div class="p-6 space-y-6">
        <!-- Display Density Preference -->
        <div class="space-y-3">
          <div>
            <span class="text-xs font-semibold text-zinc-200 uppercase tracking-wider font-mono">
              Display Density
            </span>
            <p class="text-xs text-zinc-400 mt-0.5 leading-relaxed">
              Controls card information density across the launcher without altering component structure.
            </p>
          </div>

          <div class="grid grid-cols-2 gap-3">
            <!-- Minimal (Default) -->
            <button
              type="button"
              onclick={() => onUpdateDensity('minimal')}
              class="flex flex-col items-start p-3.5 min-h-[44px] rounded-lg border text-left transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-zinc-400 {density ===
              'minimal'
                ? 'bg-zinc-800/90 border-zinc-500 text-zinc-100'
                : 'bg-zinc-900/60 border-zinc-800 text-zinc-400 hover:border-zinc-700 hover:text-zinc-200'}"
            >
              <div class="flex items-center justify-between w-full">
                <span class="text-xs font-semibold text-zinc-100">Minimal</span>
                {#if density === 'minimal'}
                  <span class="text-[10px] font-mono px-1.5 py-0.2 rounded bg-zinc-700 text-zinc-200">
                    Default
                  </span>
                {/if}
              </div>
              <p class="text-[11px] text-zinc-400 mt-1 leading-snug">
                Launcher first. App icon, status badge, and single primary action.
              </p>
            </button>

            <!-- Detailed -->
            <button
              type="button"
              onclick={() => onUpdateDensity('detailed')}
              class="flex flex-col items-start p-3.5 min-h-[44px] rounded-lg border text-left transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-zinc-400 {density ===
              'detailed'
                ? 'bg-zinc-800/90 border-zinc-500 text-zinc-100'
                : 'bg-zinc-900/60 border-zinc-800 text-zinc-400 hover:border-zinc-700 hover:text-zinc-200'}"
            >
              <div class="flex items-center justify-between w-full">
                <span class="text-xs font-semibold text-zinc-100">Detailed</span>
              </div>
              <p class="text-[11px] text-zinc-400 mt-1 leading-snug">
                Adds live sparklines, idle countdowns, secondary actions, and MemoryHero card.
              </p>
            </button>
          </div>
        </div>

        <!-- Demo Mode Fixture Toggle -->
        <div class="pt-2 border-t border-zinc-800">
          <label class="flex items-center justify-between p-3 rounded-lg bg-zinc-900/60 border border-zinc-800 cursor-pointer hover:bg-zinc-900 transition-colors">
            <div class="space-y-0.5 pr-4">
              <span class="text-xs font-semibold text-zinc-200 block">Demo Mode Fixture</span>
              <span class="text-[11px] text-zinc-400 block leading-relaxed">
                Loads a simulated dataset demonstrating all 5 card states (Running, Idle, Hibernated, Vault sealed, Closed).
              </span>
            </div>
            <input
              type="checkbox"
              checked={isDemoMode}
              onchange={(e) => onToggleDemoMode(e.currentTarget.checked)}
              class="w-4 h-4 rounded border-zinc-700 bg-zinc-900 text-zinc-100 focus:ring-zinc-400 focus:ring-offset-zinc-900"
            />
          </label>
        </div>

        <!-- Default Inactivity Hibernation -->
        <div class="pt-2 border-t border-zinc-800 space-y-2">
          <label for="global-idle-timeout" class="block text-xs font-semibold text-zinc-200 uppercase tracking-wider font-mono">
            Global Hibernation Threshold
          </label>
          <p class="text-xs text-zinc-400 leading-relaxed">
            Inactivity duration before unfocused container windows are gracefully hibernated back to the host operating system.
          </p>
          <select
            id="global-idle-timeout"
            value={globalIdleThresholdMinutes}
            onchange={(e) => onUpdateIdleThreshold(Number(e.currentTarget.value))}
            class="w-full px-3 py-2 text-xs bg-zinc-900 border border-zinc-700/80 rounded-lg text-zinc-100 focus:border-zinc-400 focus:ring-1 focus:ring-zinc-400 outline-none transition-colors"
          >
            <option value={5}>5 minutes</option>
            <option value={15}>15 minutes (Default & Recommended)</option>
            <option value={30}>30 minutes</option>
            <option value={60}>60 minutes</option>
          </select>
        </div>
      </div>

      <!-- Footer -->
      <div class="px-6 py-4 bg-zinc-900/40 border-t border-zinc-800 flex items-center justify-end">
        <button
          type="button"
          onclick={onClose}
          class="min-h-[44px] px-5 rounded-lg bg-zinc-100 hover:bg-white text-zinc-950 text-xs font-semibold active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-zinc-400 transition-colors shadow-sm"
        >
          Done
        </button>
      </div>
    </div>
  </div>
{/if}
