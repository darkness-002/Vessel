import { writable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
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

export async function syncSettingsFromBackend(): Promise<void> {
  if (typeof window === 'undefined') return;
  try {
    const backendSettings = await invoke<{
      globalIdleThresholdMinutes: number;
      isDemoMode: boolean;
    }>('get_settings');
    if (backendSettings) {
      appSettings.update(current => {
        const merged: AppSettings = {
          ...current,
          globalIdleThresholdMinutes:
            backendSettings.globalIdleThresholdMinutes ?? current.globalIdleThresholdMinutes,
          isDemoMode: backendSettings.isDemoMode ?? current.isDemoMode,
        };
        try {
          localStorage.setItem(STORAGE_KEY, JSON.stringify(merged));
        } catch {}
        return merged;
      });
    }
  } catch {
    // In headless test environments or non-Tauri browser preview, ignore
  }
}

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
    // Push settings to Rust backend settings.json
    if (
      typeof window !== 'undefined' &&
      (partial.globalIdleThresholdMinutes !== undefined || partial.isDemoMode !== undefined)
    ) {
      invoke('update_settings', {
        settings: {
          globalIdleThresholdMinutes: updated.globalIdleThresholdMinutes,
          isDemoMode: updated.isDemoMode,
        },
      }).catch(err => {
        // Expected when running in browser preview without Tauri core
        console.debug('Backend update_settings non-fatal:', err);
      });
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
