import { writable } from 'svelte/store';

export type Toast = {
  id: string;
  message: string;
  type?: 'info' | 'success' | 'error';
  duration?: number;
};

export const toasts = writable<Toast[]>([]);

export function showToast(message: string, type: 'info' | 'success' | 'error' = 'info', duration = 4000) {
  const id = Math.random().toString(36).substring(2, 9);
  const toast: Toast = { id, message, type, duration };
  
  toasts.update(current => [...current, toast]);

  if (duration > 0) {
    setTimeout(() => {
      dismissToast(id);
    }, duration);
  }
}

export function dismissToast(id: string) {
  toasts.update(current => current.filter(t => t.id !== id));
}
