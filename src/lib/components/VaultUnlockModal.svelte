<script lang="ts">
  import type { ContainerConfig, VaultMode } from '$lib/types';
  import Icon from '$lib/components/Icon.svelte';

  interface Props {
    isOpen: boolean;
    container: ContainerConfig | null;
    vaultMode?: VaultMode;
    isUnlocking: boolean;
    error?: string;
    onUnlock: (passphrase?: string) => void;
    onClose: () => void;
  }

  let {
    isOpen = false,
    container = null,
    vaultMode = 'keyring',
    isUnlocking = false,
    error = '',
    onUnlock,
    onClose,
  }: Props = $props();

  let passphrase = $state('');
  let showPassword = $state(false);

  function handleSubmit(e: SubmitEvent) {
    e.preventDefault();
    if (vaultMode === 'passphrase') {
      onUnlock(passphrase.trim());
    } else {
      onUnlock(undefined);
    }
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
    class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/80 animate-in fade-in"
    onclick={(e) => {
      if (e.target === e.currentTarget && !isUnlocking) onClose();
    }}
    role="presentation"
  >
    <div
      class="bg-[var(--bg-surface)] border border-[var(--border-subtle)] rounded-2xl w-full max-w-md shadow-2xl overflow-hidden flex flex-col animate-in zoom-in-95 select-none"
      role="dialog"
      aria-modal="true"
      aria-labelledby="vault-unlock-title"
    >
      <!-- Modal Header -->
      <div class="px-6 py-4 border-b border-[var(--border-subtle)] flex items-center justify-between">
        <div class="flex items-center gap-2.5">
          <div
            class="w-8 h-8 rounded-lg bg-[var(--bg-canvas)] border border-[var(--border-subtle)] flex items-center justify-center text-[var(--accent-primary)]"
          >
            <Icon name="lock" size={16} />
          </div>
          <div>
            <h2 id="vault-unlock-title" class="text-sm font-semibold text-[var(--text-primary)]">
              Unlock Encrypted Vault
            </h2>
            <p class="text-[11px] text-[var(--text-muted)] font-mono mt-0.5">{container.name}</p>
          </div>
        </div>

        <button
          type="button"
          onclick={onClose}
          disabled={isUnlocking}
          class="inline-flex items-center justify-center min-w-[36px] min-h-[36px] rounded-lg text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-surface-hover)] focus-visible:outline-none transition-colors disabled:opacity-40"
          aria-label="Close dialog"
        >
          <Icon name="close" size={16} />
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

        {#if vaultMode === 'passphrase'}
          <div class="text-xs text-[var(--text-muted)] leading-relaxed bg-[var(--bg-canvas)] p-3 rounded-xl border border-[var(--border-subtle)]">
            This container's profile is encrypted at rest with <strong class="text-[var(--text-primary)]">AES-256-GCM</strong> using a custom master passphrase.
            Please enter your passphrase to decrypt and launch.
          </div>

          <div>
            <label for="vault-passphrase" class="block text-xs font-medium text-[var(--text-primary)] mb-1.5">
              Master Passphrase
            </label>
            <div class="relative flex items-center">
              <input
                id="vault-passphrase"
                type={showPassword ? 'text' : 'password'}
                bind:value={passphrase}
                placeholder="Enter master passphrase"
                required
                disabled={isUnlocking}
                class="w-full px-3 py-2 text-xs font-mono bg-[var(--bg-canvas)] border border-[var(--border-subtle)] rounded-xl focus:border-[var(--accent-primary)] focus:ring-1 focus:ring-[var(--accent-primary)] outline-none text-[var(--text-primary)] placeholder:text-[var(--text-muted)] transition-colors pr-12 min-h-[40px]"
              />
              <button
                type="button"
                onclick={() => (showPassword = !showPassword)}
                class="absolute right-1 inline-flex items-center justify-center min-w-[36px] min-h-[36px] text-[var(--text-muted)] hover:text-[var(--text-primary)] focus-visible:outline-none transition-colors rounded"
                aria-label={showPassword ? 'Hide passphrase' : 'Show passphrase'}
              >
                <Icon name={showPassword ? 'lock_open' : 'lock'} size={16} />
              </button>
            </div>
          </div>
        {:else}
          <div class="text-xs text-[var(--text-muted)] leading-relaxed bg-[var(--bg-canvas)] p-3.5 rounded-xl border border-[var(--border-subtle)] space-y-1.5">
            <p>
              This container's session cookies and storage are encrypted at rest with <strong class="text-[var(--text-primary)]">AES-256-GCM</strong>.
            </p>
            <p class="text-[var(--text-primary)]">
              The cryptographic key is stored in your native <strong>OS Credential Store</strong>. Click Unlock to launch.
            </p>
          </div>
        {/if}

        <!-- Footer Actions -->
        <div class="pt-3 flex items-center justify-end gap-2.5">
          <button
            type="button"
            onclick={onClose}
            disabled={isUnlocking}
            class="min-h-[38px] px-4 rounded-xl text-xs font-medium text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-surface-hover)] focus-visible:outline-none transition-colors disabled:opacity-40"
          >
            Cancel
          </button>
          <button
            type="submit"
            disabled={isUnlocking}
            class="min-h-[38px] inline-flex items-center gap-2 px-5 rounded-xl bg-[var(--accent-primary)] hover:opacity-95 text-[var(--accent-fg)] text-xs font-semibold active:scale-[0.98] transition-colors shadow-sm disabled:opacity-50"
          >
            {#if isUnlocking}
              <Icon name="spinner" size={14} />
              <span>Decrypting Vault...</span>
            {:else}
              <Icon name="lock_open" size={14} />
              <span>{vaultMode === 'passphrase' ? 'Unlock & Launch' : 'Unlock via OS Keyring'}</span>
            {/if}
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}
