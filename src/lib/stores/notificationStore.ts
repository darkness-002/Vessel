import { writable, derived } from 'svelte/store';
import type { VesselNotification } from '$lib/types';
import { showBrain } from './uiStore';

export const notifications = writable<VesselNotification[]>([]);
export const unreadCount = writable<number>(0);
export const brainSearch = writable<string>('');
export const brainAppFilter = writable<string>('all');

export const notificationApps = derived(notifications, ($notifications) => 
  Array.from(new Set($notifications.map((note) => note.appId))).sort()
);

export const filteredNotifications = derived(
  [notifications, brainAppFilter, brainSearch],
  ([$notifications, $brainAppFilter, $brainSearch]) => {
    return $notifications.filter((note) => {
      const matchesApp = $brainAppFilter === 'all' || note.appId === $brainAppFilter;
      const searchValue = $brainSearch.trim().toLowerCase();
      const matchesSearch = !searchValue
        || note.title.toLowerCase().includes(searchValue)
        || note.body.toLowerCase().includes(searchValue)
        || note.appId.toLowerCase().includes(searchValue);
      return matchesApp && matchesSearch;
    });
  }
);
