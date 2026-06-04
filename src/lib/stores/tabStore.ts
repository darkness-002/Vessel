import { writable, get, derived } from 'svelte/store';
import type { BrowserTab, AppConfig } from '$lib/types';
import { invoke } from '@tauri-apps/api/core';
import { apps, toDisplayName, normalizeServiceUrl } from './appStore';
import { getSiteOptimizations } from '$lib/siteOptimizations';
import { activeId, currentView } from './uiStore';

export const tabs = writable<BrowserTab[]>([]);
export const activeTabId = writable<string>('');
export const tabSeed = writable<number>(0);

export const activeTab = derived(
  [tabs, activeTabId],
  ([$tabs, $activeTabId]) => $tabs.find((tab) => tab.id === $activeTabId) || null
);

export const activeApp = derived(
  [activeTab, apps],
  ([$activeTab, $apps]) => $activeTab ? $apps.find((app) => app.id === $activeTab.appId) || null : null
);

export const visibleTabs = derived(
  [activeId, tabs],
  ([$activeId, $tabs]) => $activeId ? $tabs.filter((tab) => tab.appId === $activeId) : []
);

const themes: Record<string, string> = {
  default: "",
  dark_invert: "body { filter: invert(1) hue-rotate(180deg); background: black !important; }",
  oled: "body { background-color: #000000 !important; color: #ffffff !important; }"
};

function nextTabId(appId: string) {
  tabSeed.update(s => s + 1);
  return `${appId}__tab_${Date.now()}_${get(tabSeed)}`;
}

export async function openTab(app: AppConfig, tab: BrowserTab) {
  console.log('openTab called for tab:', tab.id, 'app:', app.id);
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

    console.log('Invoking open_app for:', tab.id);
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
    console.log('open_app success for:', tab.id);
    return { ok: true as const };
  } catch (error) {
    console.error('Failed to open tab webview', error);
    return { ok: false as const, error: String(error) };
  }
}

export async function switchToTab(tabId: string) {
  console.log('switchToTab:', tabId);
  const $activeTabId = get(activeTabId);
  const $currentView = get(currentView);

  // If already active and in webview mode, skip to avoid redundant IPC calls
  if ($activeTabId === tabId && $currentView === 'webview') {
    console.log('Tab is already active and visible, skipping redundant switch.');
    return { ok: true as const };
  }

  const $tabs = get(tabs);
  const tab = $tabs.find((item) => item.id === tabId);
  if (!tab) {
    console.warn('switchToTab failed: tab not found', tabId);
    return { ok: false as const };
  }
  
  const $apps = get(apps);
  const app = $apps.find((item) => item.id === tab.appId);
  if (!app) {
    console.warn('switchToTab failed: app not found', tab.appId);
    return { ok: false as const };
  }

  activeTabId.set(tab.id);
  activeId.set(app.id);
  currentView.set('webview');
  return openTab(app, tab);
}

export async function addTabForApp(app: AppConfig, url?: string, title?: string) {
  const tabUrl = normalizeServiceUrl(url || app.url);
  try {
    new URL(tabUrl);
  } catch {
    return { ok: false as const, error: 'Invalid URL for tab launch.' };
  }

  const tab: BrowserTab = {
    id: nextTabId(app.id),
    appId: app.id,
    title: title || toDisplayName(app),
    url: tabUrl
  };
  tabs.update($tabs => [...$tabs, tab]);
  const opened = await switchToTab(tab.id);
  if (!opened?.ok) {
    tabs.update($tabs => $tabs.filter((item) => item.id !== tab.id));
    if (get(tabs).length === 0) {
      activeTabId.set('');
      activeId.set('');
      currentView.set('gallery');
    }
    return opened;
  }
  return { ok: true as const };
}

export async function closeTab(tabId: string) {
  console.log('closeTab:', tabId);
  const $tabs = get(tabs);
  const tab = $tabs.find((item) => item.id === tabId);
  if (!tab) {
    console.warn('closeTab failed: tab not found', tabId);
    return;
  }

  console.log('Invoking close_webview for:', tab.id);
  try {
    await invoke('close_webview', { id: tab.id });
    console.log('close_webview success for:', tab.id);
  } catch (e) {
    console.error('Failed to close webview', tab.id, e);
  }

  tabs.update($tabs => $tabs.filter((item) => item.id !== tabId));

  const $newTabs = get(tabs);
  if ($newTabs.length === 0) {
    console.log('No tabs left, returning to gallery');
    activeTabId.set('');
    activeId.set('');
    currentView.set('gallery');
    return;
  }

  if (get(activeTabId) === tabId) {
    console.log('Active tab closed, switching to another...');
    const sameAppTabs = $newTabs.filter((item) => item.appId === tab.appId);
    if (sameAppTabs.length > 0) {
      await switchToTab(sameAppTabs[sameAppTabs.length - 1].id);
    } else {
      await switchToTab($newTabs[$newTabs.length - 1].id);
    }
  }
}
