<script lang="ts">
  import { apps, toDisplayName, editingApp, initialSettingsState } from '$lib/stores/appStore';
  import { activeId, currentView, switchView } from '$lib/stores/uiStore';
  import { addTabForApp, switchToTab, tabs } from '$lib/stores/tabStore';
  import { goto } from '$app/navigation';

  async function launch(app: any) {
    const existingTab = $tabs.find((tab) => tab.appId === app.id);
    if (existingTab) {
      await switchToTab(existingTab.id);
      return;
    }
    await addTabForApp(app);
  }

  async function goHome() {
    await goto('/');
    activeId.set('');
    switchView('gallery');
  }

  function openSettings(app: any) {
    editingApp.set(JSON.parse(JSON.stringify(app)));
    initialSettingsState.set(JSON.stringify(app));
    switchView('settings');
  }
</script>

<aside class="fixed left-0 h-full w-14 sm:w-16 flex flex-col items-center py-3 sm:py-4 bg-surface-container-lowest z-[60] shadow-none border-r border-outline-variant/10">
  <div class="mb-8" data-tauri-drag-region>
    <button
      on:click={goHome}
      class="text-primary font-black text-xl cursor-pointer h-8 w-8 rounded-md hover:bg-surface-container-highest transition-colors"
      title="Go to home"
      aria-label="Go to home"
    >
      V
    </button>
  </div>
  
  <nav class="flex flex-col items-center space-y-6 w-full vessel-scroll overflow-y-auto flex-1">
    {#each $apps as app}
      <div class="relative group w-full flex justify-center">
        <button 
          on:click={() => launch(app)} 
          on:contextmenu|preventDefault={() => openSettings(app)} 
          class="flex flex-col items-center transition-all duration-200 scale-95 active:scale-90 {$activeId === app.id && $currentView === 'webview' ? 'text-primary border-l-2 border-primary-container w-full' : 'text-outline hover:text-on-surface w-full'}"
          title={toDisplayName(app)}
          aria-label="Launch {toDisplayName(app)}"
        >
          <div class="w-10 h-10 flex items-center justify-center rounded-lg {$activeId === app.id && $currentView === 'webview' ? 'bg-surface-variant' : 'group-hover:bg-surface-container-highest'} transition-colors text-xl font-bold">
            {app.icon}
          </div>
        </button>
      </div>
    {/each}
  </nav>

  <div class="w-full flex flex-col items-center space-y-4 px-2 mt-auto pt-4 border-t border-outline-variant/10">
    <div class="h-px w-8 bg-outline-variant opacity-20"></div>
    
    <button on:click={() => switchView('add-app')} class="flex flex-col items-center space-y-1 text-outline hover:text-on-surface transition-all duration-200 scale-95 active:scale-90 group w-full">
      <div class="w-10 h-10 flex items-center justify-center rounded-lg {$currentView === 'add-app' ? 'bg-primary/10 text-primary' : 'group-hover:bg-surface-container-highest'} transition-colors">
        <span class="material-symbols-outlined">add</span>
      </div>
      <span class="font-label tracking-tight text-[9px] uppercase">Add Site</span>
    </button>
  </div>
</aside>
