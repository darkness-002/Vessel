<script lang="ts">
  import type { ContainerConfig } from '$lib/types';

  interface Props {
    isOpen: boolean;
    container: ContainerConfig | null;
    isUnlocking: boolean;
    error?: string;
    onUnlock: (passphrase?: string) => void;
    onClose: () => void;
  }

  let {
    isOpen = false,
    container = null,
    isUnlocking = false,
    error = '',
    onUnlock,
    onClose,
  }: Props = $props();

  let passphrase = $state('');
  let showPassword = $state(false);

  function handleSubmit(e: SubmitEvent) {
    e.preventDefault();
    onUnlock(passphrase.trim() ? passphrase : undefined);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && !isUnlocking) {
      onClose();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if isOpen && container}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-sm transition-all duration-150 ease-out animate-in fade-in"
    onclick={(e) => { if (e.target === e.currentTarget && !isUnlocking) onClose(); }}
    role="presentation"
  >
    <div
      class="bg-vessel-card border border-zinc-800 rounded-xl w-full max-w-md shadow-2xl overflow-hidden flex flex-col transition-all duration-150 ease-out animate-in zoom-in-95"
      role="dialog"
      aria-modal="true"
    >
      <!-- Modal Header -->
      <div class="px-6 py-4 border-b border-zinc-800 flex items-center justify-between">
        <div class="flex items-center gap-2.5">
          <div class="w-7 h-7 rounded-lg bg-purple-500/10 border border-purple-500/25 flex items-center justify-center text-purple-400">
            <span class="material-symbols-outlined text-sm">lock</span>
          </div>
          <div>
            <h2 class="text-sm font-semibold text-zinc-100">Unlock Encrypted Vault</h2>
            <p class="text-[11px] text-zinc-400 font-mono mt-0.5">{container.name}</p>
          </div>
        </div>

        <button
          type="button"
          onclick={onClose}
          disabled={isUnlocking}
          class="text-zinc-500 hover:text-zinc-200 p-1 rounded-md transition-colors duration-150 disabled:opacity-40"
          aria-label="Close dialog"
        >
          <span class="material-symbols-outlined text-lg">close</span>
        </button>
      </div>

      <!-- Modal Body Form -->
      <form onsubmit={handleSubmit} class="p-6 space-y-4">
        {#if error}
          <div class="p-2.5 rounded-md bg-rose-500/10 border border-rose-500/20 text-rose-400 text-xs flex items-center gap-2">
            <span class="material-symbols-outlined text-sm shrink-0">error</span>
            <span>{error}</span>
          </div>
        {/if}

        <div class="text-xs text-zinc-400 leading-relaxed bg-zinc-900/60 p-3 rounded-lg border border-zinc-800/80">
          This container's session cookies, localStorage, and IndexedDB are encrypted at rest with <strong class="text-zinc-200">AES-256-GCM</strong>.
          If backed by your native OS Keyring (Windows DPAPI / Apple Keychain), leave this field empty to unlock automatically.
        </div>

        <div>
          <label for="vault-passphrase" class="block text-xs font-medium text-zinc-300 mb-1.5">
            Master Passphrase <span class="text-zinc-500 font-normal">(Optional if stored in Keyring)</span>
          </label>
          <div class="relative">
            <input
              id="vault-passphrase"
              type={showPassword ? 'text' : 'password'}
              bind:value={passphrase}
              placeholder="Leave blank for OS Keyring, or enter passphrase"
              disabled={isUnlocking}
              class="px-3 py-2 text-sm bg-zinc-900 border border-zinc-700/80 rounded-md focus:border-purple-500 focus:ring-1 focus:ring-purple-500 outline-none text-zinc-100 placeholder:text-zinc-500 transition-all duration-150 ease-out w-full pr-10 font-mono text-xs"
            />
            <button
              type="button"
              onclick={() => showPassword = !showPassword}
              class="absolute right-2.5 top-1/2 -translate-y-1/2 text-zinc-500 hover:text-zinc-300 p-0.5 rounded transition-colors duration-150"
              tabindex="-1"
              aria-label={showPassword ? 'Hide passphrase' : 'Show passphrase'}
            >
              <span class="material-symbols-outlined text-base">
                {showPassword ? 'visibility_off' : 'visibility'}
              </span>
            </button>
          </div>
        </div>

        <!-- Footer Actions -->
        <div class="pt-2 flex items-center justify-end gap-2.5">
          <button
            type="button"
            onclick={onClose}
            disabled={isUnlocking}
            class="px-3.5 py-1.5 rounded-md text-xs font-medium text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800 active:scale-[0.98] transition-all duration-150 disabled:opacity-40"
          >
            Cancel
          </button>
          <button
            type="submit"
            disabled={isUnlocking}
            class="flex items-center gap-1.5 px-4 py-1.5 rounded-md bg-purple-600 hover:bg-purple-500 text-white text-xs font-semibold active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-purple-400 transition-all duration-150 ease-out shadow-sm disabled:opacity-50"
          >
            {#if isUnlocking}
              <span class="material-symbols-outlined text-xs animate-spin">progress_activity</span>
              <span>Decrypting Vault...</span>
            {:else}
              <span class="material-symbols-outlined text-xs">lock_open</span>
              <span>Unlock & Launch</span>
            {/if}
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}
