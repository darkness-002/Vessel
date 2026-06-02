import { writable, get, derived } from 'svelte/store';
import type { BrowserTab, AppConfig } from '$lib/types';
import { invoke } from '@tauri-apps/api/core';
import { apps, getSiteOptimizations, toDisplayName, normalizeServiceUrl } from './appStore';
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

export async function switchToTab(tabId: string) {
  const $tabs = get(tabs);
  const tab = $tabs.find((item) => item.id === tabId);
  if (!tab) return { ok: false as const };
  
  const $apps = get(apps);
  const app = $apps.find((item) => item.id === tab.appId);
  if (!app) return { ok: false as const };

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
  const $tabs = get(tabs);
  const tab = $tabs.find((item) => item.id === tabId);
  if (!tab) return;

  await invoke('close_webview', { id: tab.id });
  tabs.update($tabs => $tabs.filter((item) => item.id !== tabId));

  const $newTabs = get(tabs);
  if ($newTabs.length === 0) {
    activeTabId.set('');
    activeId.set('');
    currentView.set('gallery');
    return;
  }

  if (get(activeTabId) === tabId) {
    const sameAppTabs = $newTabs.filter((item) => item.appId === tab.appId);
    if (sameAppTabs.length > 0) {
      await switchToTab(sameAppTabs[sameAppTabs.length - 1].id);
    } else {
      await switchToTab($newTabs[$newTabs.length - 1].id);
    }
  }
}
