<script lang="ts">
  import { apps, editingApp, initialSettingsState, isSaving, persistApps, toDisplayName } from '$lib/stores/appStore';
  import { switchView } from '$lib/stores/uiStore';
  import { addTabForApp, switchToTab, tabs } from '$lib/stores/tabStore';
  import { invoke } from '@tauri-apps/api/core';

  export let store: any;
  let settingsSearch = '';

  const cssSnippets = [
    { label: 'Hide Scrollbars', value: '::-webkit-scrollbar { display: none; }' },
    { label: 'OLED Black BG', value: 'body, main, #root { background: #000 !important; }' },
    { label: 'Modern Sans Serif', value: '* { font-family: system-ui, -apple-system, sans-serif !important; }' },
    { label: 'Greyscale Mode', value: 'html { filter: grayscale(100%); }' }
  ];

  function applySnippet(css: string) {
    if (!$editingApp) return;
    $editingApp.features.customCss = ($editingApp.features.customCss + '\n' + css).trim();
  }

  function setProfilePreset(value: string) {
    if (!$editingApp) return;
    $editingApp.features.profile = value;
  }

  async function saveSettings() {
    if (!$editingApp || $isSaving) return;
    isSaving.set(true);
    try {
      const draft = $editingApp;
      const $appsList = [...$apps];
      const index = $appsList.findIndex(a => a.id === draft.id);
      if (index !== -1) {
        $appsList[index] = draft;
        apps.set($appsList);
        tabs.update($tabs => $tabs.map((tab) =>
          tab.appId === draft.id && tab.title === toDisplayName($appsList[index])
            ? { ...tab, title: toDisplayName(draft) }
            : tab
        ));
        await persistApps($appsList, store);
        
        // If active tab is this app, refresh it
        import('$lib/stores/tabStore').then(async m => {
            const $activeTab = (await import('svelte/store')).get(m.activeTab);
            if ($activeTab && $activeTab.appId === draft.id) {
                await m.switchToTab($activeTab.id);
            }
        });

        initialSettingsState.set('');
        switchView('gallery');
      }
    } finally {
      isSaving.set(false);
    }
  }

  async function deleteInstance() {
    if (!$editingApp) return;
    const appIdToDelete = $editingApp.id;
    if (!confirm(`Are you sure you want to discard "${toDisplayName($editingApp)}" and ALL its session data? This cannot be undone.`)) return;

    try {
      // 1. Close any active tabs for this app
      const relatedTabs = $tabs.filter(t => t.appId === appIdToDelete);
      for (const tab of relatedTabs) {
        await invoke('close_webview', { id: tab.id });
      }
      tabs.update($t => $t.filter(t => t.appId !== appIdToDelete));

      // 2. Clear session data from disk
      await invoke('delete_app_session', { appId: appIdToDelete });

      // 3. Remove from apps list and persist
      const updatedApps = $apps.filter(a => a.id !== appIdToDelete);
      apps.set(updatedApps);
      await persistApps(updatedApps, store);

      // 4. Return to gallery
      switchView('gallery');
    } catch (error) {
      console.error('Failed to delete instance', error);
      alert('Failed to delete instance data. See console for details.');
    }
  }

  function launchTest() {
      if ($editingApp) addTabForApp($editingApp);
  }
</script>

{#if $editingApp}
  <div class="max-w-6xl mx-auto flex flex-col bg-surface-container shadow-2xl rounded-xl overflow-hidden border border-outline-variant/15 mt-2 sm:mt-4 relative">
    
    {#if $initialSettingsState && JSON.stringify($editingApp) !== $initialSettingsState}
      <div class="bg-primary/10 border-b border-primary/20 px-6 py-2 flex items-center justify-between animate-in slide-in-from-top-2 duration-300">
        <div class="flex items-center gap-2 text-primary">
          <span class="material-symbols-outlined text-sm">warning</span>
          <span class="text-[10px] font-bold uppercase tracking-widest">You have unsaved changes in this container</span>
        </div>
        <div class="flex items-center gap-4">
          <button on:click={() => editingApp.set(JSON.parse($initialSettingsState))} class="text-[9px] font-bold text-on-surface-variant hover:text-on-surface uppercase tracking-tighter">Reset All</button>
        </div>
      </div>
    {/if}

    <div class="px-4 sm:px-6 lg:px-8 py-4 sm:py-6 flex flex-col sm:flex-row gap-4 sm:items-center sm:justify-between border-b border-outline-variant/10 bg-surface-container-high">
      <div class="flex items-center space-x-4">
        <div class="w-10 h-10 bg-primary-container/20 rounded-lg flex items-center justify-center text-xl font-bold text-primary">
          {$editingApp.icon}
        </div>
        <div>
          <h1 class="text-lg sm:text-xl font-bold font-headline tracking-tight text-on-surface capitalize">{toDisplayName($editingApp)} Settings</h1>
          <p class="text-xs text-on-surface-variant uppercase tracking-widest font-mono">Target: {$editingApp.url}</p>
        </div>
      </div>
      <div class="flex items-center space-x-3">
        <button 
          on:click={launchTest} 
          class="px-4 py-1.5 text-xs font-semibold bg-surface-container-highest rounded border border-outline-variant/20 hover:text-primary transition-colors text-on-surface flex items-center gap-2"
          title="Preview settings in a live window"
        >
          <span class="material-symbols-outlined text-sm">rocket_launch</span>
          Test Settings Live
        </button>
        <div class="h-6 w-px bg-outline-variant/20 mx-1"></div>
        <button on:click={() => switchView('gallery')} class="px-4 py-1.5 text-xs font-semibold bg-surface-container-highest rounded border border-outline-variant/20 hover:bg-surface-bright transition-colors text-on-surface disabled:opacity-30" disabled={$isSaving}>Discard Changes</button>
        <button 
          on:click={saveSettings} 
          class="px-4 py-1.5 text-xs font-semibold bg-primary text-on-primary rounded shadow-lg shadow-primary-container/20 hover:brightness-110 transition-all disabled:opacity-30 flex items-center gap-2"
          disabled={$isSaving}
        >
          {#if $isSaving}
            <span class="material-symbols-outlined animate-spin text-sm">progress_activity</span>
            Saving...
          {:else}
            Save & Deploy
          {/if}
        </button>
      </div>
    </div>

    <div class="flex flex-1 overflow-hidden flex-col lg:flex-row">
      <nav class="w-full lg:w-64 bg-surface-container-low border-r border-outline-variant/10 flex lg:flex-col py-4 lg:py-6 overflow-x-auto">
        <div class="px-6 mb-4">
          <span class="text-[10px] font-bold text-on-surface-variant uppercase tracking-[0.2em] block mb-2">Search Settings</span>
          <div class="relative">
            <input 
              type="text" 
              bind:value={settingsSearch} 
              placeholder="Filter..." 
              class="w-full bg-surface-container-lowest border border-outline-variant/30 rounded px-2 py-1 text-[10px] focus:outline-none focus:ring-1 focus:ring-primary"
            />
          </div>
        </div>

        <div class="px-6 py-3 flex items-center space-x-3 bg-primary/10 text-primary border-r-2 border-primary">
          <span class="material-symbols-outlined text-sm">palette</span>
          <span class="text-sm font-bold">Appearance</span>
        </div>

        {#if $editingApp.url.includes('youtube.com') && (!settingsSearch || 'ad-blocking'.includes(settingsSearch.toLowerCase()))}
          <div class="px-6 py-3 flex items-center space-x-3 text-on-surface-variant hover:bg-surface-container-highest transition-colors">
            <span class="material-symbols-outlined text-sm">shield</span>
            <span class="text-sm font-medium">Ad-Blocking</span>
          </div>
        {/if}

        <div class="mt-auto px-6 py-8 border-t border-outline-variant/10">
          <button 
            on:click={deleteInstance}
            class="w-full py-2 bg-error/10 text-error hover:bg-error/20 rounded border border-error/30 transition-all font-label text-[10px] font-bold uppercase tracking-widest flex items-center justify-center gap-2"
          >
            <span class="material-symbols-outlined text-sm">delete_forever</span> Discard Instance
          </button>
          <p class="text-[9px] text-outline-variant mt-3 italic leading-tight">Removes instance from gallery and wipes all cookies/data from disk.</p>
        </div>
      </nav>
      
      <div class="flex-1 overflow-y-auto vessel-scroll bg-surface p-4 sm:p-6 lg:p-10 space-y-8 sm:space-y-12">
        <section class="animate-in fade-in slide-in-from-bottom-2 duration-300">
          <div class="flex items-center justify-between mb-8 border-b border-outline-variant/10 pb-4">
            <div class="flex items-center gap-3">
              <span class="material-symbols-outlined text-primary">palette</span>
              <h2 class="text-xl sm:text-2xl font-bold tracking-tight text-on-surface">Appearance Settings</h2>
            </div>
            <div class="flex items-center gap-2 px-3 py-1 bg-surface-container-highest rounded-full border border-outline-variant/20">
              <span class="w-2 h-2 rounded-full bg-primary animate-pulse"></span>
              <span class="text-[9px] font-bold text-on-surface-variant uppercase tracking-widest">Live Engine</span>
            </div>
          </div>
          
          <div class="space-y-6">
            <!-- Session Profile -->
            <div class="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4 p-4 sm:p-6 rounded-lg bg-surface-container-high border border-outline-variant/10 group hover:border-primary/20 transition-all">
              <div class="flex-1">
                <div class="flex items-center gap-2">
                  <h3 class="font-bold text-on-surface">Session Profile</h3>
                  {#if JSON.parse($initialSettingsState).features.profile !== $editingApp.features.profile}
                    <span class="w-1.5 h-1.5 rounded-full bg-primary animate-in zoom-in" title="Modified"></span>
                  {/if}
                </div>
                <p class="text-sm text-on-surface-variant">Isolated cookie jar for this instance. Login state is never shared.</p>
              </div>
              <div class="w-full sm:w-72 space-y-2">
                <div class="relative">
                  <span class="absolute left-3 top-1/2 -translate-y-1/2 material-symbols-outlined text-sm text-outline">account_circle</span>
                  <input
                    type="text"
                    bind:value={$editingApp.features.profile}
                    placeholder="default"
                    class="bg-surface-container-lowest border border-outline-variant/30 rounded-lg pl-9 pr-3 py-2 text-sm text-on-surface focus:outline-none focus:ring-1 focus:ring-primary w-full shadow-inner"
                  />
                </div>
                <div class="flex gap-1.5">
                  <button type="button" on:click={() => setProfilePreset('default')} class="text-[9px] font-bold uppercase tracking-widest px-2 py-1 rounded bg-surface-container-highest border border-outline-variant/20 text-on-surface-variant hover:text-primary hover:bg-primary/5 transition-all">Default</button>
                  <button type="button" on:click={() => setProfilePreset('work')} class="text-[9px] font-bold uppercase tracking-widest px-2 py-1 rounded bg-surface-container-highest border border-outline-variant/20 text-on-surface-variant hover:text-primary hover:bg-primary/5 transition-all">Work</button>
                  <button type="button" on:click={() => setProfilePreset('personal')} class="text-[9px] font-bold uppercase tracking-widest px-2 py-1 rounded bg-surface-container-highest border border-outline-variant/20 text-on-surface-variant hover:text-primary hover:bg-primary/5 transition-all">Personal</button>
                </div>
              </div>
            </div>

            <!-- Sleep Delay -->
            <div class="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4 p-4 sm:p-6 rounded-lg bg-surface-container-high border border-outline-variant/10 group hover:border-primary/20 transition-all">
              <div class="flex-1">
                <div class="flex items-center gap-2">
                  <h3 class="font-bold text-on-surface">Background Sleep Delay</h3>
                  {#if JSON.parse($initialSettingsState).features.idleSleepSeconds !== $editingApp.features.idleSleepSeconds}
                    <span class="w-1.5 h-1.5 rounded-full bg-primary animate-in zoom-in"></span>
                  {/if}
                </div>
                <p class="text-sm text-on-surface-variant">Delay hibernation to save RAM. 0s = Instant freeze.</p>
              </div>
              <div class="w-full sm:w-72 space-y-3">
                <div class="flex items-center gap-4">
                  <div class="flex-1 relative pt-4 pb-2">
                    <div class="flex justify-between text-[8px] text-outline font-bold uppercase absolute top-0 w-full px-1">
                      <span>0s</span>
                      <span>60s</span>
                      <span>120s</span>
                    </div>
                    <input type="range" min="0" max="120" step="5" bind:value={$editingApp.features.idleSleepSeconds} class="w-full accent-primary h-1.5 rounded-lg bg-surface-container-lowest cursor-pointer" />
                  </div>
                  <div class="flex items-center gap-1 bg-surface-container-lowest px-3 py-1.5 rounded border border-outline-variant/30 min-w-[70px] justify-center shadow-inner">
                    <input type="number" bind:value={$editingApp.features.idleSleepSeconds} class="w-8 bg-transparent text-xs font-mono font-bold text-primary focus:outline-none text-center" />
                    <span class="text-[10px] text-outline-variant font-bold">SEC</span>
                  </div>
                </div>
                <p class="text-[9px] text-on-surface-variant italic opacity-70">Lower values improve system performance but may delay re-activation.</p>
              </div>
            </div>

            <!-- Theme Selection -->
            <div class="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4 p-4 sm:p-6 rounded-lg bg-surface-container-high border border-outline-variant/10 group hover:border-primary/20 transition-all">
              <div class="flex-1">
                <div class="flex items-center gap-2">
                  <h3 class="font-bold text-on-surface">Visual Injection Palette</h3>
                  {#if JSON.parse($initialSettingsState).features.theme !== $editingApp.features.theme}
                    <span class="w-1.5 h-1.5 rounded-full bg-primary animate-in zoom-in"></span>
                  {/if}
                </div>
                <p class="text-sm text-on-surface-variant">Override site colors with high-contrast native palettes.</p>
              </div>
              <div class="relative w-full sm:w-64">
                <select bind:value={$editingApp.features.theme} class="appearance-none bg-surface-container-lowest border border-outline-variant/30 rounded-lg p-2.5 pr-10 text-sm text-on-surface focus:outline-none focus:ring-1 focus:ring-primary w-full shadow-inner">
                  <option value="default">None (Default Rendering)</option>
                  <option value="dark_invert">Force Native Dark</option>
                  <option value="oled">Total OLED Black (#000)</option>
                </select>
                <span class="material-symbols-outlined absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none text-outline">expand_more</span>
              </div>
            </div>

            {#if $editingApp.url.includes("youtube.com")}
              <div class="flex items-center justify-between p-4 sm:p-6 rounded-lg bg-surface-container-high border border-outline-variant/10 group hover:border-primary/20 transition-all">
                <div>
                  <div class="flex items-center gap-2">
                    <h3 class="font-bold text-on-surface">YouTube Shield (Ad-Block)</h3>
                    {#if JSON.parse($initialSettingsState).features.adblock !== $editingApp.features.adblock}
                      <span class="w-1.5 h-1.5 rounded-full bg-primary animate-in zoom-in"></span>
                    {/if}
                  </div>
                  <p class="text-sm text-on-surface-variant">Intercept pre-roll ads and skip promotional overlays.</p>
                </div>
                <label class="relative inline-flex items-center cursor-pointer">
                  <input type="checkbox" bind:checked={$editingApp.features.adblock} class="sr-only peer">
                  <div class="w-11 h-6 bg-surface-container-highest peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary"></div>
                </label>
              </div>
            {/if}

            <!-- Custom CSS Editor -->
            <div class="p-4 sm:p-6 rounded-lg bg-surface-container-high border border-outline-variant/10 group hover:border-primary/20 transition-all">
              <div class="mb-5 flex flex-col lg:flex-row lg:items-center justify-between gap-4">
                <div>
                  <div class="flex items-center gap-2">
                    <h3 class="font-bold text-on-surface">Custom CSS Injection</h3>
                    {#if JSON.parse($initialSettingsState).features.customCss !== $editingApp.features.customCss}
                      <span class="w-1.5 h-1.5 rounded-full bg-primary animate-in zoom-in"></span>
                    {/if}
                  </div>
                  <p class="text-sm text-on-surface-variant">Inject site-specific CSS at runtime.</p>
                </div>
                <div class="flex flex-wrap items-center gap-2">
                  <span class="text-[9px] text-outline font-bold uppercase tracking-widest mr-1">Insert Snippet:</span>
                  {#each cssSnippets as snippet}
                    <button 
                      on:click={() => applySnippet(snippet.value)}
                      class="px-2.5 py-1 rounded-md bg-surface-container-highest text-[10px] font-bold text-on-surface-variant hover:text-primary hover:bg-primary/10 transition-all border border-outline-variant/10 flex items-center gap-1"
                    >
                      <span class="material-symbols-outlined text-[10px]">add</span> {snippet.label}
                    </button>
                  {/each}
                </div>
              </div>
              <div class="relative rounded-xl overflow-hidden border border-outline-variant/30 shadow-2xl">
                <div class="bg-surface-container-highest px-4 py-2 flex items-center justify-between border-b border-outline-variant/20">
                  <div class="flex gap-1.5">
                    <div class="w-2.5 h-2.5 rounded-full bg-error/40"></div>
                    <div class="w-2.5 h-2.5 rounded-full bg-primary/40"></div>
                    <div class="w-2.5 h-2.5 rounded-full bg-primary/20"></div>
                  </div>
                  <span class="text-[9px] font-mono text-outline uppercase font-bold tracking-widest">vessel_style.css</span>
                </div>
                <textarea
                  bind:value={$editingApp.features.customCss}
                  rows="10"
                  spellcheck="false"
                  placeholder={"/* Add your custom styles here */\\n.header { display: none !important; }"}
                  class="w-full bg-surface-container-lowest p-5 text-xs sm:text-sm text-on-surface font-mono focus:outline-none leading-relaxed resize-y min-h-[200px]"
                ></textarea>
              </div>
            </div>

            <!-- Custom JS Editor -->
            <div class="p-4 sm:p-6 rounded-lg bg-surface-container-high border border-outline-variant/10 group hover:border-primary/20 transition-all">
              <div class="mb-5">
                <div class="flex items-center gap-2">
                  <h3 class="font-bold text-on-surface">Custom JavaScript Injection</h3>
                  {#if JSON.parse($initialSettingsState).features.customJs !== $editingApp.features.customJs}
                    <span class="w-1.5 h-1.5 rounded-full bg-primary animate-in zoom-in"></span>
                  {/if}
                </div>
                <p class="text-sm text-on-surface-variant">Automate DOM actions or remove interactive elements.</p>
              </div>
              <div class="relative rounded-xl overflow-hidden border border-outline-variant/30 shadow-2xl">
                <div class="bg-surface-container-highest px-4 py-2 flex items-center justify-between border-b border-outline-variant/20">
                  <div class="flex gap-1.5">
                    <div class="w-2.5 h-2.5 rounded-full bg-error/40"></div>
                    <div class="w-2.5 h-2.5 rounded-full bg-primary/40"></div>
                    <div class="w-2.5 h-2.5 rounded-full bg-primary/20"></div>
                  </div>
                  <span class="text-[9px] font-mono text-outline uppercase font-bold tracking-widest">vessel_script.js</span>
                </div>
                <textarea
                  bind:value={$editingApp.features.customJs}
                  rows="10"
                  spellcheck="false"
                  placeholder="// Run automation scripts\nconsole.log('Vessel container active');"
                  class="w-full bg-surface-container-lowest p-5 text-xs sm:text-sm text-on-surface font-mono focus:outline-none leading-relaxed resize-y min-h-[200px]"
                ></textarea>
              </div>
            </div>

            <!-- Domain Allowlist -->
            <div class="p-4 sm:p-6 rounded-lg bg-surface-container-high border border-outline-variant/10 group hover:border-primary/20 transition-all">
              <div class="mb-5">
                <div class="flex items-center gap-2">
                  <h3 class="font-bold text-on-surface">Injection Domain Allowlist</h3>
                  {#if JSON.parse($initialSettingsState).features.injectionAllowlist !== $editingApp.features.injectionAllowlist}
                    <span class="w-1.5 h-1.5 rounded-full bg-primary animate-in zoom-in"></span>
                  {/if}
                </div>
                <p class="text-sm text-on-surface-variant">Restrict code injection to specific subdomains. Separate by comma.</p>
              </div>
              <textarea
                bind:value={$editingApp.features.injectionAllowlist}
                rows="3"
                spellcheck="false"
                placeholder="example.com, auth.example.com"
                class="w-full bg-surface-container-lowest border border-outline-variant/30 rounded-lg p-4 text-xs sm:text-sm text-on-surface font-mono focus:outline-none focus:ring-1 focus:ring-primary shadow-inner"
              ></textarea>
            </div>
          </div>
        </section>
      </div>
    </div>
  </div>
{/if}
