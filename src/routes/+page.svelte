<script lang="ts">
  import { goto } from "$app/navigation";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event"; 
  import { load } from "@tauri-apps/plugin-store";
  import { onMount } from "svelte";
  import { deserializeAppsBackup, normalizeFeatures, serializeAppsBackup } from "$lib/settingsPersistence";

  type AppView = 'gallery' | 'webview' | 'settings' | 'add-app';

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
      customJs: string;
      injectionAllowlist: string;
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

  type DiagnosticEvent = {
    level: string;
    category: string;
    appId: string;
    webviewId?: string | null;
    message: string;
    detail?: string | null;
    time: string;
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
  let showDiagnostics = false;
  let notifications: VesselNotification[] = [];
  let diagnostics: DiagnosticEvent[] = [];
  let safeMode = false;
  let unreadCount = 0;
  let brainSearch = '';
  let brainAppFilter = 'all';
  let showPerformanceStats = false;
  let usage: ResourceUsage = { cpuPercent: 0, ramMb: 0 };
  let usageTimer: ReturnType<typeof setInterval> | null = null;
  let showShortcuts = false;

  const keyboardShortcuts = [
    { key: 'Ctrl + G', action: 'Go to Gallery' },
    { key: 'Ctrl + N', action: 'Deploy New Site' },
    { key: 'Ctrl + B', action: 'Toggle Global Brain' },
    { key: 'Ctrl + D', action: 'Toggle Diagnostics' },
    { key: 'Ctrl + S', action: 'Toggle Safe Mode' },
    { key: 'Esc', action: 'Close Overlays / Back' }
  ];

  function handleKeydown(e: KeyboardEvent) {
    if (e.ctrlKey) {
      switch (e.key.toLowerCase()) {
        case 'g': e.preventDefault(); goHome(); break;
        case 'n': e.preventDefault(); switchView('add-app'); break;
        case 'b': e.preventDefault(); toggleBrain(); break;
        case 'd': e.preventDefault(); toggleDiagnostics(); break;
        case 's': e.preventDefault(); setSafeMode(!safeMode); break;
      }
    }
    if (e.key === 'Escape') {
      if (showBrain) showBrain = false;
      else if (showDiagnostics) showDiagnostics = false;
      else if (showShortcuts) showShortcuts = false;
      else if (currentView !== 'gallery') switchView('gallery');
    }
  }

  // Add App Form State
  let newAppName = ''; let newAppUrl = ''; let newAppIcon = '';
  let showSuccess = false;
  let successTimer: ReturnType<typeof setTimeout> | null = null;
  let lastDeployedAppId = '';
  let isDeploying = false;
  let isSaving = false;
  let initialSettingsState = '';
  let settingsSearch = '';

  $: isNameValid = newAppName.trim().length > 0 && !!buildAppId(newAppName.trim());
  $: isUrlValid = (newAppUrl.trim().length > 0) && (() => { 
    try { 
      new URL(normalizeServiceUrl(newAppUrl)); 
      return true; 
    } catch { 
      return false; 
    } 
  })();

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
        ...normalizeFeatures(input.features),
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
    localStorage.setItem('vessel_apps_backup', serializeAppsBackup(nextApps));

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
    return deserializeAppsBackup<AppConfig>(localStorage.getItem('vessel_apps_backup'));
  }

  async function refreshUsage() {
    const next = await invoke<ResourceUsage>('get_resource_usage');
    usage = next;
  }

  function exportWorkspace() {
    const data = JSON.stringify(apps, null, 2);
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
          const merged = [...apps];
          for (const imp of importedApps) {
            const idx = merged.findIndex(a => a.id === imp.id);
            if (idx !== -1) merged[idx] = imp;
            else merged.push(imp);
          }
          apps = merged.map(a => normalizeApp(a));
          await persistApps(apps);
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

  const cssSnippets = [
    { label: 'Hide Scrollbars', value: '::-webkit-scrollbar { display: none; }' },
    { label: 'OLED Black BG', value: 'body, main, #root { background: #000 !important; }' },
    { label: 'Modern Sans Serif', value: '* { font-family: system-ui, -apple-system, sans-serif !important; }' },
    { label: 'Greyscale Mode', value: 'html { filter: grayscale(100%); }' }
  ];

  function applySnippet(css: string) {
    if (!editingApp) return;
    editingApp.features.customCss = (editingApp.features.customCss + '\n' + css).trim();
  }

  async function openTab(app: AppConfig, tab: BrowserTab) {
    const siteOptimizations = getSiteOptimizations(tab.url);
    const jsToInject = app.features?.adblock && app.url.includes("youtube.com")
      ? `setInterval(() => { const skipBtn = document.querySelector('.ytp-skip-ad-button'); if(skipBtn) skipBtn.click(); }, 500);`
      : "";
    const cssToInject = [themes[app.features?.theme || 'default'], siteOptimizations.css]
      .filter(Boolean)
      .join('\n');
    const baseJsToInject = [jsToInject, siteOptimizations.js]
      .filter(Boolean)
      .join('\n');

    try {
      const allowlist = (app.features.injectionAllowlist || '')
        .split(/[,\n\s]+/)
        .map((item) => item.trim())
        .filter(Boolean);

      await invoke("open_app", {
        id: tab.id,
        appId: app.id,
        url: tab.url,
        profile: app.features.profile || 'default',
        css: cssToInject,
        customCss: app.features.customCss || '',
        js: baseJsToInject,
        customJs: app.features.customJs || '',
        injectionAllowlist: allowlist,
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

  async function setSafeMode(enabled: boolean) {
    safeMode = enabled;
    await store.set('safeMode', enabled);
    await invoke('set_safe_mode', { enabled });
  }

  onMount(() => {
    let unlisten = () => {};
    let isUnmounted = false;

    updateViewport();
    window.addEventListener('resize', updateViewport);
    window.addEventListener('keydown', handleKeydown);

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
          || app.features.customJs === undefined
          || app.features.injectionAllowlist === undefined
          || app.features.idleSleepSeconds === undefined
      );
      const normalizedApps = rawApps.map((app) => normalizeApp(app));
      if (!isUnmounted) {
        apps = normalizedApps;
      }
      if (!savedApps || needsMigration || needsSync) {
        await persistApps(normalizedApps);
      } else {
        localStorage.setItem('vessel_apps_backup', serializeAppsBackup(normalizedApps));
      }

      const storedPerf = await store.get('showPerformanceStats');
      if (!isUnmounted) {
        showPerformanceStats = Boolean(storedPerf);
      }
      if (showPerformanceStats) {
        await setPerformanceStats(true);
      }

      const storedSafeMode = await store.get('safeMode');
      safeMode = Boolean(storedSafeMode);
      await invoke('set_safe_mode', { enabled: safeMode });

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

      const unlistenDiagnostics = await listen('vessel-diagnostic', (event: { payload: DiagnosticEvent }) => {
        diagnostics = [event.payload, ...diagnostics].slice(0, 120);
      });

      const prev = unlisten;
      unlisten = () => {
        prev();
        unlistenTabs();
        unlistenDiagnostics();
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

  function switchView(view: AppView) {
    if (currentView === 'settings' && initialSettingsState && JSON.stringify(editingApp) !== initialSettingsState) {
      if (!confirm('You have unsaved changes. Are you sure you want to navigate away?')) return;
    }
    currentView = view;
    editingApp = view === 'settings' ? editingApp : null;
    if (view !== 'settings') initialSettingsState = '';
    invoke("hide_all_webviews");
  }

  async function goHome() {
    if (currentView === 'settings' && initialSettingsState && JSON.stringify(editingApp) !== initialSettingsState) {
      if (!confirm('You have unsaved changes. Discard them?')) return;
    }
    await goto('/');
    activeId = '';
    switchView('gallery');
  }

  function toggleBrain() {
    showBrain = !showBrain;
    if (showBrain) unreadCount = 0;
  }

  function toggleDiagnostics() {
    showDiagnostics = !showDiagnostics;
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
    initialSettingsState = JSON.stringify(editingApp);
    switchView('settings');
  }

  async function saveSettings() {
    if (!editingApp || isSaving) return;
    isSaving = true;
    try {
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
        initialSettingsState = '';
        switchView('gallery');
      }
    } finally {
      isSaving = false;
    }
  }

  async function deleteInstance() {
    if (!editingApp) return;
    const appIdToDelete = editingApp.id;
    if (!confirm(`Are you sure you want to discard "${toDisplayName(editingApp)}" and ALL its session data? This cannot be undone.`)) return;

    try {
      // 1. Close any active tabs for this app
      const relatedTabs = tabs.filter(t => t.appId === appIdToDelete);
      for (const tab of relatedTabs) {
        await invoke('close_webview', { id: tab.id });
      }
      tabs = tabs.filter(t => t.appId !== appIdToDelete);

      // 2. Clear session data from disk
      await invoke('delete_app_session', { appId: appIdToDelete });

      // 3. Remove from apps list and persist
      apps = apps.filter(a => a.id !== appIdToDelete);
      await persistApps(apps);

      // 4. Return to gallery
      switchView('gallery');
    } catch (error) {
      console.error('Failed to delete instance', error);
      alert('Failed to delete instance data. See console for details.');
    }
  }

  async function addNewApp() {
    if (isDeploying) return;
    isDeploying = true;
    try {
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
        features: { theme: 'default', adblock: false, profile: 'default', customCss: '', customJs: '', injectionAllowlist: '', idleSleepSeconds: 15 } 
      };
      apps = [...apps, newApp]; 
      await persistApps(apps);
      
      lastDeployedAppId = id;
      showSuccess = true;
      if (successTimer) clearTimeout(successTimer);
      successTimer = setTimeout(() => { showSuccess = false; }, 6000); // Longer success visibility

      newAppName = ''; newAppUrl = ''; newAppIcon = ''; 
    } finally {
      isDeploying = false;
    }
  }

  const featuredApps = [
    { name: 'ChatGPT', url: 'https://chat.openai.com', icon: '🤖' },
    { name: 'Notion', url: 'https://www.notion.so', icon: '📝' },
    { name: 'Spotify', url: 'https://open.spotify.com', icon: '🎵' }
  ];

  function deployFeatured(featured: { name: string, url: string, icon: string }) {
    newAppName = featured.name;
    newAppUrl = featured.url;
    newAppIcon = featured.icon;
    switchView('add-app');
  }

  let isCheckingUrl = false;
  let urlStatus: 'none' | 'valid' | 'invalid' | 'checking' = 'none';

  async function checkUrlAccessibility() {
    if (!isUrlValid) {
      urlStatus = 'invalid';
      return;
    }
    isCheckingUrl = true;
    urlStatus = 'checking';
    // Simulate accessibility check
    await new Promise(r => setTimeout(r, 1500));
    isCheckingUrl = false;
    urlStatus = 'valid';
  }

  function resetForm() {
    newAppName = '';
    newAppUrl = '';
    newAppIcon = '';
    formError = '';
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
          <button 
            on:click={() => launch(app)} 
            on:contextmenu|preventDefault={() => openSettings(app)} 
            class="flex flex-col items-center transition-all duration-200 scale-95 active:scale-90 {activeId === app.id && currentView === 'webview' ? 'text-primary border-l-2 border-primary-container w-full' : 'text-outline hover:text-on-surface w-full'}"
            title={toDisplayName(app)}
            aria-label="Launch {toDisplayName(app)}"
          >
            <div class="w-10 h-10 flex items-center justify-center rounded-lg {activeId === app.id && currentView === 'webview' ? 'bg-surface-variant' : 'group-hover:bg-surface-container-highest'} transition-colors text-xl font-bold">
              {app.icon}
            </div>
          </button>
        </div>
      {/each}
    </nav>

    <div class="w-full flex flex-col items-center space-y-4 px-2 mt-auto pt-4 border-t border-outline-variant/10">
      <div class="h-px w-8 bg-outline-variant opacity-20"></div>
      
      <button on:click={() => switchView('add-app')} class="flex flex-col items-center space-y-1 text-outline hover:text-on-surface transition-all duration-200 scale-95 active:scale-90 group w-full">
        <div class="w-10 h-10 flex items-center justify-center rounded-lg {currentView === 'add-app' ? 'bg-primary/10 text-primary' : 'group-hover:bg-surface-container-highest'} transition-colors">
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
      <button
        on:click={() => setSafeMode(!safeMode)}
        class="p-1.5 rounded-lg transition-colors {safeMode ? 'text-error hover:bg-error/10' : 'text-on-surface-variant hover:text-on-surface hover:bg-surface-container-highest'}"
        title="Toggle safe mode"
        aria-label="Toggle safe mode"
      >
        <span class="material-symbols-outlined text-[20px]">security</span>
      </button>
      <button
        on:click={toggleDiagnostics}
        class="p-1.5 rounded-lg transition-colors {showDiagnostics ? 'text-primary' : 'text-on-surface-variant hover:text-on-surface hover:bg-surface-container-highest'}"
        title="Diagnostics"
        aria-label="Diagnostics"
      >
        <span class="material-symbols-outlined text-[20px]">monitoring</span>
      </button>
      <button
        on:click={() => showShortcuts = !showShortcuts}
        class="p-1.5 rounded-lg transition-colors {showShortcuts ? 'text-primary' : 'text-on-surface-variant hover:text-on-surface hover:bg-surface-container-highest'}"
        title="Keyboard Shortcuts"
        aria-label="Keyboard Shortcuts"
      >
        <span class="material-symbols-outlined text-[20px]">keyboard</span>
      </button>
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
    class="ml-14 sm:ml-16 mb-0 p-4 sm:p-6 lg:p-8 overflow-y-auto relative bg-background"
    style:height="calc(100vh - 56px)"
  >
    
    {#if currentView === 'webview'}
      <div class="w-full h-full flex flex-col items-center justify-center text-outline-variant opacity-30">
        <span class="material-symbols-outlined text-6xl mb-4">memory</span>
        <span class="font-headline tracking-widest uppercase">Native Container Active</span>
      </div>
    {/if}

    {#if currentView === 'gallery'}
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

            {#each apps as app}
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

            {#each featuredApps.filter(f => !apps.some(a => a.name === f.name)) as featured}
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
            {#if apps.length + featuredApps.length < 6}
              {#each Array(6 - (apps.length + featuredApps.length)) as _, i}
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
              {#if diagnostics.length === 0}
                <div class="p-4 rounded-lg border border-outline-variant/10 bg-surface-container-low text-center opacity-60">
                  <p class="text-xs text-on-surface-variant">System initialized. Awaiting container events.</p>
                </div>
              {/if}
              {#each diagnostics.slice(0, 10) as event}
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
                <span class="text-xs font-bold font-mono text-on-surface">{apps.length}</span>
              </div>
              <div class="flex justify-between items-center">
                <span class="text-xs text-on-surface-variant">Isolated Sessions</span>
                <span class="text-xs font-bold font-mono text-on-surface">{apps.length}</span>
              </div>
              <div class="mt-4 pt-4 border-t border-outline-variant/10">
                <p class="text-[10px] text-on-surface-variant leading-relaxed opacity-70 italic">Vessel provides true process isolation and memory hibernation for every site deployed above.</p>
              </div>
            </div>
          </section>
        </aside>
      </div>
    {/if}

    {#if currentView === 'add-app'}
      <div class="max-w-4xl mx-auto py-8">
        <div class="flex items-center justify-between mb-8">
          <div>
            <h1 class="font-headline font-extrabold text-3xl text-on-surface mb-1">Deploy New Instance</h1>
            <p class="text-on-surface-variant text-sm">Create an isolated, high-performance container for any web application.</p>
          </div>
          <button on:click={() => switchView('gallery')} class="flex items-center gap-2 px-4 py-2 rounded-lg bg-surface-container-high hover:bg-surface-container-highest text-on-surface transition-all border border-outline-variant/20 shadow-sm active:scale-95">
            <span class="material-symbols-outlined text-sm">arrow_back</span>
            <span class="font-label text-[10px] uppercase font-bold tracking-widest">Back to Gallery</span>
          </button>
        </div>

        <div class="grid grid-cols-1 lg:grid-cols-3 gap-8">
          <div class="lg:col-span-2 space-y-6">
            <div class="bg-surface-container rounded-xl p-6 sm:p-8 border border-outline-variant/10 shadow-sm space-y-6">
              
              <!-- Name Field -->
              <div class="space-y-1.5">
                <div class="flex justify-between items-center">
                  <label for="instance-name" class="text-[10px] text-outline uppercase tracking-widest font-bold">
                    Instance Name <span class="text-primary">*</span>
                  </label>
                  <span class="text-[9px] text-outline-variant font-mono">{newAppName.length}/32</span>
                </div>
                <div class="relative group">
                  <input 
                    id="instance-name" 
                    type="text" 
                    bind:value={newAppName} 
                    maxlength="32"
                    placeholder="e.g. ChatGPT, Spotify, Notion" 
                    aria-required="true"
                    class="w-full bg-surface-container-lowest border {newAppName.length > 0 ? (isNameValid ? 'border-primary/50' : 'border-error/50') : 'border-outline-variant/30'} rounded-lg p-3 text-sm text-on-surface focus:outline-none focus:ring-1 focus:ring-primary focus:border-primary transition-all shadow-inner"
                  />
                  <div class="absolute right-3 top-1/2 -translate-y-1/2 transition-opacity pointer-events-none">
                    {#if newAppName.length > 0}
                      <span class="material-symbols-outlined text-sm {isNameValid ? 'text-primary' : 'text-error'}">{isNameValid ? 'check_circle' : 'cancel'}</span>
                    {:else}
                      <span class="material-symbols-outlined text-sm text-outline group-focus-within:text-primary">edit</span>
                    {/if}
                  </div>
                </div>
                {#if newAppName.length > 0}
                  <p class="text-[9px] {isNameValid ? 'text-primary' : 'text-error'} font-bold flex items-center gap-1 animate-in fade-in slide-in-from-left-1">
                    <span class="material-symbols-outlined text-[10px]">{isNameValid ? 'check' : 'close'}</span>
                    {isNameValid ? `✓ ${newAppName} (available)` : '✗ Only letters and numbers allowed'}
                  </p>
                {:else}
                  <p class="text-[9px] text-on-surface-variant italic">Letters and numbers only. This will be used as the internal App ID.</p>
                {/if}
              </div>

              <!-- URL Field -->
              <div class="space-y-1.5">
                <div class="flex items-center gap-2">
                  <label for="target-url" class="text-[10px] text-outline uppercase tracking-widest font-bold">
                    Target URL <span class="text-primary">*</span>
                  </label>
                  <div class="group relative">
                    <span class="material-symbols-outlined text-xs text-outline cursor-help">info</span>
                    <div class="absolute bottom-full left-1/2 -translate-x-1/2 mb-2 w-48 p-2 bg-surface-container-highest text-[10px] text-on-surface rounded-md shadow-xl border border-outline-variant/20 opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none z-10">
                      Specify the login page or main dashboard. Vessel handles isolated cookie jars for this URL.
                    </div>
                  </div>
                </div>
                <div class="relative group">
                  <input 
                    id="target-url" 
                    type="url" 
                    bind:value={newAppUrl} 
                    placeholder="https://app.notion.so" 
                    aria-required="true"
                    on:blur={checkUrlAccessibility}
                    class="w-full bg-surface-container-lowest border {urlStatus === 'valid' ? 'border-primary/50' : (urlStatus === 'invalid' ? 'border-error/50' : 'border-outline-variant/30')} rounded-lg p-3 text-sm text-on-surface focus:outline-none focus:ring-1 focus:ring-primary focus:border-primary transition-all shadow-inner"
                  />
                  <div class="absolute right-3 top-1/2 -translate-y-1/2 transition-opacity pointer-events-none flex items-center gap-2">
                    {#if urlStatus === 'checking'}
                      <span class="material-symbols-outlined text-sm animate-spin text-primary">progress_activity</span>
                    {:else if urlStatus === 'valid'}
                      <span class="material-symbols-outlined text-sm text-primary">verified</span>
                    {:else if urlStatus === 'invalid'}
                      <span class="material-symbols-outlined text-sm text-error">link_off</span>
                    {:else if newAppUrl.length > 0}
                      <span class="material-symbols-outlined text-sm {isUrlValid ? 'text-primary' : 'text-error'}">{isUrlValid ? 'link' : 'link_off'}</span>
                    {/if}
                  </div>
                </div>
                {#if urlStatus === 'checking'}
                  <p class="text-[9px] text-primary animate-pulse font-bold">Checking URL accessibility...</p>
                {:else if urlStatus === 'valid'}
                  <p class="text-[9px] text-primary font-bold">✓ URL accessible (Vessel Ready)</p>
                {:else if urlStatus === 'invalid'}
                  <p class="text-[9px] text-error font-bold">✗ Invalid URL format or unreachable</p>
                {:else}
                  <p class="text-[9px] text-on-surface-variant italic">Protocols like <code class="bg-surface-container-highest px-1 rounded">https://</code> are automatically validated.</p>
                {/if}
              </div>

              <!-- Icon Field & Preview -->
              <div class="flex flex-col sm:flex-row gap-6">
                <div class="flex-1 space-y-1.5">
                  <label for="icon-character" class="text-[10px] text-outline uppercase tracking-widest font-bold">
                    App Identity (Icon)
                  </label>
                  <input 
                    id="icon-character" 
                    type="text" 
                    bind:value={newAppIcon} 
                    maxlength="2" 
                    placeholder="C" 
                    class="w-full bg-surface-container-lowest border border-outline-variant/30 rounded-lg p-3 text-sm text-on-surface focus:outline-none focus:ring-1 focus:ring-primary focus:border-primary transition-all shadow-inner"
                  />
                  <p class="text-[9px] text-on-surface-variant">Type a single letter or paste an emoji.</p>
                </div>
                
                <div class="flex flex-col items-center justify-center p-4 bg-surface-container-low border border-outline-variant/10 rounded-xl min-w-[120px]">
                  <span class="text-[9px] text-outline uppercase tracking-widest mb-3 font-bold">Live Preview</span>
                  <div class="w-16 h-16 rounded-2xl bg-surface-container-highest flex items-center justify-center text-3xl font-bold border border-outline-variant/30 text-on-surface shadow-lg group hover:scale-105 transition-transform">
                    {newAppIcon || (newAppName ? newAppName.charAt(0).toUpperCase() : '?')}
                  </div>
                  <span class="text-[8px] text-outline-variant uppercase mt-3 tracking-tighter">Sidebar Appearance</span>
                </div>
              </div>

              {#if formError}
                <div class="p-4 rounded-lg bg-error/10 border border-error/20 flex items-start gap-3 animate-in fade-in slide-in-from-top-1">
                  <span class="material-symbols-outlined text-error text-lg">error_outline</span>
                  <div>
                    <p class="text-xs text-error font-bold uppercase tracking-wider mb-0.5">Validation Error</p>
                    <p class="text-xs text-error/80">{formError}</p>
                  </div>
                </div>
              {/if}

              <div class="pt-4 border-t border-outline-variant/10 flex items-center justify-between">
                <div class="flex items-center gap-4">
                  <button
                    on:click={addNewApp}
                    class="px-8 py-3 bg-primary text-on-primary font-label font-bold rounded-lg text-xs uppercase tracking-[0.15em] hover:brightness-110 active:scale-95 transition-all disabled:opacity-30 disabled:cursor-not-allowed shadow-xl shadow-primary/20 flex items-center gap-2"
                    disabled={!isNameValid || !isUrlValid || isDeploying || showSuccess}
                  >
                    {#if isDeploying}
                      <span class="material-symbols-outlined animate-spin text-sm">progress_activity</span>
                      Deploying...
                    {:else}
                      Deploy Container
                    {/if}
                  </button>
                  <button
                    on:click={resetForm}
                    class="px-4 py-3 text-on-surface-variant hover:text-on-surface font-label font-bold text-xs uppercase tracking-widest transition-colors disabled:opacity-30"
                    disabled={isDeploying}
                  >
                    Reset Form
                  </button>
                </div>
              </div>
            </div>
          </div>

          <!-- Success / Launch State -->
          <div class="space-y-6">
            {#if showSuccess && lastDeployedAppId}
              <div class="bg-primary/5 border border-primary/20 rounded-xl p-8 flex flex-col items-center text-center animate-in zoom-in-95 duration-500 shadow-2xl shadow-primary/5">
                <div class="w-16 h-16 rounded-full bg-primary/10 flex items-center justify-center mb-6">
                  <span class="material-symbols-outlined text-primary text-3xl">task_alt</span>
                </div>
                <h3 class="font-headline font-bold text-xl text-on-surface mb-2">Instance Ready</h3>
                <p class="text-on-surface-variant text-sm mb-8">The isolated container for <span class="text-primary font-bold">"{apps.find(a => a.id === lastDeployedAppId)?.name}"</span> has been successfully provisioned.</p>
                
                <div class="w-full space-y-3">
                  <button 
                    on:click={() => {
                      const app = apps.find(a => a.id === lastDeployedAppId);
                      if (app) launch(app);
                      showSuccess = false;
                    }}
                    class="w-full py-3 bg-primary text-on-primary rounded-lg font-label font-bold text-xs uppercase tracking-widest hover:brightness-110 active:scale-95 transition-all shadow-lg shadow-primary/20"
                  >
                    Launch Now
                  </button>
                  <button 
                    on:click={() => { showSuccess = false; switchView('gallery'); }}
                    class="w-full py-3 bg-surface-container-highest text-on-surface rounded-lg font-label font-bold text-xs uppercase tracking-widest hover:bg-surface-bright transition-all border border-outline-variant/10"
                  >
                    View in Gallery
                  </button>
                </div>
              </div>
            {:else}
              <div class="bg-surface-container-lowest border border-outline-variant/10 rounded-xl p-6 space-y-4">
                <h4 class="text-[10px] text-outline uppercase font-bold tracking-[0.2em] mb-4">Deployment Guide</h4>
                <div class="flex gap-4">
                  <div class="w-6 h-6 rounded-full bg-surface-container-highest flex items-center justify-center text-[10px] font-bold text-outline shrink-0">1</div>
                  <p class="text-xs text-on-surface-variant leading-relaxed">Containers use <span class="text-on-surface font-medium">isolated sessions</span>. Your login state is never shared between instances.</p>
                </div>
                <div class="flex gap-4">
                  <div class="w-6 h-6 rounded-full bg-surface-container-highest flex items-center justify-center text-[10px] font-bold text-outline shrink-0">2</div>
                  <p class="text-xs text-on-surface-variant leading-relaxed">Performance is optimized through <span class="text-on-surface font-medium">Native Hibernation</span>. Inactive tabs sleep to save RAM.</p>
                </div>
                <div class="flex gap-4">
                  <div class="w-6 h-6 rounded-full bg-surface-container-highest flex items-center justify-center text-[10px] font-bold text-outline shrink-0">3</div>
                  <p class="text-xs text-on-surface-variant leading-relaxed">You can customize ad-blocking and <span class="text-on-surface font-medium">CSS injection</span> after deployment in the settings panel.</p>
                </div>
              </div>
              {/if}

              {#if showShortcuts}
              <div class="fixed inset-0 z-[100] flex items-center justify-center p-4">
                <div 
                  class="absolute inset-0 bg-background/80 backdrop-blur-sm" 
                  on:click={() => showShortcuts = false}
                  role="button"
                  tabindex="0"
                  on:keydown={(e) => e.key === 'Escape' && (showShortcuts = false)}
                  aria-label="Close shortcuts modal"
                ></div>
                <div class="relative bg-surface-container border border-outline-variant/20 rounded-2xl shadow-2xl w-full max-w-md overflow-hidden animate-in zoom-in-95 duration-200">
                  <div class="p-6 border-b border-outline-variant/10 flex items-center justify-between">
                    <div class="flex items-center gap-3">
                      <span class="material-symbols-outlined text-primary">keyboard</span>
                      <h2 class="text-xl font-bold text-on-surface">Keyboard Shortcuts</h2>
                    </div>
                    <button on:click={() => showShortcuts = false} class="p-1 rounded-full hover:bg-surface-container-highest transition-colors text-outline">
                      <span class="material-symbols-outlined">close</span>
                    </button>
                  </div>
                  <div class="p-6 space-y-4">
                    {#each keyboardShortcuts as shortcut}
                      <div class="flex items-center justify-between group">
                        <span class="text-sm text-on-surface-variant group-hover:text-on-surface transition-colors">{shortcut.action}</span>
                        <kbd class="px-2 py-1 rounded bg-surface-container-highest border border-outline-variant/30 text-[10px] font-mono font-bold text-primary shadow-sm">{shortcut.key}</kbd>
                      </div>
                    {/each}
                  </div>
                  <div class="bg-surface-container-high p-4 flex justify-center">
                    <p class="text-[10px] text-outline-variant uppercase tracking-widest font-bold italic">Vessel Control Plane v0.1.0</p>
                  </div>
                </div>
              </div>
              {/if}
              </div>
        </div>
      </div>
    {/if}

    {#if currentView === 'settings' && editingApp}
      <div class="max-w-6xl mx-auto flex flex-col bg-surface-container shadow-2xl rounded-xl overflow-hidden border border-outline-variant/15 mt-2 sm:mt-4 relative">
        
        {#if initialSettingsState && JSON.stringify(editingApp) !== initialSettingsState}
          <div class="bg-primary/10 border-b border-primary/20 px-6 py-2 flex items-center justify-between animate-in slide-in-from-top-2 duration-300">
            <div class="flex items-center gap-2 text-primary">
              <span class="material-symbols-outlined text-sm">warning</span>
              <span class="text-[10px] font-bold uppercase tracking-widest">You have unsaved changes in this container</span>
            </div>
            <div class="flex items-center gap-4">
              <button on:click={() => editingApp = JSON.parse(initialSettingsState)} class="text-[9px] font-bold text-on-surface-variant hover:text-on-surface uppercase tracking-tighter">Reset All</button>
            </div>
          </div>
        {/if}

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
            <button 
              on:click={() => { if (editingApp) launch(editingApp); }} 
              class="px-4 py-1.5 text-xs font-semibold bg-surface-container-highest rounded border border-outline-variant/20 hover:text-primary transition-colors text-on-surface flex items-center gap-2"
              title="Preview settings in a live window"
            >
              <span class="material-symbols-outlined text-sm">rocket_launch</span>
              Test Settings Live
            </button>
            <div class="h-6 w-px bg-outline-variant/20 mx-1"></div>
            <button on:click={() => switchView('gallery')} class="px-4 py-1.5 text-xs font-semibold bg-surface-container-highest rounded border border-outline-variant/20 hover:bg-surface-bright transition-colors text-on-surface disabled:opacity-30" disabled={isSaving}>Discard Changes</button>
            <button 
              on:click={saveSettings} 
              class="px-4 py-1.5 text-xs font-semibold bg-primary text-on-primary rounded shadow-lg shadow-primary-container/20 hover:brightness-110 transition-all disabled:opacity-30 flex items-center gap-2"
              disabled={isSaving}
            >
              {#if isSaving}
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

            {#if !settingsSearch || 'appearance'.includes(settingsSearch.toLowerCase())}
              <div class="px-6 py-3 flex items-center space-x-3 bg-primary/10 text-primary border-r-2 border-primary">
                <span class="material-symbols-outlined text-sm">palette</span>
                <span class="text-sm font-bold">Appearance</span>
              </div>
            {/if}

            {#if editingApp.url.includes('youtube.com') && (!settingsSearch || 'ad-blocking'.includes(settingsSearch.toLowerCase()))}
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
            {#if !settingsSearch || 'appearance'.includes(settingsSearch.toLowerCase()) || 'session profile'.includes(settingsSearch.toLowerCase()) || 'sleep delay'.includes(settingsSearch.toLowerCase()) || 'theme'.includes(settingsSearch.toLowerCase())}
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
                      {#if JSON.parse(initialSettingsState).features.profile !== editingApp.features.profile}
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
                        bind:value={editingApp.features.profile}
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
                      {#if JSON.parse(initialSettingsState).features.idleSleepSeconds !== editingApp.features.idleSleepSeconds}
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
                        <input type="range" min="0" max="120" step="5" bind:value={editingApp.features.idleSleepSeconds} class="w-full accent-primary h-1.5 rounded-lg bg-surface-container-lowest cursor-pointer" />
                      </div>
                      <div class="flex items-center gap-1 bg-surface-container-lowest px-3 py-1.5 rounded border border-outline-variant/30 min-w-[70px] justify-center shadow-inner">
                        <input type="number" bind:value={editingApp.features.idleSleepSeconds} class="w-8 bg-transparent text-xs font-mono font-bold text-primary focus:outline-none text-center" />
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
                      {#if JSON.parse(initialSettingsState).features.theme !== editingApp.features.theme}
                        <span class="w-1.5 h-1.5 rounded-full bg-primary animate-in zoom-in"></span>
                      {/if}
                    </div>
                    <p class="text-sm text-on-surface-variant">Override site colors with high-contrast native palettes.</p>
                  </div>
                  <div class="relative w-full sm:w-64">
                    <select bind:value={editingApp.features.theme} class="appearance-none bg-surface-container-lowest border border-outline-variant/30 rounded-lg p-2.5 pr-10 text-sm text-on-surface focus:outline-none focus:ring-1 focus:ring-primary w-full shadow-inner">
                      <option value="default">None (Default Rendering)</option>
                      <option value="dark_invert">Force Native Dark</option>
                      <option value="oled">Total OLED Black (#000)</option>
                    </select>
                    <span class="material-symbols-outlined absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none text-outline">expand_more</span>
                  </div>
                </div>

                {#if editingApp.url.includes("youtube.com")}
                  <div class="flex items-center justify-between p-4 sm:p-6 rounded-lg bg-surface-container-high border border-outline-variant/10 group hover:border-primary/20 transition-all">
                    <div>
                      <div class="flex items-center gap-2">
                        <h3 class="font-bold text-on-surface">YouTube Shield (Ad-Block)</h3>
                        {#if JSON.parse(initialSettingsState).features.adblock !== editingApp.features.adblock}
                          <span class="w-1.5 h-1.5 rounded-full bg-primary animate-in zoom-in"></span>
                        {/if}
                      </div>
                      <p class="text-sm text-on-surface-variant">Intercept pre-roll ads and skip promotional overlays.</p>
                    </div>
                    <label class="relative inline-flex items-center cursor-pointer">
                      <input type="checkbox" bind:checked={editingApp.features.adblock} class="sr-only peer">
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
                        {#if JSON.parse(initialSettingsState).features.customCss !== editingApp.features.customCss}
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
                      bind:value={editingApp.features.customCss}
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
                      {#if JSON.parse(initialSettingsState).features.customJs !== editingApp.features.customJs}
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
                      bind:value={editingApp.features.customJs}
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
                      {#if JSON.parse(initialSettingsState).features.injectionAllowlist !== editingApp.features.injectionAllowlist}
                        <span class="w-1.5 h-1.5 rounded-full bg-primary animate-in zoom-in"></span>
                      {/if}
                    </div>
                    <p class="text-sm text-on-surface-variant">Restrict code injection to specific subdomains. Separate by comma.</p>
                  </div>
                  <textarea
                    bind:value={editingApp.features.injectionAllowlist}
                    rows="3"
                    spellcheck="false"
                    placeholder="example.com, auth.example.com"
                    class="w-full bg-surface-container-lowest border border-outline-variant/30 rounded-lg p-4 text-xs sm:text-sm text-on-surface font-mono focus:outline-none focus:ring-1 focus:ring-primary shadow-inner"
                  ></textarea>
                </div>
              </div>
            </section>
            {/if}
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

  {#if showDiagnostics}
    <div class="fixed top-14 right-0 bottom-0 w-full sm:w-[360px] lg:w-[420px] glass-panel z-[75] border-l border-outline-variant/15 flex flex-col shadow-2xl">
      <div class="p-4 border-b border-outline-variant/10 flex items-center justify-between">
        <div>
          <h3 class="text-lg font-bold text-on-surface">Diagnostics</h3>
          <p class="text-[10px] uppercase tracking-widest text-on-surface-variant">IPC and Injection Events</p>
        </div>
        <button
          on:click={() => diagnostics = []}
          class="bg-surface-container-highest hover:bg-surface-container-high text-on-surface-variant px-3 py-1 rounded text-[10px] uppercase tracking-widest border border-outline-variant/20"
        >
          Clear
        </button>
      </div>

      <div class="px-4 py-2 text-xs text-on-surface-variant border-b border-outline-variant/10">
        Safe mode: <span class={safeMode ? 'text-error' : 'text-primary'}>{safeMode ? 'ON' : 'OFF'}</span>
      </div>

      <div class="flex-grow overflow-y-auto vessel-scroll p-4 space-y-3">
        {#if diagnostics.length === 0}
          <div class="text-sm text-outline-variant text-center mt-10">No diagnostics yet.</div>
        {/if}
        {#each diagnostics as entry}
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



</div>