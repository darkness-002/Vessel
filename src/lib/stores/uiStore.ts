import { writable } from 'svelte/store';
import type { AppView, ResourceUsage } from '$lib/types';
import { invoke } from '@tauri-apps/api/core';

export const currentView = writable<AppView>('gallery');
export const activeId = writable<string>('');
export const showBrain = writable<boolean>(false);
export const showDiagnostics = writable<boolean>(false);
export const showShortcuts = writable<boolean>(false);
export const safeMode = writable<boolean>(false);
export const showPerformanceStats = writable<boolean>(false);
export const usage = writable<ResourceUsage>({ cpuPercent: 0, ramMb: 0 });
export const viewportWidth = writable<number>(0);
export const viewportHeight = writable<number>(0);

let usageTimer: ReturnType<typeof setInterval> | null = null;

export async function setPerformanceStats(enabled: boolean, store: any) {
  showPerformanceStats.set(enabled);
  if (store) {
    await store.set('showPerformanceStats', enabled);
  }

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

async function refreshUsage() {
  try {
    const next = await invoke<ResourceUsage>('get_resource_usage');
    usage.set(next);
  } catch (error) {
    console.error('Failed to refresh usage', error);
  }
}

export async function setSafeMode(enabled: boolean, store: any) {
  safeMode.set(enabled);
  if (store) {
    await store.set('safeMode', enabled);
  }
  await invoke('set_safe_mode', { enabled });
}

export function updateViewport() {
  viewportWidth.set(window.innerWidth);
  viewportHeight.set(window.innerHeight);
}

export function switchView(view: AppView) {
  currentView.set(view);
  if (view !== 'settings') {
    // editingApp will be handled in appStore or local to component if preferred
  }
  invoke("hide_all_webviews");
}

export function toggleBrain() {
  showBrain.update(v => !v);
}

export function toggleDiagnostics() {
  showDiagnostics.update(v => !v);
}

export function toggleShortcuts() {
  showShortcuts.update(v => !v);
}
