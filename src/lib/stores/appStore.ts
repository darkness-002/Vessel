import { writable, get } from 'svelte/store';
import type { AppConfig } from '$lib/types';
import { serializeAppsBackup, normalizeFeatures } from '$lib/settingsPersistence';
import { invoke } from '@tauri-apps/api/core';

export const apps = writable<AppConfig[]>([]);
export const editingApp = writable<AppConfig | null>(null);
export const initialSettingsState = writable<string>('');
export const isSaving = writable<boolean>(false);

export const newAppForm = writable({
  name: '',
  url: '',
  icon: ''
});

export function buildAppId(name: string) {
  return name
    .toLowerCase()
    .trim()
    .replace(/[^a-z0-9\s-]/g, '')
    .replace(/\s+/g, '-')
    .replace(/-+/g, '-');
}

export function normalizeServiceUrl(url: string) {
  const ensureProtocol = (u: string) => /^https?:\/\//i.test(u) ? u : `https://${u}`;
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

export function normalizeApp(input: Partial<AppConfig> & { id: string; icon: string; url: string }): AppConfig {
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

export async function persistApps(nextApps: AppConfig[], store: any) {
  localStorage.setItem('vessel_apps_backup', serializeAppsBackup(nextApps));

  if (!store) return;
  try {
    await store.set('apps', nextApps);
    if (typeof store.save === 'function') {
      await store.save();
    }
  } catch (error) {
    console.error('Failed to persist apps in store, backup kept in localStorage', error);
  }
}

export function toDisplayName(app: AppConfig) {
  return app.name || app.id.replace(/-/g, ' ');
}

export function getSiteOptimizations(targetUrl: string) {
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
