<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event"; 
  import { load } from "@tauri-apps/plugin-store";
  import { onMount } from "svelte";

  // Core State
  let apps: any[] = [];
  let store: any;
  let activeId = '';
  let currentView: 'gallery' | 'webview' | 'settings' = 'gallery';
  let editingApp: any = null;
  
  // Overlay State
  let showBrain = false;
  let notifications: any[] = [];
  let unreadCount = 0;

  // Add App Form State
  let newAppName = ''; let newAppUrl = ''; let newAppIcon = '';

  const themes: Record<string, string> = {
    default: "",
    dark_invert: "body { filter: invert(1) hue-rotate(180deg); background: black !important; }",
    oled: "body { background-color: #000000 !important; color: #ffffff !important; }"
  };

  onMount(async () => {
    store = await load('vessel_settings.json', { autoSave: true });
    const savedApps = await store.get('apps');
    apps = savedApps || [];
    if (!savedApps) await store.set('apps', apps);

    listen("vessel-notification", (event: any) => {
      notifications = [event.payload, ...notifications];
      if (!showBrain) unreadCount++;
    });
  });

  async function launch(app: any) {
    activeId = app.id;
    currentView = 'webview';
    let jsToInject = app.features?.adblock && app.url.includes("youtube.com") 
      ? `setInterval(() => { const skipBtn = document.querySelector('.ytp-skip-ad-button'); if(skipBtn) skipBtn.click(); }, 500);` 
      : "";
    const cssToInject = themes[app.features?.theme || 'default'];
    await invoke("open_app", { id: app.id, url: app.url, css: cssToInject, js: jsToInject });
  }

  function switchView(view: 'gallery' | 'settings') {
    currentView = view;
    invoke("hide_all_webviews");
  }

  function toggleBrain() {
    showBrain = !showBrain;
    if (showBrain) unreadCount = 0;
  }

  function openSettings(app: any) {
    editingApp = JSON.parse(JSON.stringify(app));
    switchView('settings');
  }

  async function saveSettings() {
    const index = apps.findIndex(a => a.id === editingApp.id);
    if (index !== -1) {
      apps[index] = editingApp;
      await store.set('apps', apps);
      if (activeId === editingApp.id) launch(editingApp);
      else switchView('gallery');
    }
  }

  async function addNewApp() {
    if (!newAppName || !newAppUrl) return;
    const newApp = { 
      id: newAppName.toLowerCase().replace(/\s+/g, '-'), 
      icon: newAppIcon || newAppName.charAt(0).toUpperCase(), 
      url: newAppUrl, 
      features: { theme: 'default', adblock: false } 
    };
    apps = [...apps, newApp]; 
    await store.set('apps', apps);
    newAppName = ''; newAppUrl = ''; newAppIcon = ''; 
  }
</script>

<div class="bg-background text-on-background font-body select-none overflow-hidden h-screen w-screen flex flex-col">
  
  <aside class="fixed left-0 h-full w-16 flex flex-col items-center py-4 bg-surface-container-lowest z-[60] shadow-none">
    <div class="mb-8" data-tauri-drag-region>
      <span class="text-primary font-black text-xl cursor-grab">V</span>
    </div>
    
    <nav class="flex flex-col items-center space-y-6 w-full vessel-scroll overflow-y-auto">
      {#each apps as app}
        <div class="relative group w-full flex justify-center">
          <button on:click={() => launch(app)} on:contextmenu|preventDefault={() => openSettings(app)} class="flex flex-col items-center space-y-1 transition-all duration-200 scale-95 active:scale-90 {activeId === app.id && currentView === 'webview' ? 'text-primary border-l-2 border-primary-container w-full' : 'text-outline hover:text-on-surface w-full'}">
            <div class="w-10 h-10 flex items-center justify-center rounded-lg {activeId === app.id && currentView === 'webview' ? 'bg-surface-variant' : 'group-hover:bg-surface-container-highest'} transition-colors text-xl font-bold">
              {app.icon}
            </div>
            <span class="font-label tracking-tight text-[9px] uppercase truncate px-1 max-w-full">{app.id}</span>
          </button>
        </div>
      {/each}
      
      <div class="h-px w-8 bg-outline-variant opacity-20 my-2"></div>
      
      <button on:click={() => switchView('gallery')} class="flex flex-col items-center space-y-1 text-outline hover:text-on-surface transition-all duration-200 scale-95 active:scale-90 group">
        <div class="w-10 h-10 flex items-center justify-center rounded-lg group-hover:bg-surface-container-highest transition-colors">
          <span class="material-symbols-outlined">add</span>
        </div>
        <span class="font-label tracking-tight text-[9px] uppercase">Add Site</span>
      </button>
    </nav>
  </aside>

  <header data-tauri-drag-region class="h-12 w-full flex items-center px-4 bg-surface-dim sticky top-0 z-40 ml-16 justify-between border-none cursor-grab">
    <div class="flex items-center space-x-4 flex-1 max-w-2xl pointer-events-none">
      <div class="flex items-center space-x-2 text-on-surface font-bold text-lg mr-4">
        <span class="material-symbols-outlined text-primary-fixed-dim">token</span>
        <span>Vessel</span>
      </div>
      <div class="relative w-full group">
        <div class="absolute inset-y-0 left-3 flex items-center">
          <span class="material-symbols-outlined text-outline text-sm">search</span>
        </div>
        <input class="w-full h-8 bg-surface-container-lowest border-none rounded-lg pl-10 pr-4 text-sm font-label font-medium tracking-wide text-on-surface-variant focus:ring-1 focus:ring-primary-container transition-all" placeholder="vessel://{currentView}" readonly type="text"/>
      </div>
    </div>
    
    <div class="flex items-center space-x-4 mr-16"> <div class="flex items-center space-x-3">
        <button on:click={toggleBrain} class="relative p-1.5 rounded-lg hover:bg-surface-container-highest transition-colors {showBrain ? 'text-primary' : 'text-on-surface-variant hover:text-on-surface'}">
          <span class="material-symbols-outlined text-[20px]">notifications</span>
          {#if unreadCount > 0}
            <span class="absolute top-1 right-1 bg-error w-2 h-2 rounded-full border-2 border-surface-dim"></span>
          {/if}
        </button>
      </div>
    </div>
  </header>

  <main class="ml-16 mb-6 p-8 h-[calc(100vh-72px)] overflow-y-auto scrollbar-hide relative bg-background">
    
    {#if currentView === 'webview'}
      <div class="w-full h-full flex flex-col items-center justify-center text-outline-variant opacity-30">
        <span class="material-symbols-outlined text-6xl mb-4">memory</span>
        <span class="font-headline tracking-widest uppercase">Native Container Active</span>
      </div>
    {/if}

    {#if currentView === 'gallery'}
      <header class="mb-12">
        <h1 class="font-headline font-extrabold text-5xl tracking-tight text-on-surface mb-2">Workspace Gallery</h1>
        <p class="text-on-surface-variant font-body max-w-xl text-lg">Extend your workspace with new instances and toolsets.</p>
      </header>

      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-6">
        <div class="col-span-1 md:col-span-2 row-span-2 relative overflow-hidden rounded-xl bg-surface-container-lowest border border-outline-variant/10 p-8 flex flex-col justify-between">
          <div>
            <div class="flex items-center space-x-2 mb-4">
              <span class="material-symbols-outlined text-primary-fixed-dim" style="font-variation-settings: 'FILL' 1;">add_circle</span>
              <span class="font-label text-sm font-bold text-on-surface uppercase tracking-widest">New Deployment</span>
            </div>
            <h2 class="font-headline font-bold text-3xl text-on-surface mb-2">Create Custom Instance</h2>
            <p class="text-on-surface-variant text-base mb-6 max-w-sm">Deploy any web application into an isolated, optimized Vessel container.</p>
            
            <div class="space-y-4 max-w-md">
              <div>
                <label class="text-[10px] text-outline uppercase tracking-widest block mb-1">Instance Name</label>
                <input type="text" bind:value={newAppName} placeholder="e.g. ChatGPT" class="w-full bg-surface-container border border-outline-variant/30 rounded-lg p-3 text-sm text-on-surface focus:outline-none focus:ring-1 focus:ring-primary"/>
              </div>
              <div>
                <label class="text-[10px] text-outline uppercase tracking-widest block mb-1">Target URL</label>
                <input type="url" bind:value={newAppUrl} placeholder="https://chatgpt.com" class="w-full bg-surface-container border border-outline-variant/30 rounded-lg p-3 text-sm text-on-surface focus:outline-none focus:ring-1 focus:ring-primary"/>
              </div>
              <div>
                <label class="text-[10px] text-outline uppercase tracking-widest block mb-1">Icon Character</label>
                <input type="text" bind:value={newAppIcon} maxlength="1" placeholder="C" class="w-full bg-surface-container border border-outline-variant/30 rounded-lg p-3 text-sm text-on-surface text-center focus:outline-none focus:ring-1 focus:ring-primary"/>
              </div>
            </div>
          </div>
          <div class="mt-8 flex items-center space-x-4">
            <button on:click={addNewApp} class="px-6 py-2.5 primary-gradient text-on-primary font-label font-bold rounded-sm text-sm hover:brightness-110">Deploy Container</button>
          </div>
        </div>

        {#each apps as app}
          <div class="bg-surface-container rounded-xl p-6 border border-outline-variant/10 flex flex-col justify-between hover:bg-surface-container-high transition-colors">
            <div>
              <div class="w-12 h-12 bg-surface-container-lowest rounded-lg flex items-center justify-center mb-6 ring-1 ring-outline-variant/20 text-2xl font-bold text-on-surface">
                {app.icon}
              </div>
              <div class="flex justify-between items-start mb-2">
                <h3 class="font-headline font-bold text-lg text-on-surface capitalize">{app.id}</h3>
                <span class="text-on-surface-variant text-xs font-mono">ACTIVE</span>
              </div>
              <p class="text-on-surface-variant text-sm mb-4 truncate">{app.url}</p>
            </div>
            <div class="flex items-center justify-end mt-4 border-t border-outline-variant/10 pt-4">
              <button on:click={() => openSettings(app)} class="text-primary font-label text-xs font-bold tracking-widest uppercase flex items-center gap-1 hover:brightness-125">
                <span class="material-symbols-outlined text-sm">tune</span> Config
              </button>
            </div>
          </div>
        {/each}
      </div>
    {/if}

    {#if currentView === 'settings' && editingApp}
      <div class="max-w-6xl mx-auto flex flex-col bg-surface-container shadow-2xl rounded-xl overflow-hidden border border-outline-variant/15 mt-4">
        <div class="px-8 py-6 flex items-center justify-between border-b border-outline-variant/10 bg-surface-container-high">
          <div class="flex items-center space-x-4">
            <div class="w-10 h-10 bg-primary-container/20 rounded-lg flex items-center justify-center text-xl font-bold text-primary">
              {editingApp.icon}
            </div>
            <div>
              <h1 class="text-xl font-bold font-headline tracking-tight text-on-surface capitalize">{editingApp.id} Settings</h1>
              <p class="text-xs text-on-surface-variant uppercase tracking-widest font-mono">Target: {editingApp.url}</p>
            </div>
          </div>
          <div class="flex items-center space-x-3">
            <button on:click={() => switchView('gallery')} class="px-4 py-1.5 text-xs font-semibold bg-surface-container-highest rounded border border-outline-variant/20 hover:bg-surface-bright transition-colors text-on-surface">Discard</button>
            <button on:click={saveSettings} class="px-4 py-1.5 text-xs font-semibold bg-gradient-to-br from-primary to-primary-container text-on-primary rounded shadow-lg shadow-primary-container/20 hover:opacity-90 transition-opacity">Save & Deploy</button>
          </div>
        </div>

        <div class="flex flex-1 overflow-hidden">
          <nav class="w-64 bg-surface-container-low border-r border-outline-variant/10 flex flex-col py-6">
            <div class="px-6 mb-4"><span class="text-[10px] font-bold text-on-surface-variant uppercase tracking-[0.2em]">Categories</span></div>
            <div class="px-6 py-3 flex items-center space-x-3 bg-primary/10 text-primary border-r-2 border-primary">
              <span class="material-symbols-outlined text-sm">palette</span>
              <span class="text-sm font-bold">Appearance</span>
            </div>
            {#if editingApp.url.includes('youtube.com')}
              <div class="px-6 py-3 flex items-center space-x-3 text-on-surface-variant hover:bg-surface-container-highest transition-colors">
                <span class="material-symbols-outlined text-sm">shield</span>
                <span class="text-sm font-medium">Ad-Blocking</span>
              </div>
            {/if}
          </nav>
          
          <div class="flex-1 overflow-y-auto vessel-scroll bg-surface p-10 space-y-12">
            <section>
              <div class="flex items-center justify-between mb-8">
                <h2 class="text-2xl font-bold tracking-tight text-on-surface">Appearance</h2>
              </div>
              <div class="space-y-8">
                <div class="flex items-center justify-between p-6 rounded-lg bg-surface-container-high border border-outline-variant/10">
                  <div>
                    <h3 class="font-bold text-on-surface">Visual Injection Engine</h3>
                    <p class="text-sm text-on-surface-variant">Override default site styles with native UI palettes.</p>
                  </div>
                  <select bind:value={editingApp.features.theme} class="bg-surface-container-lowest border border-outline-variant/30 rounded-lg p-2 text-sm text-on-surface focus:outline-none focus:border-primary w-48">
                    <option value="default">None (Default)</option>
                    <option value="dark_invert">Force Dark Mode</option>
                    <option value="oled">True OLED Black</option>
                  </select>
                </div>

                {#if editingApp.url.includes("youtube.com")}
                  <div class="flex items-center justify-between p-6 rounded-lg bg-surface-container-high border border-outline-variant/10">
                    <div>
                      <h3 class="font-bold text-on-surface">Ad-blocking Engine</h3>
                      <p class="text-sm text-on-surface-variant">Intercept and nullify script-based advertisement injections.</p>
                    </div>
                    <input type="checkbox" bind:checked={editingApp.features.adblock} class="w-5 h-5 accent-primary bg-surface-container-lowest border-outline-variant/30 rounded"/>
                  </div>
                {/if}
              </div>
            </section>
          </div>
        </div>
      </div>
    {/if}
  </main>

  {#if showBrain}
    <div class="fixed top-12 right-0 bottom-6 w-[400px] glass-panel z-[70] border-l border-outline-variant/15 flex flex-col shadow-2xl shadow-primary/5 transition-transform duration-300">
      <div class="p-6 flex items-center justify-between">
        <div>
          <h2 class="text-2xl font-black tracking-tight text-on-surface">Global Brain</h2>
          <p class="text-[10px] font-mono text-outline uppercase mt-1">Cross-Platform Sync Active</p>
        </div>
        <button on:click={() => notifications = []} class="bg-surface-container-highest hover:bg-surface-container-high text-on-surface-variant px-3 py-1.5 rounded-sm text-[10px] font-bold uppercase tracking-widest transition-all border border-outline-variant/20">
          Clear
        </button>
      </div>
      
      <div class="flex-grow overflow-y-auto vessel-scroll px-6 space-y-4 pb-6">
        {#if notifications.length === 0}
          <div class="text-center text-outline-variant mt-20 text-sm">Awaiting neural input streams.</div>
        {/if}
        {#each notifications as note}
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

  <footer class="bg-surface-container-lowest fixed bottom-0 left-16 right-0 h-6 flex items-center px-4 z-50 justify-between shadow-none border-t border-outline-variant/10">
    <div class="flex items-center space-x-6">
      <div class="flex items-center space-x-2 text-on-surface">
        <span class="material-symbols-outlined text-[12px] text-primary">memory</span>
        <span class="font-mono text-[10px] uppercase tracking-widest">CPU 1.2%</span>
      </div>
      <div class="flex items-center space-x-2 text-outline">
        <span class="material-symbols-outlined text-[12px]">speed</span>
        <span class="font-mono text-[10px] uppercase tracking-widest">RAM 140MB</span>
      </div>
    </div>
    <div class="flex items-center space-x-2 text-primary">
      <span class="material-symbols-outlined text-[12px]" style="font-variation-settings: 'FILL' 1;">verified_user</span>
      <span class="font-mono text-[10px] uppercase tracking-widest">Secure Tunnel: Active</span>
    </div>
  </footer>

</div>