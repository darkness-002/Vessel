<script lang="ts">
  import { currentView, safeMode, showDiagnostics, showShortcuts, showBrain, setSafeMode, toggleBrain, toggleDiagnostics } from '$lib/stores/uiStore';
  import { visibleTabs, activeTabId, switchToTab, closeTab, addTabForApp, activeApp } from '$lib/stores/tabStore';
  import { unreadCount } from '$lib/stores/notificationStore';
  import { editingApp, initialSettingsState } from '$lib/stores/appStore';

  export let store: any;

  function shortenTabTitle(title: string) {
    return title.length > 20 ? `${title.slice(0, 20)}...` : title;
  }

  async function openNewTabForActiveApp() {
    if ($activeApp) {
      await addTabForApp($activeApp);
    }
  }

  function openCurrentSiteSettings() {
    if ($activeApp) {
      editingApp.set(JSON.parse(JSON.stringify($activeApp)));
      initialSettingsState.set(JSON.stringify($activeApp));
      currentView.set('settings');
    }
  }
</script>

<header class="h-14 w-full flex items-center px-2 sm:px-3 bg-surface-dim sticky top-0 z-40 ml-14 sm:ml-16 justify-between border-b border-outline-variant/10 overflow-hidden">
  {#if $currentView === 'webview'}
    <div class="flex items-center gap-2 flex-1 min-w-0">
      <div class="flex items-center gap-1 overflow-x-auto vessel-scroll pr-2">
        {#each $visibleTabs as tab}
          <div
            role="button"
            tabindex="0"
            on:click={() => switchToTab(tab.id)}
            on:keydown={(e) => (e.key === 'Enter' || e.key === ' ') && switchToTab(tab.id)}
            class="group h-8 min-w-32 max-w-52 px-3 rounded-md border text-xs font-medium flex items-center justify-between gap-2 transition-colors {$activeTabId === tab.id ? 'bg-surface-container text-on-surface border-outline-variant/30' : 'bg-surface-container-lowest text-on-surface-variant border-outline-variant/15 hover:bg-surface-container-high'}"
            title={tab.url}
          >
            <span class="truncate">{shortenTabTitle(tab.title)}</span>
            <button
              type="button"
              class="material-symbols-outlined text-sm opacity-60 group-hover:opacity-100"
              on:click|stopPropagation={() => closeTab(tab.id)}
            >close</button>
          </div>
        {/each}
      </div>
      {#if $activeApp}
        <button
          on:click={openNewTabForActiveApp}
          class="h-8 w-8 rounded-md flex items-center justify-center text-on-surface-variant hover:text-on-surface hover:bg-surface-container-highest transition-colors"
          title="New tab"
          aria-label="New tab"
        >
          <span class="material-symbols-outlined text-[18px]">add</span>
        </button>
      {/if}
    </div>
  {:else}
    <div class="flex items-center space-x-3 flex-1 min-w-0">
      <div class="flex items-center space-x-2 text-on-surface font-bold text-lg mr-2">
        <span class="material-symbols-outlined text-primary-fixed-dim">token</span>
        <span class="hidden sm:inline">Vessel</span>
      </div>
      <div class="relative w-full max-w-xl group hidden md:block">
        <div class="absolute inset-y-0 left-3 flex items-center">
          <span class="material-symbols-outlined text-outline text-sm">search</span>
        </div>
        <input class="w-full h-8 bg-surface-container-lowest border border-outline-variant/20 rounded-lg pl-10 pr-4 text-sm font-label font-medium tracking-wide text-on-surface-variant focus:ring-1 focus:ring-primary-container transition-all" placeholder="vessel://{$currentView}" readonly type="text"/>
      </div>
    </div>
  {/if}

  <div class="flex items-center gap-2 pr-1 sm:pr-2">
    <button
      on:click={() => setSafeMode(!$safeMode, store)} 
      class="p-1.5 rounded-lg transition-colors {$safeMode ? 'text-error hover:bg-error/10' : 'text-on-surface-variant hover:text-on-surface hover:bg-surface-container-highest'}"
      title="Toggle safe mode"
      aria-label="Toggle safe mode"
    >
      <span class="material-symbols-outlined text-[20px]">security</span>
    </button>
    <button
      on:click={toggleDiagnostics}
      class="p-1.5 rounded-lg transition-colors {$showDiagnostics ? 'text-primary' : 'text-on-surface-variant hover:text-on-surface hover:bg-surface-container-highest'}"
      title="Diagnostics"
      aria-label="Diagnostics"
    >
      <span class="material-symbols-outlined text-[20px]">monitoring</span>
    </button>
    <button
      on:click={() => showShortcuts.set(!$showShortcuts)}
      class="p-1.5 rounded-lg transition-colors {$showShortcuts ? 'text-primary' : 'text-on-surface-variant hover:text-on-surface hover:bg-surface-container-highest'}"
      title="Keyboard Shortcuts"
      aria-label="Keyboard Shortcuts"
    >
      <span class="material-symbols-outlined text-[20px]">keyboard</span>
    </button>
    <button on:click={toggleBrain} class="relative p-1.5 rounded-lg hover:bg-surface-container-highest transition-colors {$showBrain ? 'text-primary' : 'text-on-surface-variant hover:text-on-surface'}" title="Notifications" aria-label="Notifications">
      <span class="material-symbols-outlined text-[20px]">notifications</span>
      {#if $unreadCount > 0}
        <span class="absolute top-1 right-1 bg-error w-2 h-2 rounded-full border-2 border-surface-dim"></span>
      {/if}
    </button>
    {#if $currentView === 'webview' && $activeApp}
      <button
        on:click={openCurrentSiteSettings}
        class="p-1.5 rounded-lg text-on-surface-variant hover:text-on-surface hover:bg-surface-container-highest transition-colors"
        title="Site settings"
        aria-label="Site settings"
      >
        <span class="material-symbols-outlined text-[20px]">settings</span>
      </button>
    {/if}
  </div>
</header>
