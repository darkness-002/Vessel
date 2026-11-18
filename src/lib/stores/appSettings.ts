import { writable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import type { AppSettings, CardDensity, ThemeMode, DashboardViewMode } from '$lib/types';

const STORAGE_KEY = 'vessel_app_settings';

export const defaultSettings: AppSettings = {
  theme: 'monochrome',
  viewMode: 'simple',
  density: 'minimal',
  isDemoMode: false,
  globalIdleThresholdMinutes: 15,
  muteAllNotifications: false,
};

export function applyTheme(theme: ThemeMode) {
  if (typeof window !== 'undefined' && document?.documentElement) {
    document.documentElement.setAttribute('data-theme', theme);
  }
}

// Ensure default Monochrome theme is immediately applied to document upon module load
if (typeof window !== 'undefined') {
  applyTheme('monochrome');
}

function loadSettings(): AppSettings {
  if (typeof window === 'undefined' || !window.localStorage) {
    return defaultSettings;
  }
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) {
      applyTheme(defaultSettings.theme);
      return defaultSettings;
    }
    const parsed = JSON.parse(raw);
    const theme: ThemeMode = ['monochrome', 'paper'].includes(parsed.theme)
      ? parsed.theme
      : 'monochrome';
    const viewMode: DashboardViewMode =
      parsed.viewMode === 'detailed' || parsed.density === 'detailed' ? 'detailed' : 'simple';
    const density: CardDensity = viewMode === 'detailed' ? 'detailed' : 'minimal';

    applyTheme(theme);

    return {
      theme,
      viewMode,
      density,
      isDemoMode: Boolean(parsed.isDemoMode),
      globalIdleThresholdMinutes:
        typeof parsed.globalIdleThresholdMinutes === 'number'
          ? parsed.globalIdleThresholdMinutes
          : 15,
      muteAllNotifications: Boolean(parsed.muteAllNotifications),
    };
  } catch {
    applyTheme(defaultSettings.theme);
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
      muteAllNotifications?: boolean;
    }>('get_settings');
    if (backendSettings) {
      appSettings.update(current => {
        const merged: AppSettings = {
          ...current,
          globalIdleThresholdMinutes:
            backendSettings.globalIdleThresholdMinutes ?? current.globalIdleThresholdMinutes,
          isDemoMode: backendSettings.isDemoMode ?? current.isDemoMode,
          muteAllNotifications:
            backendSettings.muteAllNotifications !== undefined
              ? backendSettings.muteAllNotifications
              : current.muteAllNotifications,
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
    let viewMode = partial.viewMode ?? current.viewMode;
    let density = partial.density ?? current.density;

    if (partial.viewMode && !partial.density) {
      density = partial.viewMode === 'detailed' ? 'detailed' : 'minimal';
    } else if (partial.density && !partial.viewMode) {
      viewMode = partial.density === 'detailed' ? 'detailed' : 'simple';
    }

    const updated: AppSettings = {
      ...current,
      ...partial,
      viewMode,
      density,
    };

    if (updated.theme) {
      applyTheme(updated.theme);
    }

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
      (partial.globalIdleThresholdMinutes !== undefined ||
        partial.isDemoMode !== undefined ||
        partial.muteAllNotifications !== undefined)
    ) {
      invoke('update_settings', {
        settings: {
          globalIdleThresholdMinutes: updated.globalIdleThresholdMinutes,
          isDemoMode: updated.isDemoMode,
          muteAllNotifications: updated.muteAllNotifications,
          lockAllShortcut: 'Ctrl+Shift+L',
        },
      }).catch(err => {
        console.debug('Backend update_settings non-fatal:', err);
      });
    }
    return updated;
  });
}

export function toggleDensity() {
  appSettings.update(current => {
    const nextDensity: CardDensity = current.density === 'minimal' ? 'detailed' : 'minimal';
    const nextView: DashboardViewMode = nextDensity === 'detailed' ? 'detailed' : 'simple';
    const updated = { ...current, density: nextDensity, viewMode: nextView };
    if (typeof window !== 'undefined' && window.localStorage) {
      try {
        localStorage.setItem(STORAGE_KEY, JSON.stringify(updated));
      } catch {}
    }
    return updated;
  });
}

export function setTheme(theme: ThemeMode) {
  updateAppSettings({ theme });
}

export function setViewMode(viewMode: DashboardViewMode) {
  updateAppSettings({ viewMode, density: viewMode === 'detailed' ? 'detailed' : 'minimal' });
}

export function setDemoMode(enabled: boolean) {
  updateAppSettings({ isDemoMode: enabled });
}
