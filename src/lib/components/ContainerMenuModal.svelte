<script lang="ts">
  import type { ContainerConfig, ContainerCustomSettings } from '$lib/types';
  import Icon from '$lib/components/Icon.svelte';

  interface Props {
    isOpen: boolean;
    container: ContainerConfig | null;
    onSave: (containerId: string, settings: ContainerCustomSettings) => void;
    onPinDesktop: (container: ContainerConfig) => void;
    onDelete: (container: ContainerConfig) => void;
    onClose: () => void;
  }

  let {
    isOpen = false,
    container = null,
    onSave,
    onPinDesktop,
    onDelete,
    onClose,
  }: Props = $props();

  let editedName = $state('');
  let editedProfileLabel = $state('');
  let editedNeverHibernate = $state(false);
  let editedIdleThresholdMinutes = $state(15);

  $effect(() => {
    if (container) {
      editedName = container.name || '';
      editedProfileLabel = container.profileLabel || '';
      editedNeverHibernate = Boolean(container.neverHibernate);
      editedIdleThresholdMinutes = container.idleThresholdMinutes || 15;
    }
  });

  function handleSubmit(e: SubmitEvent) {
    e.preventDefault();
    if (!container) return;
    onSave(container.id, {
      name: editedName.trim() || container.name,
      profileLabel: editedProfileLabel.trim() || undefined,
      neverHibernate: editedNeverHibernate,
      idleThresholdMinutes: editedIdleThresholdMinutes,
    });
    onClose();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onClose();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if isOpen && container}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/80 animate-in fade-in"
    onclick={(e) => {
      if (e.target === e.currentTarget) onClose();
    }}
    role="presentation"
  >
    <div
      class="bg-[var(--bg-surface)] border border-[var(--border-subtle)] rounded-2xl w-full max-w-md shadow-2xl overflow-hidden flex flex-col animate-in zoom-in-95 select-none"
      role="dialog"
      aria-modal="true"
      aria-labelledby="container-settings-title"
    >
      <!-- Header -->
      <div class="px-6 py-4 border-b border-[var(--border-subtle)] flex items-center justify-between">
        <div class="flex items-center gap-2.5">
          <div
            class="w-8 h-8 rounded-lg bg-[var(--bg-canvas)] border border-[var(--border-subtle)] flex items-center justify-center text-[var(--accent-primary)]"
          >
            <Icon name="sliders" size={16} />
          </div>
          <div>
            <h2 id="container-settings-title" class="text-sm font-semibold text-[var(--text-primary)]">
              Container Preferences
            </h2>
            <p class="text-[11px] text-[var(--text-muted)] font-mono mt-0.5">{container.name}</p>
          </div>
        </div>

        <button
          type="button"
          onclick={onClose}
          class="inline-flex items-center justify-center min-w-[36px] min-h-[36px] rounded-lg text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-surface-hover)] focus-visible:outline-none transition-colors"
          aria-label="Close dialog"
        >
          <Icon name="close" size={16} />
        </button>
      </div>

      <!-- Form Body -->
      <form onsubmit={handleSubmit} class="p-6 space-y-4">
        <!-- Name Input -->
        <div>
          <label for="edit-container-name" class="block text-xs font-medium text-[var(--text-primary)] mb-1.5">
            Container Name
          </label>
          <input
            id="edit-container-name"
            type="text"
            bind:value={editedName}
            required
            class="w-full px-3 py-2 text-xs bg-[var(--bg-canvas)] border border-[var(--border-subtle)] rounded-xl text-[var(--text-primary)] placeholder:text-[var(--text-muted)] focus:border-[var(--accent-primary)] focus:ring-1 focus:ring-[var(--accent-primary)] outline-none transition-colors"
          />
        </div>

        <!-- Profile Label / Tag -->
        <div>
          <label for="edit-profile-label" class="block text-xs font-medium text-[var(--text-primary)] mb-1.5">
            Profile Label <span class="text-[var(--text-muted)] font-normal">(Optional tag, e.g. Work / Personal)</span>
          </label>
          <input
            id="edit-profile-label"
            type="text"
            bind:value={editedProfileLabel}
            placeholder="e.g. Work Account"
            class="w-full px-3 py-2 text-xs bg-[var(--bg-canvas)] border border-[var(--border-subtle)] rounded-xl text-[var(--text-primary)] placeholder:text-[var(--text-muted)] focus:border-[var(--accent-primary)] focus:ring-1 focus:ring-[var(--accent-primary)] outline-none transition-colors"
          />
        </div>

        <!-- Never Hibernate Toggle -->
        <div class="pt-1">
          <label class="flex items-center justify-between p-3 rounded-xl bg-[var(--bg-canvas)] border border-[var(--border-subtle)] cursor-pointer hover:bg-[var(--bg-surface-hover)] transition-colors">
            <div class="space-y-0.5">
              <span class="text-xs font-medium text-[var(--text-primary)] block">Never Hibernate</span>
              <span class="text-[11px] text-[var(--text-muted)] block leading-normal">
                Keep window process active even when unfocused for long periods
              </span>
            </div>
            <input
              type="checkbox"
              bind:checked={editedNeverHibernate}
              class="w-4 h-4 rounded border-[var(--border-subtle)] bg-[var(--bg-canvas)] text-[var(--accent-primary)] focus:ring-[var(--accent-primary)]"
            />
          </label>
        </div>

        <!-- Idle Threshold Picker -->
        <div>
          <label for="edit-idle-threshold" class="block text-xs font-medium text-[var(--text-primary)] mb-1.5">
            Inactivity Hibernation Threshold
          </label>
          <select
            id="edit-idle-threshold"
            bind:value={editedIdleThresholdMinutes}
            disabled={editedNeverHibernate}
            class="w-full px-3 py-2 text-xs bg-[var(--bg-canvas)] border border-[var(--border-subtle)] rounded-xl text-[var(--text-primary)] focus:border-[var(--accent-primary)] focus:ring-1 focus:ring-[var(--accent-primary)] outline-none transition-colors disabled:opacity-40"
          >
            <option value={5}>5 minutes</option>
            <option value={15}>15 minutes (Recommended)</option>
            <option value={30}>30 minutes</option>
            <option value={60}>60 minutes (1 hour)</option>
          </select>
        </div>

        <!-- Secondary Shortcuts & Actions -->
        <div class="pt-2 border-t border-[var(--border-subtle)] flex flex-col gap-2">
          <button
            type="button"
            onclick={() => {
              if (container) onPinDesktop(container);
            }}
            class="w-full flex items-center justify-between min-h-[38px] px-3.5 rounded-xl bg-[var(--bg-canvas)] border border-[var(--border-subtle)] hover:border-[var(--accent-primary)]/50 text-xs text-[var(--text-primary)] transition-colors"
          >
            <div class="flex items-center gap-2">
              <Icon name="pin" size={15} class="text-[var(--accent-primary)]" />
              <span>Pin to Desktop / Create Shortcut</span>
            </div>
            <span class="text-[10px] text-[var(--text-muted)] font-mono">.lnk / .command</span>
          </button>

          <button
            type="button"
            onclick={() => {
              if (container) onDelete(container);
              onClose();
            }}
            class="w-full flex items-center justify-between min-h-[38px] px-3.5 rounded-xl bg-rose-500/10 border border-rose-500/20 hover:bg-rose-500/15 text-xs text-rose-400 hover:text-rose-300 transition-colors"
          >
            <div class="flex items-center gap-2">
              <Icon name="delete" size={15} class="text-rose-400" />
              <span>Delete Container & Wipe Vault</span>
            </div>
            <span class="text-[10px] text-rose-400 font-mono">Permanent</span>
          </button>
        </div>

        <!-- Footer Actions -->
        <div class="pt-3 flex items-center justify-end gap-2.5">
          <button
            type="button"
            onclick={onClose}
            class="min-h-[38px] px-4 rounded-xl text-xs font-medium text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-surface-hover)] focus-visible:outline-none transition-colors"
          >
            Cancel
          </button>
          <button
            type="submit"
            class="min-h-[38px] px-4 rounded-xl bg-[var(--accent-primary)] hover:opacity-95 text-[var(--accent-fg)] text-xs font-semibold active:scale-[0.98] transition-colors shadow-sm"
          >
            Save Changes
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}
