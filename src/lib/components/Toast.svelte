<script lang="ts">
  import type { Toast } from '$lib/stores/toastStore';
  import Icon from '$lib/components/Icon.svelte';

  interface Props {
    toast: Toast;
    onDismiss: (id: string) => void;
  }

  let { toast, onDismiss }: Props = $props();

  function handleActionClick() {
    if (toast.action) {
      toast.action.onClick();
      onDismiss(toast.id);
    }
  }
</script>

<div
  class="pointer-events-auto flex items-center justify-between gap-3 p-3 rounded-xl border border-zinc-800 shadow-2xl bg-[#15181e] text-zinc-100 min-h-[52px]"
  role="status"
  aria-live="polite"
>
  <div class="flex items-center gap-3 min-w-0">
    <div
      class="w-7 h-7 rounded-lg flex items-center justify-center shrink-0 {toast.type === 'success'
        ? 'bg-emerald-500/10 text-emerald-400'
        : toast.type === 'error'
          ? 'bg-rose-500/10 text-rose-400'
          : 'bg-zinc-800 text-zinc-300'}"
    >
      {#if toast.type === 'success'}
        <Icon name="check" size={16} />
      {:else if toast.type === 'error'}
        <Icon name="warning" size={16} />
      {:else}
        <Icon name="info" size={16} />
      {/if}
    </div>

    <div class="text-xs font-medium text-zinc-200 leading-snug">
      {toast.message}
    </div>
  </div>

  <div class="flex items-center gap-1 shrink-0">
    {#if toast.action}
      <button
        type="button"
        onclick={handleActionClick}
        class="inline-flex items-center gap-1.5 px-3 py-2 min-h-[44px] rounded-lg bg-zinc-800 hover:bg-zinc-700 active:scale-[0.98] text-xs font-semibold text-zinc-100 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-zinc-400 transition-colors"
      >
        <Icon name="undo" size={13} />
        <span>{toast.action.label}</span>
      </button>
    {/if}

    <button
      type="button"
      onclick={() => onDismiss(toast.id)}
      class="inline-flex items-center justify-center w-11 h-11 min-w-[44px] min-h-[44px] rounded-lg text-zinc-400 hover:text-zinc-100 hover:bg-zinc-800/80 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-zinc-400 transition-colors"
      aria-label="Dismiss notification"
    >
      <Icon name="close" size={16} />
    </button>
  </div>
</div>
