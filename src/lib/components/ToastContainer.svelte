<script lang="ts">
  import { toasts, dismissToast } from '$lib/stores/toastStore';
  import Toast from '$lib/components/Toast.svelte';
</script>

<div class="fixed bottom-6 right-6 z-[100] flex flex-col gap-2.5 pointer-events-none max-w-md w-full px-4 sm:px-0">
  {#each $toasts as toast (toast.id)}
    <Toast {toast} onDismiss={dismissToast} />
  {/each}
</div>
