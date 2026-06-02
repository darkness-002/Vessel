<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event"; 
  import { load } from "@tauri-apps/plugin-store";
  import { goto } from "$app/navigation";
  import { deserializeAppsBackup, serializeAppsBackup } from "$lib/settingsPersistence";

  // Stores
  import { apps, normalizeApp, persistApps, editingApp, initialSettingsState } from "$lib/stores/appStore";
  import { tabs, activeTabId, addTabForApp, switchToTab } from "$lib/stores/tabStore";
  import { currentView, activeId, showBrain, showDiagnostics, showShortcuts, safeMode, showPerformanceStats, updateViewport, switchView, toggleBrain, toggleDiagnostics, setPerformanceStats, setSafeMode } from "$lib/stores/uiStore";
  import { notifications, unreadCount } from "$lib/stores/notificationStore";
  import { diagnostics } from "$lib/stores/diagnosticStore";

  // Components
  import Sidebar from "$lib/components/Sidebar.svelte";
  import Header from "$lib/components/Header.svelte";
  import Gallery from "$lib/components/Gallery.svelte";
  import AddApp from "$lib/components/AddApp.svelte";
  import Settings from "$lib/components/Settings.svelte";
  import BrainPanel from "$lib/components/BrainPanel.svelte";
  import DiagnosticsPanel from "$lib/components/DiagnosticsPanel.svelte";
  import ShortcutOverlay from "$lib/components/ShortcutOverlay.svelte";

  import type { AppConfig, VesselNotification, NewTabEvent, DiagnosticEvent } from "$lib/types";

  let store: any;
  let storeReady = false;

  function handleKeydown(e: KeyboardEvent) {
    if (e.ctrlKey) {
      switch (e.key.toLowerCase()) {
        case 'g': e.preventDefault(); goHome(); break;
        case 'n': e.preventDefault(); switchView('add-app'); break;
        case 'b': e.preventDefault(); toggleBrain(); break;
        case 'd': e.preventDefault(); toggleDiagnostics(); break;
        case 's': e.preventDefault(); setSafeMode(!$safeMode, store); break;
      }
    }
    if (e.key === 'Escape') {
      if ($showBrain) showBrain.set(false);
      else if ($showDiagnostics) showDiagnostics.set(false);
      else if ($showShortcuts) showShortcuts.set(false);
      else if ($currentView !== 'gallery') switchView('gallery');
    }
  }

  async function goHome() {
    if ($currentView === 'settings' && $initialSettingsState && JSON.stringify($editingApp) !== $initialSettingsState) {
      if (!confirm('You have unsaved changes. Discard them?')) return;
    }
    await goto('/');
    activeId.set('');
    switchView('gallery');
  }

  function loadAppsBackup() {
    return deserializeAppsBackup<AppConfig>(localStorage.getItem('vessel_apps_backup'));
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
      
      const normalizedApps = rawApps.map((app) => normalizeApp(app as any));
      if (!isUnmounted) {
        apps.set(normalizedApps);
      }
      
      const needsSync = storeApps.length !== backupApps.length;
      const needsMigration = rawApps.some((app: any) =>
        !app.features
          || app.features.profile === undefined
          || app.features.customCss === undefined
          || app.features.customJs === undefined
          || app.features.injectionAllowlist === undefined
          || app.features.idleSleepSeconds === undefined
      );

      if (!savedApps || needsMigration || needsSync) {
        await persistApps(normalizedApps, store);
      } else {
        localStorage.setItem('vessel_apps_backup', serializeAppsBackup(normalizedApps));
      }

      const storedPerf = await store.get('showPerformanceStats');
      if (!isUnmounted) {
        showPerformanceStats.set(Boolean(storedPerf));
      }
      if ($showPerformanceStats) {
        await setPerformanceStats(true, store);
      }

      const storedSafeMode = await store.get('safeMode');
      safeMode.set(Boolean(storedSafeMode));
      await invoke('set_safe_mode', { enabled: $safeMode });

      const dbNotifications = await invoke<VesselNotification[]>('get_notifications', { limit: 50 });
      if (!isUnmounted) {
        notifications.set(dbNotifications);
      }

      const unlistenNotif = await listen("vessel-notification", (event: { payload: VesselNotification }) => {
        notifications.update(n => [event.payload, ...n].slice(0, 50));
        if (!$showBrain) unreadCount.update(c => c + 1);
      });

      const unlistenTabs = await listen('vessel-open-tab', (event: { payload: NewTabEvent }) => {
        const app = $apps.find((item) => item.id === event.payload.appId);
        if (app) addTabForApp(app, event.payload.url, event.payload.title || 'New Tab');
      });

      const unlistenDiagnostics = await listen('vessel-diagnostic', (event: { payload: DiagnosticEvent }) => {
        diagnostics.update(d => [event.payload, ...d].slice(0, 120));
      });

      unlisten = () => {
        unlistenNotif();
        unlistenTabs();
        unlistenDiagnostics();
      };
    })();

    return () => {
      isUnmounted = true;
      window.removeEventListener('resize', updateViewport);
      unlisten();
    };
  });
</script>

<div class="bg-background text-on-background font-body select-none overflow-hidden h-screen w-screen flex flex-col">
  
  <Sidebar />

  <Header {store} />

  <main
    class="ml-14 sm:ml-16 mb-0 p-4 sm:p-6 lg:p-8 overflow-y-auto relative bg-background"
    style:height="calc(100vh - 56px)"
  >
    
    {#if $currentView === 'webview'}
      <div class="w-full h-full flex flex-col items-center justify-center text-outline-variant opacity-30">
        <span class="material-symbols-outlined text-6xl mb-4">memory</span>
        <span class="font-headline tracking-widest uppercase">Native Container Active</span>
      </div>
    {/if}

    {#if $currentView === 'gallery'}
      <Gallery {store} />
    {/if}

    {#if $currentView === 'add-app'}
      <AddApp {store} />
    {/if}

    {#if $currentView === 'settings'}
      <Settings {store} />
    {/if}
  </main>

  <BrainPanel />

  <DiagnosticsPanel />

  <ShortcutOverlay />

</div>
