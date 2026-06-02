<script lang="ts">
  import { showBrain } from '$lib/stores/uiStore';
  import { filteredNotifications, unreadCount, brainSearch, brainAppFilter, notificationApps, notifications } from '$lib/stores/notificationStore';
  import { invoke } from '@tauri-apps/api/core';

  async function clearBrain() {
    notifications.set([]);
    unreadCount.set(0);
    await invoke('clear_notifications');
  }
</script>

{#if $showBrain}
  <div class="fixed top-14 right-0 bottom-0 w-full sm:w-[360px] lg:w-[400px] glass-panel z-[70] border-l border-outline-variant/15 flex flex-col shadow-2xl shadow-primary/5 transition-transform duration-300">
    <div class="p-6 flex items-center justify-between">
      <div>
        <h2 class="text-2xl font-black tracking-tight text-on-surface">Global Brain</h2>
        <p class="text-[10px] font-mono text-outline uppercase mt-1">Cross-Platform Sync Active</p>
      </div>
      <button on:click={clearBrain} class="bg-surface-container-highest hover:bg-surface-container-high text-on-surface-variant px-3 py-1.5 rounded-sm text-[10px] font-bold uppercase tracking-widest transition-all border border-outline-variant/20">
        Clear
      </button>
    </div>

    <div class="px-6 pb-4 grid grid-cols-1 sm:grid-cols-2 gap-3">
      <input
        type="text"
        bind:value={$brainSearch}
        placeholder="Search notifications"
        class="w-full rounded-lg border border-outline-variant/30 bg-surface-container-lowest px-3 py-2 text-xs text-on-surface focus:outline-none focus:ring-1 focus:ring-primary"
      />
      <select
        bind:value={$brainAppFilter}
        class="w-full rounded-lg border border-outline-variant/30 bg-surface-container-lowest px-3 py-2 text-xs text-on-surface focus:outline-none focus:ring-1 focus:ring-primary"
      >
        <option value="all">All Apps</option>
        {#each $notificationApps as appId}
          <option value={appId}>{appId}</option>
        {/each}
      </select>
    </div>
    
    <div class="flex-grow overflow-y-auto vessel-scroll px-6 space-y-4 pb-6">
      {#if $filteredNotifications.length === 0}
        <div class="text-center text-outline-variant mt-20 text-sm">Awaiting neural input streams.</div>
      {/if}
      {#each $filteredNotifications as note}
        <div class="bg-surface-container-low/80 hover:bg-surface-container-high transition-all rounded-lg p-5 border border-outline-variant/10 group cursor-pointer">
          <div class="flex items-start justify-between mb-3">
            <div class="flex items-center space-x-3">
              <div class="w-8 h-8 rounded bg-primary-container/20 flex items-center justify-center">
                <span class="material-symbols-outlined text-primary text-lg">memory</span>
              </div>
              <div>
                <h4 class="text-xs font-bold text-on-surface capitalize">{note.appId}</h4>
                <p class="text-[10px] text-outline font-mono">{note.time}</p>
              </div>
            </div>
          </div>
          <h5 class="text-sm font-bold text-primary-fixed-dim mb-1">{note.title}</h5>
          <p class="text-sm text-on-surface-variant leading-relaxed">{note.body}</p>
        </div>
      {/each}
    </div>
  </div>
{/if}
