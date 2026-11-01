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
    class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-sm animate-in fade-in"
    onclick={(e) => {
      if (e.target === e.currentTarget) onClose();
    }}
    role="presentation"
  >
    <div
      class="bg-[#15181e] border border-zinc-800 rounded-xl w-full max-w-md shadow-2xl overflow-hidden flex flex-col animate-in zoom-in-95 select-none"
      role="dialog"
      aria-modal="true"
      aria-labelledby="container-settings-title"
    >
      <!-- Header -->
      <div class="px-6 py-4 border-b border-zinc-800 flex items-center justify-between">
        <div class="flex items-center gap-2.5">
          <div
            class="w-8 h-8 rounded-lg bg-zinc-800 border border-zinc-700/60 flex items-center justify-center text-zinc-300"
          >
            <Icon name="sliders" size={16} />
          </div>
          <div>
            <h2 id="container-settings-title" class="text-sm font-semibold text-zinc-100">
              Container Preferences
            </h2>
            <p class="text-[11px] text-zinc-400 font-mono mt-0.5">{container.name}</p>
          </div>
        </div>

        <button
          type="button"
          onclick={onClose}
          class="inline-flex items-center justify-center min-w-[44px] min-h-[44px] rounded-lg text-zinc-400 hover:text-zinc-100 hover:bg-zinc-800 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-zinc-400 transition-colors"
          aria-label="Close dialog"
        >
          <Icon name="close" size={18} />
        </button>
      </div>

      <!-- Form Body -->
      <form onsubmit={handleSubmit} class="p-6 space-y-4">
        <!-- Name Input -->
        <div>
          <label for="edit-container-name" class="block text-xs font-medium text-zinc-300 mb-1.5">
            Container Name
          </label>
          <input
            id="edit-container-name"
            type="text"
            bind:value={editedName}
            required
            class="w-full px-3 py-2 text-sm bg-zinc-900 border border-zinc-700/80 rounded-lg text-zinc-100 placeholder:text-zinc-500 focus:border-zinc-400 focus:ring-1 focus:ring-zinc-400 outline-none transition-colors"
          />
        </div>

        <!-- Profile Label / Tag -->
        <div>
          <label for="edit-profile-label" class="block text-xs font-medium text-zinc-300 mb-1.5">
            Profile Label <span class="text-zinc-500 font-normal">(Optional tag, e.g. Work / Personal)</span>
          </label>
          <input
            id="edit-profile-label"
            type="text"
            bind:value={editedProfileLabel}
            placeholder="e.g. Work Account"
            class="w-full px-3 py-2 text-sm bg-zinc-900 border border-zinc-700/80 rounded-lg text-zinc-100 placeholder:text-zinc-500 focus:border-zinc-400 focus:ring-1 focus:ring-zinc-400 outline-none transition-colors"
          />
        </div>

        <!-- Never Hibernate Toggle -->
        <div class="pt-1">
          <label class="flex items-center justify-between p-3 rounded-lg bg-zinc-900/60 border border-zinc-800 cursor-pointer hover:bg-zinc-900 transition-colors">
            <div class="space-y-0.5">
              <span class="text-xs font-medium text-zinc-200 block">Never Hibernate</span>
              <span class="text-[11px] text-zinc-400 block leading-normal">
                Keep window process active even when unfocused for long periods
              </span>
            </div>
            <input
              type="checkbox"
              bind:checked={editedNeverHibernate}
              class="w-4 h-4 rounded border-zinc-700 bg-zinc-900 text-zinc-100 focus:ring-zinc-400 focus:ring-offset-zinc-900"
            />
          </label>
        </div>

        <!-- Idle Threshold Picker -->
        <div>
          <label for="edit-idle-threshold" class="block text-xs font-medium text-zinc-300 mb-1.5">
            Inactivity Hibernation Threshold
          </label>
          <select
            id="edit-idle-threshold"
            bind:value={editedIdleThresholdMinutes}
            disabled={editedNeverHibernate}
            class="w-full px-3 py-2 text-xs bg-zinc-900 border border-zinc-700/80 rounded-lg text-zinc-100 focus:border-zinc-400 focus:ring-1 focus:ring-zinc-400 outline-none transition-colors disabled:opacity-40"
          >
            <option value={5}>5 minutes</option>
            <option value={15}>15 minutes (Recommended)</option>
            <option value={30}>30 minutes</option>
            <option value={60}>60 minutes (1 hour)</option>
          </select>
        </div>

        <!-- Secondary Shortcuts & Actions -->
        <div class="pt-2 border-t border-zinc-800/80 flex flex-col gap-2">
          <button
            type="button"
            onclick={() => {
              if (container) onPinDesktop(container);
            }}
            class="w-full flex items-center justify-between min-h-[44px] px-3.5 rounded-lg bg-zinc-900/80 border border-zinc-800 hover:border-zinc-700 text-xs text-zinc-300 hover:text-zinc-100 transition-colors"
          >
            <div class="flex items-center gap-2">
              <Icon name="pin" size={15} class="text-zinc-400" />
              <span>Pin to Desktop / Create Shortcut</span>
            </div>
            <span class="text-[10px] text-zinc-500 font-mono">.lnk / .command</span>
          </button>

          <button
            type="button"
            onclick={() => {
              if (container) onDelete(container);
              onClose();
            }}
            class="w-full flex items-center justify-between min-h-[44px] px-3.5 rounded-lg bg-rose-500/10 border border-rose-500/20 hover:bg-rose-500/15 text-xs text-rose-400 hover:text-rose-300 transition-colors"
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
            class="min-h-[44px] px-4 rounded-lg text-xs font-medium text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-zinc-400 transition-colors"
          >
            Cancel
          </button>
          <button
            type="submit"
            class="min-h-[44px] px-4 rounded-lg bg-zinc-100 hover:bg-white text-zinc-950 text-xs font-semibold active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-zinc-400 transition-colors shadow-sm"
          >
            Save Changes
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}
