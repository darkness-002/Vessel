import { describe, it, expect } from 'vitest';
import type { ContainerConfig } from '$lib/types';
import { toasts, showToast, dismissToast } from '$lib/stores/toastStore';
import { get } from 'svelte/store';

describe('Vessel Frontend Sanity', () => {
  it('conforms to ContainerConfig schema', () => {
    const container: ContainerConfig = {
      id: 'chatgpt',
      name: 'ChatGPT',
      url: 'https://chatgpt.com',
      icon: null,
      createdAt: '2026-10-20T12:00:00Z'
    };

    expect(container.id).toBe('chatgpt');
    expect(container.name).toBe('ChatGPT');
    expect(container.url).toBe('https://chatgpt.com');
  });

  it('manages toast notifications correctly', () => {
    expect(get(toasts)).toEqual([]);
    
    showToast('Sandbox launched', 'success', 0);
    const current = get(toasts);
    expect(current.length).toBe(1);
    expect(current[0].message).toBe('Sandbox launched');
    expect(current[0].type).toBe('success');

    dismissToast(current[0].id);
    expect(get(toasts)).toEqual([]);
  });
});
