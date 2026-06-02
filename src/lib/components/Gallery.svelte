<script lang="ts">
  import { apps, toDisplayName, persistApps, normalizeApp, newAppForm } from '$lib/stores/appStore';
  import { currentView, activeId, switchView } from '$lib/stores/uiStore';
  import { addTabForApp, switchToTab, tabs } from '$lib/stores/tabStore';
  import { diagnostics } from '$lib/stores/diagnosticStore';
  import { editingApp, initialSettingsState } from '$lib/stores/appStore';

  export let store: any;

  const featuredApps = [
    { name: 'ChatGPT', url: 'https://chat.openai.com', icon: '🤖' },
    { name: 'Notion', url: 'https://www.notion.so', icon: '📝' },
    { name: 'Spotify', url: 'https://open.spotify.com', icon: '🎵' }
  ];

  async function launch(app: any) {
    const existingTab = $tabs.find((tab) => tab.appId === app.id);
    if (existingTab) {
      await switchToTab(existingTab.id);
      return;
    }
    await addTabForApp(app);
  }

  function openSettings(app: any) {
    editingApp.set(JSON.parse(JSON.stringify(app)));
    initialSettingsState.set(JSON.stringify(app));
    switchView('settings');
  }

  function exportWorkspace() {
    const data = JSON.stringify($apps, null, 2);
    const blob = new Blob([data], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `vessel_workspace_${new Date().toISOString().split('T')[0]}.json`;
    a.click();
    URL.revokeObjectURL(url);
  }

  async function importWorkspace(event: Event) {
    const input = event.target as HTMLInputElement;
    if (!input.files || input.files.length === 0) return;
    
    const file = input.files[0];
    const reader = new FileReader();
    reader.onload = async (e) => {
      try {
        const content = e.target?.result as string;
        const importedApps = JSON.parse(content);
        if (!Array.isArray(importedApps)) throw new Error('Invalid format');
        
        if (confirm(`Import ${importedApps.length} apps? This will merge with your current workspace.`)) {
          // Merge by ID, imported takes precedence
          const merged = [...$apps];
          for (const imp of importedApps) {
            const idx = merged.findIndex(a => a.id === imp.id);
            if (idx !== -1) merged[idx] = imp;
            else merged.push(imp);
          }
          const normalized = merged.map(a => normalizeApp(a));
          apps.set(normalized);
          await persistApps(normalized, store);
          alert('Workspace imported successfully.');
        }
      } catch (err) {
        alert('Failed to import workspace: ' + err);
      } finally {
        input.value = '';
      }
    };
    reader.readAsText(file);
  }

  function deployFeatured(featured: { name: string, url: string, icon: string }) {
    newAppForm.set({
      name: featured.name,
      url: featured.url,
      icon: featured.icon
    });
    switchView('add-app');
  }
</script>

<div class="flex flex-col lg:flex-row gap-8 sm:gap-12">
  <div class="flex-1">
    <header class="mb-8 flex flex-col sm:flex-row sm:items-end justify-between gap-4">
      <div>
        <h1 class="font-headline font-extrabold text-3xl sm:text-4xl lg:text-5xl tracking-tight text-on-surface mb-2">Workspace Gallery</h1>
        <p class="text-on-surface-variant font-body max-w-xl text-sm sm:text-base lg:text-lg">Deploy isolated web applications and manage your active containers.</p>
      </div>
      <div class="flex items-center gap-2">
        <input type="file" id="import-workspace" accept=".json" on:change={importWorkspace} class="hidden" />
        <button 
          on:click={() => document.getElementById('import-workspace')?.click()}
          class="flex items-center gap-2 px-3 py-1.5 rounded bg-surface-container-highest hover:bg-surface-bright text-on-surface-variant hover:text-on-surface transition-all border border-outline-variant/20 text-[10px] font-bold uppercase tracking-widest"
          title="Import apps from JSON"
        >
          <span class="material-symbols-outlined text-sm">upload</span> Import
        </button>
        <button 
          on:click={exportWorkspace}
          class="flex items-center gap-2 px-3 py-1.5 rounded bg-surface-container-highest hover:bg-surface-bright text-on-surface-variant hover:text-on-surface transition-all border border-outline-variant/20 text-[10px] font-bold uppercase tracking-widest"
          title="Export apps to JSON"
        >
          <span class="material-symbols-outlined text-sm">download</span> Export
        </button>
      </div>
    </header>

    <div class="grid grid-cols-[repeat(auto-fill,minmax(280px,1fr))] gap-4 sm:gap-6 w-full">
      <button 
        on:click={() => switchView('add-app')} 
        class="group bg-surface-container-lowest rounded-xl p-4 sm:p-6 border-2 border-dashed border-primary/30 flex flex-col items-center justify-center hover:bg-primary/5 hover:border-primary hover:-translate-y-1 hover:shadow-lg transition-all min-h-[160px] space-y-3 cursor-pointer focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary"
        aria-label="Deploy New Site"
      >
        <div class="w-12 h-12 rounded-full bg-primary/10 flex items-center justify-center group-hover:bg-primary/20 transition-colors group-hover:scale-110 duration-200">
          <span class="material-symbols-outlined text-primary transition-colors">rocket_launch</span>
        </div>
        <div class="text-center">
          <h3 class="font-headline font-bold text-on-surface">Deploy New Site</h3>
          <p class="text-[10px] text-primary uppercase tracking-widest mt-1 font-bold">Primary Action</p>
        </div>
      </button>

      {#each $apps as app}
        <div 
          class="group bg-surface-container rounded-xl p-4 sm:p-6 border border-outline-variant/10 flex flex-col justify-between hover:bg-surface-container-high hover:border-primary/40 hover:-translate-y-1 hover:shadow-lg transition-all min-h-[160px] cursor-pointer" 
          on:click={() => launch(app)} 
          role="button" 
          tabindex="0" 
          on:keydown={(e) => (e.key === 'Enter' || e.key === ' ') && launch(app)}
          aria-label="Launch {toDisplayName(app)}"
        >
          <div>
            <div class="flex items-center justify-between mb-4">
              <div class="w-10 h-10 bg-surface-container-lowest rounded-lg flex items-center justify-center ring-1 ring-outline-variant/20 text-xl font-bold text-on-surface group-hover:ring-primary/50 transition-all">
                {app.icon}
              </div>
              <div class="flex items-center gap-2">
                <span class="text-on-surface-variant text-[10px] font-mono bg-surface-container-highest px-2 py-0.5 rounded uppercase tracking-tighter">Active</span>
                <button 
                  on:click|stopPropagation={() => openSettings(app)} 
                  class="p-1 rounded-md text-on-surface-variant hover:text-primary hover:bg-primary/10 transition-colors"
                  title="Configure Instance"
                  aria-label="Configure {toDisplayName(app)}"
                >
                  <span class="material-symbols-outlined text-sm">tune</span>
                </button>
              </div>
            </div>
            <div class="flex justify-between items-start mb-1">
              <h3 class="font-headline font-bold text-base text-on-surface capitalize group-hover:text-primary transition-colors">{toDisplayName(app)}</h3>
            </div>
            <p class="text-on-surface-variant text-xs mb-4 truncate opacity-70">{app.url}</p>
          </div>
          <div class="flex items-center justify-between mt-2 border-t border-outline-variant/5 pt-3">
            <span class="text-primary font-label text-[10px] font-bold tracking-widest uppercase flex items-center gap-1 group-hover:brightness-125">
              <span class="material-symbols-outlined text-sm">rocket_launch</span> Launch Container
            </span>
          </div>
        </div>
      {/each}

      {#each featuredApps.filter(f => !$apps.some(a => a.name === f.name)) as featured}
        <button 
          on:click={() => deployFeatured(featured)}
          class="group bg-surface-container-low/50 rounded-xl p-4 sm:p-6 border border-outline-variant/10 flex flex-col justify-between hover:bg-surface-container-high hover:border-primary/30 hover:-translate-y-1 hover:shadow-lg transition-all min-h-[160px] cursor-pointer text-left"
          aria-label="Deploy {featured.name}"
        >
          <div>
            <div class="flex items-center justify-between mb-4">
              <div class="w-10 h-10 bg-surface-container-lowest rounded-lg flex items-center justify-center ring-1 ring-outline-variant/20 text-xl font-bold text-on-surface opacity-60 group-hover:opacity-100 transition-all">
                {featured.icon}
              </div>
              <span class="text-primary text-[9px] font-bold bg-primary/10 px-2 py-0.5 rounded uppercase tracking-widest">Featured</span>
            </div>
            <h3 class="font-headline font-bold text-base text-on-surface/60 group-hover:text-on-surface transition-colors">Deploy {featured.name}</h3>
            <p class="text-on-surface-variant text-[10px] mt-1 opacity-50">Quick-start template</p>
          </div>
          <div class="flex items-center gap-1 text-[10px] font-bold text-outline uppercase tracking-widest group-hover:text-primary transition-colors">
            <span class="material-symbols-outlined text-sm">add_circle</span> Add Instance
          </div>
        </button>
      {/each}

      <!-- Density placeholders -->
      {#if $apps.length + featuredApps.length < 6}
        {#each Array(6 - ($apps.length + featuredApps.length)) as _, i}
          <button 
            on:click={() => switchView('add-app')}
            class="group bg-surface-dim/30 rounded-xl p-6 border border-dashed border-outline-variant/20 flex flex-col items-center justify-center hover:bg-surface-container-high hover:border-primary/50 hover:-translate-y-1 active:scale-[0.98] transition-all cursor-pointer min-h-[160px] opacity-40 hover:opacity-100 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary"
            aria-label="Add Container Slot"
          >
            <span class="material-symbols-outlined text-3xl text-outline-variant group-hover:text-primary mb-2 transition-all group-hover:scale-110">add_circle</span>
            <p class="text-[10px] uppercase font-bold tracking-widest text-outline-variant group-hover:text-on-surface">Empty Slot</p>
          </button>
        {/each}
      {/if}
    </div>
  </div>

  <!-- Timeline / Activity Section -->
  <aside class="w-full lg:w-80 space-y-8">
    <section>
      <h2 class="text-[10px] font-bold text-on-surface-variant uppercase tracking-[0.2em] mb-4 flex items-center gap-2">
        <span class="material-symbols-outlined text-sm">history</span> Deployment Timeline
      </h2>
      <div class="space-y-4 vessel-scroll max-h-[500px] overflow-y-auto pr-2">
        {#if $diagnostics.length === 0}
          <div class="p-4 rounded-lg border border-outline-variant/10 bg-surface-container-low text-center opacity-60">
            <p class="text-xs text-on-surface-variant">System initialized. Awaiting container events.</p>
          </div>
        {/if}
        {#each $diagnostics.slice(0, 10) as event}
          <div class="relative pl-4 border-l border-outline-variant/20 py-1">
            <div class="absolute -left-1 top-2 w-2 h-2 rounded-full {event.level === 'warn' ? 'bg-error' : 'bg-primary'}"></div>
            <div class="flex items-center justify-between mb-0.5">
              <span class="text-[10px] font-bold text-on-surface capitalize">{event.appId}</span>
              <span class="text-[9px] text-outline-variant font-mono">{event.time}</span>
            </div>
            <p class="text-[11px] text-on-surface-variant line-clamp-1">{event.message}</p>
          </div>
        {/each}
      </div>
    </section>

    <section class="bg-primary/5 rounded-xl p-5 border border-primary/10">
      <h2 class="text-[10px] font-bold text-primary uppercase tracking-[0.2em] mb-3 flex items-center gap-2">
        <span class="material-symbols-outlined text-sm">info</span> System Status
      </h2>
      <div class="space-y-2">
        <div class="flex justify-between items-center">
          <span class="text-xs text-on-surface-variant">Active Containers</span>
          <span class="text-xs font-bold font-mono text-on-surface">{$apps.length}</span>
        </div>
        <div class="flex justify-between items-center">
          <span class="text-xs text-on-surface-variant">Isolated Sessions</span>
          <span class="text-xs font-bold font-mono text-on-surface">{$apps.length}</span>
        </div>
        <div class="mt-4 pt-4 border-t border-outline-variant/10">
          <p class="text-[10px] text-on-surface-variant leading-relaxed opacity-70 italic">Vessel provides true process isolation and memory hibernation for every site deployed above.</p>
        </div>
      </div>
    </section>
  </aside>
</div>
