<script lang="ts">
  import { toasts, dismissToast } from '$lib/stores/toastStore';
</script>

<div class="fixed bottom-6 right-6 z-[100] flex flex-col gap-2 pointer-events-none max-w-sm w-full">
  {#each $toasts as toast (toast.id)}
    <div
      class="pointer-events-auto flex items-start gap-3 p-3.5 rounded-xl border border-zinc-800 shadow-2xl backdrop-blur-md transition-all duration-300 animate-in slide-in-from-bottom-3 fade-in bg-vessel-card/95 text-zinc-100"
      role="status"
    >
      <span class="material-symbols-outlined text-lg shrink-0 mt-0.5 {toast.type === 'success' ? 'text-emerald-400' : toast.type === 'error' ? 'text-rose-400' : 'text-zinc-400'}">
        {toast.type === 'success' ? 'check_circle' : toast.type === 'error' ? 'error' : 'info'}
      </span>
      <div class="flex-1 text-xs leading-relaxed font-medium text-zinc-200">
        {toast.message}
      </div>
      <button
        type="button"
        on:click={() => dismissToast(toast.id)}
        class="text-zinc-500 hover:text-zinc-200 transition-colors p-0.5 rounded"
        aria-label="Close notification"
      >
        <span class="material-symbols-outlined text-sm">close</span>
      </button>
    </div>
  {/each}
</div>
