import { writable } from 'svelte/store';
import type { AppSettings, CardDensity } from '$lib/types';

const STORAGE_KEY = 'vessel_app_settings';

const defaultSettings: AppSettings = {
  density: 'minimal', // "minimal" (default) and "detailed"
  isDemoMode: false,
  globalIdleThresholdMinutes: 15,
};

function loadSettings(): AppSettings {
  if (typeof window === 'undefined' || !window.localStorage) {
    return defaultSettings;
  }
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return defaultSettings;
    const parsed = JSON.parse(raw);
    return {
      density: parsed.density === 'detailed' ? 'detailed' : 'minimal',
      isDemoMode: Boolean(parsed.isDemoMode),
      globalIdleThresholdMinutes:
        typeof parsed.globalIdleThresholdMinutes === 'number'
          ? parsed.globalIdleThresholdMinutes
          : 15,
    };
  } catch {
    return defaultSettings;
  }
}

export const appSettings = writable<AppSettings>(loadSettings());

export function updateAppSettings(partial: Partial<AppSettings>) {
  appSettings.update(current => {
    const updated = { ...current, ...partial };
    if (typeof window !== 'undefined' && window.localStorage) {
      try {
        localStorage.setItem(STORAGE_KEY, JSON.stringify(updated));
      } catch (e) {
        console.error('Failed to save settings:', e);
      }
    }
    return updated;
  });
}

export function toggleDensity() {
  appSettings.update(current => {
    const nextDensity: CardDensity = current.density === 'minimal' ? 'detailed' : 'minimal';
    const updated = { ...current, density: nextDensity };
    if (typeof window !== 'undefined' && window.localStorage) {
      try {
        localStorage.setItem(STORAGE_KEY, JSON.stringify(updated));
      } catch {}
    }
    return updated;
  });
}

export function setDemoMode(enabled: boolean) {
  updateAppSettings({ isDemoMode: enabled });
}
