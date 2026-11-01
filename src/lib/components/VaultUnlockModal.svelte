<script lang="ts">
  import type { ContainerConfig } from '$lib/types';
  import Icon from '$lib/components/Icon.svelte';

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
    if (e.key === 'Escape' && !isUnlocking && isOpen) {
      onClose();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if isOpen && container}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-sm animate-in fade-in"
    onclick={(e) => {
      if (e.target === e.currentTarget && !isUnlocking) onClose();
    }}
    role="presentation"
  >
    <div
      class="bg-[#15181e] border border-zinc-800 rounded-xl w-full max-w-md shadow-2xl overflow-hidden flex flex-col animate-in zoom-in-95 select-none"
      role="dialog"
      aria-modal="true"
      aria-labelledby="vault-unlock-title"
    >
      <!-- Modal Header -->
      <div class="px-6 py-4 border-b border-zinc-800 flex items-center justify-between">
        <div class="flex items-center gap-2.5">
          <div
            class="w-8 h-8 rounded-lg bg-purple-500/10 border border-purple-500/25 flex items-center justify-center text-purple-400"
          >
            <Icon name="lock" size={16} />
          </div>
          <div>
            <h2 id="vault-unlock-title" class="text-sm font-semibold text-zinc-100">
              Unlock Encrypted Vault
            </h2>
            <p class="text-[11px] text-zinc-400 font-mono mt-0.5">{container.name}</p>
          </div>
        </div>

        <button
          type="button"
          onclick={onClose}
          disabled={isUnlocking}
          class="inline-flex items-center justify-center min-w-[44px] min-h-[44px] rounded-lg text-zinc-400 hover:text-zinc-100 hover:bg-zinc-800 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-purple-400 transition-colors disabled:opacity-40"
          aria-label="Close dialog"
        >
          <Icon name="close" size={18} />
        </button>
      </div>

      <!-- Modal Body Form -->
      <form onsubmit={handleSubmit} class="p-6 space-y-4">
        {#if error}
          <div
            class="p-3 rounded-lg bg-rose-500/10 border border-rose-500/20 text-rose-300 text-xs flex items-center gap-2.5"
            role="alert"
          >
            <Icon name="warning" size={16} class="text-rose-400 shrink-0" />
            <span>{error}</span>
          </div>
        {/if}

        <div class="text-xs text-zinc-400 leading-relaxed bg-zinc-900/60 p-3 rounded-lg border border-zinc-800/80">
          This container's session cookies, localStorage, and IndexedDB are encrypted at rest with <strong class="text-zinc-200">AES-256-GCM</strong>.
          If backed by your native OS Keyring (Windows DPAPI / Apple Keychain), leave this field empty to unlock automatically.
        </div>

        <div>
          <label for="vault-passphrase" class="block text-xs font-medium text-zinc-300 mb-1.5">
            Master Passphrase <span class="text-zinc-500 font-normal">(Optional if stored in OS Keyring)</span>
          </label>
          <div class="relative flex items-center">
            <input
              id="vault-passphrase"
              type={showPassword ? 'text' : 'password'}
              bind:value={passphrase}
              placeholder="Leave blank for OS Keyring, or enter passphrase"
              disabled={isUnlocking}
              class="w-full px-3 py-2 text-xs font-mono bg-zinc-900 border border-zinc-700/80 rounded-lg focus:border-purple-500 focus:ring-1 focus:ring-purple-500 outline-none text-zinc-100 placeholder:text-zinc-500 transition-colors pr-12 min-h-[44px]"
            />
            <button
              type="button"
              onclick={() => (showPassword = !showPassword)}
              class="absolute right-1 inline-flex items-center justify-center min-w-[40px] min-h-[40px] text-zinc-400 hover:text-zinc-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-purple-400 transition-colors rounded"
              aria-label={showPassword ? 'Hide passphrase' : 'Show passphrase'}
            >
              <Icon name={showPassword ? 'lock_open' : 'lock'} size={16} />
            </button>
          </div>
        </div>

        <!-- Footer Actions -->
        <div class="pt-3 flex items-center justify-end gap-2.5">
          <button
            type="button"
            onclick={onClose}
            disabled={isUnlocking}
            class="min-h-[44px] px-4 rounded-lg text-xs font-medium text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-zinc-400 transition-colors disabled:opacity-40"
          >
            Cancel
          </button>
          <button
            type="submit"
            disabled={isUnlocking}
            class="min-h-[44px] inline-flex items-center gap-2 px-5 rounded-lg bg-purple-600 hover:bg-purple-500 text-white text-xs font-semibold active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-purple-400 transition-colors shadow-sm disabled:opacity-50"
          >
            {#if isUnlocking}
              <Icon name="spinner" size={14} />
              <span>Decrypting Vault...</span>
            {:else}
              <Icon name="lock_open" size={14} />
              <span>Unlock & Launch</span>
            {/if}
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}
