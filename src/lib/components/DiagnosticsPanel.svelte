<script lang="ts">
  import { showDiagnostics, safeMode } from '$lib/stores/uiStore';
  import { diagnostics } from '$lib/stores/diagnosticStore';
</script>

{#if $showDiagnostics}
  <div class="fixed top-14 right-0 bottom-0 w-full sm:w-[360px] lg:w-[420px] glass-panel z-[75] border-l border-outline-variant/15 flex flex-col shadow-2xl">
    <div class="p-4 border-b border-outline-variant/10 flex items-center justify-between">
      <div>
        <h3 class="text-lg font-bold text-on-surface">Diagnostics</h3>
        <p class="text-[10px] uppercase tracking-widest text-on-surface-variant">IPC and Injection Events</p>
      </div>
      <button
        on:click={() => diagnostics.set([])}
        class="bg-surface-container-highest hover:bg-surface-container-high text-on-surface-variant px-3 py-1 rounded text-[10px] uppercase tracking-widest border border-outline-variant/20"
      >
        Clear
      </button>
    </div>

    <div class="px-4 py-2 text-xs text-on-surface-variant border-b border-outline-variant/10">
      Safe mode: <span class={$safeMode ? 'text-error' : 'text-primary'}>{$safeMode ? 'ON' : 'OFF'}</span>
    </div>

    <div class="flex-grow overflow-y-auto vessel-scroll p-4 space-y-3">
      {#if $diagnostics.length === 0}
        <div class="text-sm text-outline-variant text-center mt-10">No diagnostics yet.</div>
      {/if}
      {#each $diagnostics as entry}
        <div class="rounded-lg border border-outline-variant/20 bg-surface-container-lowest p-3">
          <div class="flex items-center justify-between mb-1">
            <span class="text-[10px] uppercase tracking-wider {entry.level === 'warn' ? 'text-error' : 'text-primary'}">{entry.level}</span>
            <span class="text-[10px] text-outline">{entry.time}</span>
          </div>
          <div class="text-xs text-on-surface mb-1">{entry.message}</div>
          <div class="text-[10px] text-on-surface-variant">{entry.category} | {entry.appId}</div>
          {#if entry.detail}
            <pre class="mt-2 text-[10px] text-on-surface-variant whitespace-pre-wrap break-words">{entry.detail}</pre>
          {/if}
        </div>
      {/each}
    </div>
  </div>
{/if}
