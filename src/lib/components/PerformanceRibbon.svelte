<script lang="ts">
  interface Props {
    activeCount: number;
    hibernatedCount: number;
    lockedCount: number;
    totalActiveRam: number;
    totalSavedRam: number;
  }

  let {
    activeCount = 0,
    hibernatedCount = 0,
    lockedCount = 0,
    totalActiveRam = 0,
    totalSavedRam = 0,
  }: Props = $props();

  function formatMemory(mb: number): string {
    if (mb >= 1024) {
      return `${(mb / 1024).toFixed(2)} GB`;
    }
    return `${mb.toFixed(1)} MB`;
  }
</script>

<div class="w-full bg-zinc-950/80 backdrop-blur-md border border-zinc-800/80 rounded-xl px-4 py-2.5 flex flex-wrap items-center justify-between gap-4 shadow-lg text-xs font-mono">
  <!-- Left: Process States Counter -->
  <div class="flex items-center gap-3">
    <!-- Active / Running -->
    <div class="flex items-center gap-1.5 px-2.5 py-1 rounded-md bg-emerald-500/10 border border-emerald-500/20 text-emerald-400">
      <span class="w-1.5 h-1.5 rounded-full bg-emerald-400 {activeCount > 0 ? 'animate-pulse' : ''}"></span>
      <span class="font-semibold">{activeCount}</span>
      <span class="text-[11px] text-emerald-500/90 font-sans">Active</span>
    </div>

    <!-- Hibernated -->
    <div class="flex items-center gap-1.5 px-2.5 py-1 rounded-md bg-sky-500/10 border border-sky-500/20 text-sky-400">
      <span class="material-symbols-outlined text-[13px] leading-none">bedtime</span>
      <span class="font-semibold">{hibernatedCount}</span>
      <span class="text-[11px] text-sky-500/90 font-sans">Hibernated</span>
    </div>

    <!-- Vault Locked -->
    <div class="flex items-center gap-1.5 px-2.5 py-1 rounded-md bg-purple-500/10 border border-purple-500/20 text-purple-400">
      <span class="material-symbols-outlined text-[13px] leading-none">lock</span>
      <span class="font-semibold">{lockedCount}</span>
      <span class="text-[11px] text-purple-500/90 font-sans">Vault Sealed</span>
    </div>
  </div>

  <!-- Right: Real-time Memory Readouts -->
  <div class="flex items-center gap-4">
    <!-- Live RAM Consumption -->
    <div class="flex items-center gap-2">
      <span class="text-zinc-500 font-sans text-[11px]">Active Footprint:</span>
      <span class="px-2 py-0.5 rounded bg-zinc-900 border border-zinc-700/60 font-semibold text-zinc-200">
        {formatMemory(totalActiveRam)}
      </span>
    </div>

    <!-- Memory Saved vs Electron -->
    <div class="flex items-center gap-2">
      <span class="text-zinc-500 font-sans text-[11px]">Reclaimed vs. Electron:</span>
      <span class="px-2 py-0.5 rounded bg-emerald-500/15 border border-emerald-500/30 font-semibold text-emerald-300">
        +{formatMemory(totalSavedRam)}
      </span>
    </div>
  </div>
</div>
