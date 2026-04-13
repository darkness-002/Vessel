<script lang="ts">
  import { goto } from "$app/navigation";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event"; 
  import { load } from "@tauri-apps/plugin-store";
  import { onMount } from "svelte";

  type AppView = 'gallery' | 'webview' | 'settings';

  type AppConfig = {
    id: string;
    name?: string;
    icon: string;
    url: string;
    features: {
      theme: keyof typeof themes;
      adblock: boolean;
      profile: string;
      customCss: string;
      idleSleepSeconds: number;
    };
  };

  type VesselNotification = {
    appId: string;
    title: string;
    body: string;
    time: string;
  };

  type BrowserTab = {
    id: string;
    appId: string;
    title: string;
    url: string;
  };

  type NewTabEvent = {
    appId: string;
    url: string;
    title?: string | null;
  };

  type ResourceUsage = {
    cpuPercent: number;
    ramMb: number;
  };

  // Core State
  let apps: AppConfig[] = [];
  let store: Awaited<ReturnType<typeof load>>;
  let activeId = '';
  let currentView: AppView = 'gallery';
  let storeReady = false;
  let editingApp: AppConfig | null = null;
  let formError = '';
  let tabs: BrowserTab[] = [];
  let activeTabId = '';
  let activeTab: BrowserTab | null = null;
  let activeApp: AppConfig | null = null;
  let tabSeed = 0;
  let viewportWidth = 0;
  let viewportHeight = 0;
  
  // Overlay State
  let showBrain = false;
  let notifications: VesselNotification[] = [];
  let unreadCount = 0;
  let brainSearch = '';
  let brainAppFilter = 'all';
  let showPerformanceStats = false;
  let usage: ResourceUsage = { cpuPercent: 0, ramMb: 0 };
  let usageTimer: ReturnType<typeof setInterval> | null = null;

  // Add App Form State
  let newAppName = ''; let newAppUrl = ''; let newAppIcon = '';

  const themes: Record<string, string> = {
    default: "",
    dark_invert: "body { filter: invert(1) hue-rotate(180deg); background: black !important; }",
    oled: "body { background-color: #000000 !important; color: #ffffff !important; }"
  };

  function updateViewport() {
    viewportWidth = window.innerWidth;
    viewportHeight = window.innerHeight;
  }

  function ensureProtocol(url: string) {
    return /^https?:\/\//i.test(url) ? url : `https://${url}`;
  }

  function normalizeServiceUrl(url: string) {
    try {
      const parsed = new URL(ensureProtocol(url));
      const host = parsed.hostname.toLowerCase();
      if (host === 'spotify.com' || host === 'www.spotify.com') {
        return 'https://open.spotify.com/';
      }
      return parsed.toString();
    } catch {
      return ensureProtocol(url);
    }
  }

  function getSiteOptimizations(targetUrl: string) {
    const url = targetUrl.toLowerCase();

    if (url.includes('open.spotify.com') || url.includes('spotify.com')) {
      return {
        css: `
          .Root__top-container,
          .Root__nav-bar,
          .main-globalNav-searchContainer,
          .main-home-filterChipsSection {
            backdrop-filter: none !important;
          }

          [data-testid="context-menu"],
          [data-testid="yolo-highlight-snippet"],
          .encore-light-theme,
          #onetrust-banner-sdk,
          #onetrust-consent-sdk {
            z-index: 1 !important;
          }

          .main-nowPlayingBar-nowPlayingBar {
            contain: layout style paint;
          }
        `,
        js: `
          (() => {
            const dismissSpotifyNoise = () => {
              const selectors = [
                '#onetrust-accept-btn-handler',
                'button[data-testid="cookie-banner-accept-button"]',
                'button[aria-label="Close"]'
              ];
              for (const selector of selectors) {
                const button = document.querySelector(selector);
                if (button) {
                  try { button.click(); } catch {}
                }
              }
            };

            dismissSpotifyNoise();
            setInterval(dismissSpotifyNoise, 3000);
          })();
        `
      };
    }

    return { css: '', js: '' };
  }

  function normalizeApp(input: Partial<AppConfig> & { id: string; icon: string; url: string }): AppConfig {
    const normalizedUrl = normalizeServiceUrl(input.url);
    return {
      id: input.id,
      name: input.name,
      icon: input.icon,
      url: normalizedUrl,
      features: {
        theme: input.features?.theme || 'default',
        adblock: Boolean(input.features?.adblock),
        profile: input.features?.profile || 'default',
        customCss: input.features?.customCss || '',
        idleSleepSeconds: Number(input.features?.idleSleepSeconds ?? 15)
      }
    };
  }

  function nextTabId(appId: string) {
    tabSeed += 1;
    return `${appId}__tab_${Date.now()}_${tabSeed}`;
  }

  $: activeTab = tabs.find((tab) => tab.id === activeTabId) || null;
  $: activeApp = activeTab ? apps.find((app) => app.id === activeTab.appId) || null : null;
  $: visibleTabs = activeId ? tabs.filter((tab) => tab.appId === activeId) : [];

  function shortenTabTitle(title: string) {
    return title.length > 20 ? `${title.slice(0, 20)}...` : title;
  }

  $: notificationApps = Array.from(new Set(notifications.map((note) => note.appId))).sort();
  $: filteredNotifications = notifications.filter((note) => {
    const matchesApp = brainAppFilter === 'all' || note.appId === brainAppFilter;
    const searchValue = brainSearch.trim().toLowerCase();
    const matchesSearch = !searchValue
      || note.title.toLowerCase().includes(searchValue)
      || note.body.toLowerCase().includes(searchValue)
      || note.appId.toLowerCase().includes(searchValue);
    return matchesApp && matchesSearch;
  });

  function toDisplayName(app: AppConfig) {
    return app.name || app.id.replace(/-/g, ' ');
  }

  function buildAppId(name: string) {
    return name
      .toLowerCase()
      .trim()
      .replace(/[^a-z0-9\s-]/g, '')
      .replace(/\s+/g, '-')
      .replace(/-+/g, '-');
  }

  async function persistApps(nextApps: AppConfig[]) {
    localStorage.setItem('vessel_apps_backup', JSON.stringify(nextApps));

    if (!storeReady) return;
    try {
      await store.set('apps', nextApps);
      if (typeof store.save === 'function') {
        await store.save();
      }
    } catch (error) {
      console.error('Failed to persist apps in store, backup kept in localStorage', error);
    }
  }

  function loadAppsBackup() {
    try {
      const raw = localStorage.getItem('vessel_apps_backup');
      if (!raw) return [];
      const parsed = JSON.parse(raw);
      if (!Array.isArray(parsed)) return [];
      return parsed as AppConfig[];
    } catch {
      return [];
    }
  }

  async function refreshUsage() {
    const next = await invoke<ResourceUsage>('get_resource_usage');
    usage = next;
  }

  async function openTab(app: AppConfig, tab: BrowserTab) {
    const siteOptimizations = getSiteOptimizations(tab.url);
    const jsToInject = app.features?.adblock && app.url.includes("youtube.com")
      ? `setInterval(() => { const skipBtn = document.querySelector('.ytp-skip-ad-button'); if(skipBtn) skipBtn.click(); }, 500);`
      : "";
    const cssToInject = [themes[app.features?.theme || 'default'], siteOptimizations.css]
      .filter(Boolean)
      .join('\n');
    const finalJsToInject = [jsToInject, siteOptimizations.js]
      .filter(Boolean)
      .join('\n');

    try {
      await invoke("open_app", {
        id: tab.id,
        appId: app.id,
        url: tab.url,
        profile: app.features.profile || 'default',
        css: cssToInject,
        customCss: app.features.customCss || '',
        js: finalJsToInject,
        idleSleepSeconds: Number(app.features.idleSleepSeconds || 0)
      });
      return { ok: true as const };
    } catch (error) {
      console.error('Failed to open tab webview', error);
      return { ok: false as const, error: String(error) };
    }
  }

  async function switchToTab(tabId: string) {
    const tab = tabs.find((item) => item.id === tabId);
    if (!tab) return { ok: false as const };
    const app = apps.find((item) => item.id === tab.appId);
    if (!app) return { ok: false as const };

    activeTabId = tab.id;
    activeId = app.id;
    currentView = 'webview';
    return openTab(app, tab);
  }

  async function addTabForApp(app: AppConfig, url?: string, title?: string) {
    const tabUrl = normalizeServiceUrl(url || app.url);
    try {
      new URL(tabUrl);
    } catch {
      formError = 'Invalid URL for tab launch.';
      return;
    }

    const tab: BrowserTab = {
      id: nextTabId(app.id),
      appId: app.id,
      title: title || toDisplayName(app),
      url: tabUrl
    };
    tabs = [...tabs, tab];
    const opened = await switchToTab(tab.id);
    if (!opened?.ok) {
      tabs = tabs.filter((item) => item.id !== tab.id);
      if (tabs.length === 0) {
        activeTabId = '';
        activeId = '';
        switchView('gallery');
      }
      const launchError = opened && 'error' in opened ? opened.error : null;
      formError = launchError
        ? `Launch failed: ${launchError}`
        : 'Launch failed: unable to open this site.';
    }
  }

  async function closeTab(tabId: string) {
    const tab = tabs.find((item) => item.id === tabId);
    if (!tab) return;

    await invoke('close_webview', { id: tab.id });
    tabs = tabs.filter((item) => item.id !== tabId);

    if (tabs.length === 0) {
      activeTabId = '';
      activeId = '';
      switchView('gallery');
      return;
    }

    if (activeTabId === tabId) {
      const sameAppTabs = tabs.filter((item) => item.appId === tab.appId);
      if (sameAppTabs.length > 0) {
        await switchToTab(sameAppTabs[sameAppTabs.length - 1].id);
      } else {
        await switchToTab(tabs[tabs.length - 1].id);
      }
    }
  }

  async function openTabFromEvent(payload: NewTabEvent) {
    const app = apps.find((item) => item.id === payload.appId);
    if (!app) return;
    await addTabForApp(app, payload.url, payload.title || 'New Tab');
  }

  function openCurrentSiteSettings() {
    if (activeApp) openSettings(activeApp);
  }

  async function openNewTabForActiveApp() {
    if (!activeApp) return;
    await addTabForApp(activeApp);
  }

  async function setPerformanceStats(enabled: boolean) {
    showPerformanceStats = enabled;
    await store.set('showPerformanceStats', enabled);

    if (usageTimer) {
      clearInterval(usageTimer);
      usageTimer = null;
    }

    if (enabled) {
      await refreshUsage();
      usageTimer = setInterval(() => {
        refreshUsage();
      }, 3000);
    }
  }

  onMount(() => {
    let unlisten = () => {};
    let isUnmounted = false;

    updateViewport();
    window.addEventListener('resize', updateViewport);

    (async () => {
      store = await load('vessel_settings.json', { autoSave: true, defaults: {} });
      storeReady = true;
      const savedApps = await store.get('apps');
      const backupApps = loadAppsBackup();
      const storeApps = Array.isArray(savedApps) ? (savedApps as AppConfig[]) : [];
      const rawApps = storeApps.length >= backupApps.length ? storeApps : backupApps;
      const needsSync = storeApps.length !== backupApps.length;
      const needsMigration = rawApps.some((app) =>
        !app.features
          || app.features.profile === undefined
          || app.features.customCss === undefined
          || app.features.idleSleepSeconds === undefined
      );
      const normalizedApps = rawApps.map((app) => normalizeApp(app));
      if (!isUnmounted) {
        apps = normalizedApps;
      }
      if (!savedApps || needsMigration || needsSync) {
        await persistApps(normalizedApps);
      } else {
        localStorage.setItem('vessel_apps_backup', JSON.stringify(normalizedApps));
      }

      const storedPerf = await store.get('showPerformanceStats');
      if (!isUnmounted) {
        showPerformanceStats = Boolean(storedPerf);
      }
      if (showPerformanceStats) {
        await setPerformanceStats(true);
      }

      const dbNotifications = await invoke<VesselNotification[]>('get_notifications', { limit: 50 });
      if (!isUnmounted) {
        notifications = dbNotifications;
      }

      unlisten = await listen("vessel-notification", (event: { payload: VesselNotification }) => {
        notifications = [event.payload, ...notifications].slice(0, 50);
        if (!showBrain) unreadCount++;
      });

      const unlistenTabs = await listen('vessel-open-tab', (event: { payload: NewTabEvent }) => {
        openTabFromEvent(event.payload);
      });

      const prev = unlisten;
      unlisten = () => {
        prev();
        unlistenTabs();
      };
    })();

    return () => {
      isUnmounted = true;
      window.removeEventListener('resize', updateViewport);
      if (usageTimer) clearInterval(usageTimer);
      unlisten();
    };
  });

  async function launch(app: AppConfig) {
    const existingTab = tabs.find((tab) => tab.appId === app.id);
    if (existingTab) {
      await switchToTab(existingTab.id);
      return;
    }
    await addTabForApp(app);
  }

  function switchView(view: 'gallery' | 'settings') {
    currentView = view;
    editingApp = view === 'settings' ? editingApp : null;
    invoke("hide_all_webviews");
  }

  async function goHome() {
    await goto('/');
    activeId = '';
    switchView('gallery');
  }

  function toggleBrain() {
    showBrain = !showBrain;
    if (showBrain) unreadCount = 0;
  }

  async function clearBrain() {
    notifications = [];
    unreadCount = 0;
    await invoke('clear_notifications');
  }

  function setProfilePreset(value: string) {
    if (!editingApp) return;
    editingApp.features.profile = value;
  }

  function openSettings(app: AppConfig) {
    editingApp = JSON.parse(JSON.stringify(app));
    switchView('settings');
  }

  async function saveSettings() {
    if (!editingApp) return;
    const draft = editingApp;
    const index = apps.findIndex(a => a.id === draft.id);
    if (index !== -1) {
      apps[index] = draft;
      tabs = tabs.map((tab) =>
        tab.appId === draft.id && tab.title === toDisplayName(apps[index])
          ? { ...tab, title: toDisplayName(draft) }
          : tab
      );
      await persistApps(apps);
      if (activeTab && activeTab.appId === draft.id) {
        await switchToTab(activeTab.id);
      }
      switchView('gallery');
    }
  }

  async function addNewApp() {
    formError = '';
    const trimmedName = newAppName.trim();
    const trimmedUrl = newAppUrl.trim();
    if (!trimmedName || !trimmedUrl) {
      formError = 'Name and URL are required.';
      return;
    }

    const id = buildAppId(trimmedName);
    if (!id) {
      formError = 'Use letters or numbers in the app name.';
      return;
    }

    if (apps.some((app) => app.id === id)) {
      formError = 'An app with this name already exists.';
      return;
    }

    const normalizedUrl = normalizeServiceUrl(trimmedUrl);
    try {
      // Validate URL once to avoid broken webview launches.
      new URL(normalizedUrl);
    } catch {
      formError = 'Please enter a valid URL.';
      return;
    }

    const newApp: AppConfig = { 
      id,
      name: trimmedName,
      icon: (newAppIcon || trimmedName.charAt(0)).toUpperCase(), 
      url: normalizedUrl, 
      features: { theme: 'default', adblock: false, profile: 'default', customCss: '', idleSleepSeconds: 15 } 
    };
    apps = [...apps, newApp]; 
    await persistApps(apps);
    newAppName = ''; newAppUrl = ''; newAppIcon = ''; 
  }
</script>

<div class="bg-background text-on-background font-body select-none overflow-hidden h-screen w-screen flex flex-col">
  
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
      {#each apps as app}
        <div class="relative group w-full flex justify-center">
          <button on:click={() => launch(app)} on:contextmenu|preventDefault={() => openSettings(app)} class="flex flex-col items-center space-y-1 transition-all duration-200 scale-95 active:scale-90 {activeId === app.id && currentView === 'webview' ? 'text-primary border-l-2 border-primary-container w-full' : 'text-outline hover:text-on-surface w-full'}">
            <div class="w-10 h-10 flex items-center justify-center rounded-lg {activeId === app.id && currentView === 'webview' ? 'bg-surface-variant' : 'group-hover:bg-surface-container-highest'} transition-colors text-xl font-bold">
              {app.icon}
            </div>
            <span class="font-label tracking-tight text-[9px] uppercase truncate px-1 max-w-full">{toDisplayName(app)}</span>
          </button>
        </div>
      {/each}
    </nav>

    <div class="w-full flex flex-col items-center space-y-4 px-2 mt-auto pt-4 border-t border-outline-variant/10">
      <div class="h-px w-8 bg-outline-variant opacity-20"></div>
      
      <button on:click={() => switchView('gallery')} class="flex flex-col items-center space-y-1 text-outline hover:text-on-surface transition-all duration-200 scale-95 active:scale-90 group w-full">
        <div class="w-10 h-10 flex items-center justify-center rounded-lg group-hover:bg-surface-container-highest transition-colors">
          <span class="material-symbols-outlined">add</span>
        </div>
        <span class="font-label tracking-tight text-[9px] uppercase">Add Site</span>
      </button>
    </div>
  </aside>

  <header class="h-14 w-full flex items-center px-2 sm:px-3 bg-surface-dim sticky top-0 z-40 ml-14 sm:ml-16 justify-between border-b border-outline-variant/10 overflow-hidden">
    {#if currentView === 'webview'}
      <div class="flex items-center gap-2 flex-1 min-w-0">
        <div class="flex items-center gap-1 overflow-x-auto vessel-scroll pr-2">
          {#each visibleTabs as tab}
            <div
              role="button"
              tabindex="0"
              on:click={() => switchToTab(tab.id)}
              on:keydown={(e) => (e.key === 'Enter' || e.key === ' ') && switchToTab(tab.id)}
              class="group h-8 min-w-32 max-w-52 px-3 rounded-md border text-xs font-medium flex items-center justify-between gap-2 transition-colors {activeTabId === tab.id ? 'bg-surface-container text-on-surface border-outline-variant/30' : 'bg-surface-container-lowest text-on-surface-variant border-outline-variant/15 hover:bg-surface-container-high'}"
              title={tab.url}
            >
              <span class="truncate">{shortenTabTitle(tab.title)}</span>
              {#if visibleTabs.length > 1}
                <button
                  type="button"
                  class="material-symbols-outlined text-sm opacity-60 group-hover:opacity-100"
                  on:click|stopPropagation={() => closeTab(tab.id)}
                >close</button>
              {/if}
            </div>
          {/each}
        </div>
        {#if activeApp}
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
          <input class="w-full h-8 bg-surface-container-lowest border border-outline-variant/20 rounded-lg pl-10 pr-4 text-sm font-label font-medium tracking-wide text-on-surface-variant focus:ring-1 focus:ring-primary-container transition-all" placeholder="vessel://{currentView}" readonly type="text"/>
        </div>
      </div>
    {/if}

    <div class="flex items-center gap-2 pr-1 sm:pr-2">
      <button on:click={toggleBrain} class="relative p-1.5 rounded-lg hover:bg-surface-container-highest transition-colors {showBrain ? 'text-primary' : 'text-on-surface-variant hover:text-on-surface'}" title="Notifications" aria-label="Notifications">
        <span class="material-symbols-outlined text-[20px]">notifications</span>
        {#if unreadCount > 0}
          <span class="absolute top-1 right-1 bg-error w-2 h-2 rounded-full border-2 border-surface-dim"></span>
        {/if}
      </button>
      {#if currentView === 'webview' && activeApp}
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

  <main
    class="ml-14 sm:ml-16 mb-0 p-4 sm:p-6 lg:p-8 overflow-y-auto scrollbar-hide relative bg-background"
    style:height="calc(100vh - 56px)"
  >
    
    {#if currentView === 'webview'}
      <div class="w-full h-full flex flex-col items-center justify-center text-outline-variant opacity-30">
        <span class="material-symbols-outlined text-6xl mb-4">memory</span>
        <span class="font-headline tracking-widest uppercase">Native Container Active</span>
      </div>
    {/if}

    {#if currentView === 'gallery'}
      <header class="mb-8 sm:mb-12">
        <h1 class="font-headline font-extrabold text-3xl sm:text-4xl lg:text-5xl tracking-tight text-on-surface mb-2">Workspace Gallery</h1>
        <p class="text-on-surface-variant font-body max-w-xl text-sm sm:text-base lg:text-lg">Extend your workspace with new instances and toolsets.</p>
      </header>

      <div class="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-4 sm:gap-6">
        <div class="col-span-1 sm:col-span-2 row-span-2 relative overflow-hidden rounded-xl bg-surface-container-lowest border border-outline-variant/10 p-4 sm:p-6 lg:p-8 flex flex-col justify-between">
          <div>
            <div class="flex items-center space-x-2 mb-4">
              <span class="material-symbols-outlined text-primary-fixed-dim" style="font-variation-settings: 'FILL' 1;">add_circle</span>
              <span class="font-label text-sm font-bold text-on-surface uppercase tracking-widest">New Deployment</span>
            </div>
            <h2 class="font-headline font-bold text-2xl sm:text-3xl text-on-surface mb-2">Create Custom Instance</h2>
            <p class="text-on-surface-variant text-sm sm:text-base mb-6 max-w-sm">Deploy any web application into an isolated, optimized Vessel container.</p>
            
            <div class="space-y-4 max-w-md">
              <div>
                <label for="instance-name" class="text-[10px] text-outline uppercase tracking-widest block mb-1">Instance Name</label>
                <input id="instance-name" type="text" bind:value={newAppName} placeholder="e.g. ChatGPT" class="w-full bg-surface-container border border-outline-variant/30 rounded-lg p-3 text-sm text-on-surface focus:outline-none focus:ring-1 focus:ring-primary"/>
              </div>
              <div>
                <label for="target-url" class="text-[10px] text-outline uppercase tracking-widest block mb-1">Target URL</label>
                <input id="target-url" type="url" bind:value={newAppUrl} placeholder="https://chatgpt.com" class="w-full bg-surface-container border border-outline-variant/30 rounded-lg p-3 text-sm text-on-surface focus:outline-none focus:ring-1 focus:ring-primary"/>
              </div>
              <div>
                <label for="icon-character" class="text-[10px] text-outline uppercase tracking-widest block mb-1">Icon Character</label>
                <input id="icon-character" type="text" bind:value={newAppIcon} maxlength="1" placeholder="C" class="w-full bg-surface-container border border-outline-variant/30 rounded-lg p-3 text-sm text-on-surface text-center focus:outline-none focus:ring-1 focus:ring-primary"/>
              </div>
            </div>
            {#if formError}
              <p class="mt-4 text-xs text-error uppercase tracking-wider">{formError}</p>
            {/if}
          </div>
          
          <div class="mt-8 flex items-center space-x-4">
            <button
              on:click={addNewApp}
              class="px-5 sm:px-6 py-2.5 primary-gradient text-on-primary font-label font-bold rounded-sm text-xs sm:text-sm hover:brightness-110 disabled:opacity-50 disabled:cursor-not-allowed"
              disabled={!newAppName.trim() || !newAppUrl.trim()}
            >
              Deploy Container
            </button>
          </div>
        </div>

        {#each apps as app}
          <div class="bg-surface-container rounded-xl p-4 sm:p-6 border border-outline-variant/10 flex flex-col justify-between hover:bg-surface-container-high transition-colors min-h-44">
            <div>
              <div class="w-12 h-12 bg-surface-container-lowest rounded-lg flex items-center justify-center mb-6 ring-1 ring-outline-variant/20 text-2xl font-bold text-on-surface">
                {app.icon}
              </div>
              <div class="flex justify-between items-start mb-2">
                <h3 class="font-headline font-bold text-base sm:text-lg text-on-surface capitalize">{toDisplayName(app)}</h3>
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
      <div class="max-w-6xl mx-auto flex flex-col bg-surface-container shadow-2xl rounded-xl overflow-hidden border border-outline-variant/15 mt-2 sm:mt-4">
        <div class="px-4 sm:px-6 lg:px-8 py-4 sm:py-6 flex flex-col sm:flex-row gap-4 sm:items-center sm:justify-between border-b border-outline-variant/10 bg-surface-container-high">
          <div class="flex items-center space-x-4">
            <div class="w-10 h-10 bg-primary-container/20 rounded-lg flex items-center justify-center text-xl font-bold text-primary">
              {editingApp.icon}
            </div>
            <div>
              <h1 class="text-lg sm:text-xl font-bold font-headline tracking-tight text-on-surface capitalize">{toDisplayName(editingApp)} Settings</h1>
              <p class="text-xs text-on-surface-variant uppercase tracking-widest font-mono">Target: {editingApp.url}</p>
            </div>
          </div>
          <div class="flex items-center space-x-3">
            <button on:click={() => switchView('gallery')} class="px-4 py-1.5 text-xs font-semibold bg-surface-container-highest rounded border border-outline-variant/20 hover:bg-surface-bright transition-colors text-on-surface">Discard</button>
            <button on:click={saveSettings} class="px-4 py-1.5 text-xs font-semibold bg-gradient-to-br from-primary to-primary-container text-on-primary rounded shadow-lg shadow-primary-container/20 hover:opacity-90 transition-opacity">Save & Deploy</button>
          </div>
        </div>

        <div class="flex flex-1 overflow-hidden flex-col lg:flex-row">
          <nav class="w-full lg:w-64 bg-surface-container-low border-r border-outline-variant/10 flex lg:flex-col py-4 lg:py-6 overflow-x-auto">
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
          
          <div class="flex-1 overflow-y-auto vessel-scroll bg-surface p-4 sm:p-6 lg:p-10 space-y-8 sm:space-y-12">
            <section>
              <div class="flex items-center justify-between mb-8">
                <h2 class="text-xl sm:text-2xl font-bold tracking-tight text-on-surface">Appearance</h2>
              </div>
              <div class="space-y-8">
                <div class="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4 p-4 sm:p-6 rounded-lg bg-surface-container-high border border-outline-variant/10">
                  <div>
                    <h3 class="font-bold text-on-surface">Session Profile</h3>
                    <p class="text-sm text-on-surface-variant">Use different isolated cookie jars per app, like Work and Personal.</p>
                  </div>
                  <div class="w-full sm:w-64 space-y-2">
                    <input
                      type="text"
                      bind:value={editingApp.features.profile}
                      placeholder="default"
                      class="bg-surface-container-lowest border border-outline-variant/30 rounded-lg p-2 text-sm text-on-surface focus:outline-none focus:border-primary w-full"
                    />
                    <div class="flex gap-2">
                      <button type="button" on:click={() => setProfilePreset('default')} class="text-[10px] uppercase tracking-widest px-2 py-1 rounded border border-outline-variant/30 text-on-surface-variant hover:text-on-surface">Default</button>
                      <button type="button" on:click={() => setProfilePreset('work')} class="text-[10px] uppercase tracking-widest px-2 py-1 rounded border border-outline-variant/30 text-on-surface-variant hover:text-on-surface">Work</button>
                      <button type="button" on:click={() => setProfilePreset('personal')} class="text-[10px] uppercase tracking-widest px-2 py-1 rounded border border-outline-variant/30 text-on-surface-variant hover:text-on-surface">Personal</button>
                    </div>
                  </div>
                </div>

                <div class="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4 p-4 sm:p-6 rounded-lg bg-surface-container-high border border-outline-variant/10">
                  <div>
                    <h3 class="font-bold text-on-surface">Background Sleep Delay</h3>
                    <p class="text-sm text-on-surface-variant">Delay hibernation when this app goes to background. 0 means instant sleep.</p>
                  </div>
                  <div class="w-full sm:w-64 flex items-center gap-3">
                    <input type="range" min="0" max="120" step="5" bind:value={editingApp.features.idleSleepSeconds} class="w-full accent-primary" />
                    <span class="text-xs font-mono text-on-surface-variant w-14 text-right">{editingApp.features.idleSleepSeconds}s</span>
                  </div>
                </div>

                <div class="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4 p-4 sm:p-6 rounded-lg bg-surface-container-high border border-outline-variant/10">
                  <div>
                    <h3 class="font-bold text-on-surface">Visual Injection Engine</h3>
                    <p class="text-sm text-on-surface-variant">Override default site styles with native UI palettes.</p>
                  </div>
                  <div class="relative w-full sm:w-48">
                    <select bind:value={editingApp.features.theme} class="appearance-none bg-surface-container-lowest border border-outline-variant/30 rounded-lg p-2 pr-9 text-sm text-on-surface focus:outline-none focus:ring-1 focus:ring-primary focus:border-primary w-full">
                      <option value="default">None (Default)</option>
                      <option value="dark_invert">Force Dark Mode</option>
                      <option value="oled">True OLED Black</option>
                    </select>
                    <span class="material-symbols-outlined absolute right-2 top-1/2 -translate-y-1/2 pointer-events-none text-outline text-base">expand_more</span>
                  </div>
                </div>

                {#if editingApp.url.includes("youtube.com")}
                  <div class="flex items-center justify-between p-4 sm:p-6 rounded-lg bg-surface-container-high border border-outline-variant/10">
                    <div>
                      <h3 class="font-bold text-on-surface">Ad-blocking Engine</h3>
                      <p class="text-sm text-on-surface-variant">Intercept and nullify script-based advertisement injections.</p>
                    </div>
                    <input type="checkbox" bind:checked={editingApp.features.adblock} class="w-5 h-5 accent-primary bg-surface-container-lowest border-outline-variant/30 rounded"/>
                  </div>
                {/if}

                <div class="p-4 sm:p-6 rounded-lg bg-surface-container-high border border-outline-variant/10">
                  <div class="mb-3">
                    <h3 class="font-bold text-on-surface">Custom CSS Injection</h3>
                    <p class="text-sm text-on-surface-variant">Apply site-specific CSS themes and visual fixes.</p>
                  </div>
                  <textarea
                    bind:value={editingApp.features.customCss}
                    rows="8"
                    placeholder="Example: body selector font-family Space Grotesk"
                    class="w-full rounded-lg border border-outline-variant/30 bg-surface-container-lowest p-3 text-xs sm:text-sm text-on-surface font-mono focus:outline-none focus:ring-1 focus:ring-primary"
                  ></textarea>
                </div>
              </div>
            </section>
          </div>
        </div>
      </div>
    {/if}
  </main>

  {#if showBrain}
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
          bind:value={brainSearch}
          placeholder="Search notifications"
          class="w-full rounded-lg border border-outline-variant/30 bg-surface-container-lowest px-3 py-2 text-xs text-on-surface focus:outline-none focus:ring-1 focus:ring-primary"
        />
        <select
          bind:value={brainAppFilter}
          class="w-full rounded-lg border border-outline-variant/30 bg-surface-container-lowest px-3 py-2 text-xs text-on-surface focus:outline-none focus:ring-1 focus:ring-primary"
        >
          <option value="all">All Apps</option>
          {#each notificationApps as appId}
            <option value={appId}>{appId}</option>
          {/each}
        </select>
      </div>
      
      <div class="flex-grow overflow-y-auto vessel-scroll px-6 space-y-4 pb-6">
        {#if filteredNotifications.length === 0}
          <div class="text-center text-outline-variant mt-20 text-sm">Awaiting neural input streams.</div>
        {/if}
        {#each filteredNotifications as note}
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



</div>