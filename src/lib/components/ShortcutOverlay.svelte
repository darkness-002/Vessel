<script lang="ts">
  import { showShortcuts } from '$lib/stores/uiStore';

  const keyboardShortcuts = [
    { key: 'Ctrl + G', action: 'Go to Gallery' },
    { key: 'Ctrl + N', action: 'Deploy New Site' },
    { key: 'Ctrl + B', action: 'Toggle Global Brain' },
    { key: 'Ctrl + D', action: 'Toggle Diagnostics' },
    { key: 'Ctrl + S', action: 'Toggle Safe Mode' },
    { key: 'Esc', action: 'Close Overlays / Back' }
  ];
</script>

{#if $showShortcuts}
  <div class="fixed inset-0 z-[100] flex items-center justify-center p-4">
    <div 
      class="absolute inset-0 bg-background/80 backdrop-blur-sm" 
      on:click={() => showShortcuts.set(false)}
      role="button"
      tabindex="0"
      on:keydown={(e) => e.key === 'Escape' && showShortcuts.set(false)}
      aria-label="Close shortcuts modal"
    ></div>
    <div class="relative bg-surface-container border border-outline-variant/20 rounded-2xl shadow-2xl w-full max-w-md overflow-hidden animate-in zoom-in-95 duration-200">
      <div class="p-6 border-b border-outline-variant/10 flex items-center justify-between">
        <div class="flex items-center gap-3">
          <span class="material-symbols-outlined text-primary">keyboard</span>
          <h2 class="text-xl font-bold text-on-surface">Keyboard Shortcuts</h2>
        </div>
        <button on:click={() => showShortcuts.set(false)} class="p-1 rounded-full hover:bg-surface-container-highest transition-colors text-outline">
          <span class="material-symbols-outlined">close</span>
        </button>
      </div>
      <div class="p-6 space-y-4">
        {#each keyboardShortcuts as shortcut}
          <div class="flex items-center justify-between group">
            <span class="text-sm text-on-surface-variant group-hover:text-on-surface transition-colors">{shortcut.action}</span>
            <kbd class="px-2 py-1 rounded bg-surface-container-highest border border-outline-variant/30 text-[10px] font-mono font-bold text-primary shadow-sm">{shortcut.key}</kbd>
          </div>
        {/each}
      </div>
      <div class="bg-surface-container-high p-4 flex justify-center">
        <p class="text-[10px] text-outline-variant uppercase tracking-widest font-bold italic">Vessel Control Plane v0.1.0</p>
      </div>
    </div>
  </div>
{/if}
