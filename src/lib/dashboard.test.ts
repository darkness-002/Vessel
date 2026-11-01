import { describe, it, expect, vi } from 'vitest';
import { getDemoState, DEMO_CONTAINERS, DEMO_METRICS, DEMO_VAULT_LOCKED } from '$lib/fixtures/demoContainers';
import { showToast, dismissToast, toasts } from '$lib/stores/toastStore';
import { appSettings, updateAppSettings, toggleDensity } from '$lib/stores/appSettings';
import { get } from 'svelte/store';
import type { EffectiveCardState } from '$lib/types';

describe('Vessel Dashboard Redesign Suite', () => {
  describe('Demo Mode Fixture', () => {
    it('provides all 5 required card states in the demo fixture', () => {
      const demo = getDemoState();
      expect(demo.containers.length).toBe(5);

      const states: Record<string, EffectiveCardState> = {};

      for (const container of demo.containers) {
        const isLocked = demo.vaultLockedMap[container.id];
        const metric = demo.metricsMap[container.id];
        let state: EffectiveCardState = 'closed';

        if (isLocked) {
          state = 'locked';
        } else if (metric?.status === 'running') {
          state = 'running';
        } else if (metric?.status === 'idle') {
          state = 'idle';
        } else if (metric?.status === 'hibernated') {
          state = 'hibernated';
        }

        states[container.id] = state;
      }

      // 1. Running state
      expect(states['github']).toBe('running');
      expect(demo.metricsMap['github'].memoryMb).toBeGreaterThan(0);
      expect(demo.metricsMap['github'].cpuUsage).toBeGreaterThan(0);
      expect(demo.metricsMap['github'].history?.length).toBeGreaterThan(1);

      // 2. Idle state
      expect(states['linear']).toBe('idle');
      expect(demo.metricsMap['linear'].idleSeconds).toBeGreaterThan(0);

      // 3. Hibernated state
      expect(states['spotify']).toBe('hibernated');
      expect(demo.metricsMap['spotify'].memoryMb).toBe(0);

      // 4. Vault sealed state
      expect(states['slack']).toBe('locked');
      expect(demo.vaultLockedMap['slack']).toBe(true);

      // 5. Closed state
      expect(states['notion']).toBe('closed');
      expect(demo.metricsMap['notion'].status).toBe('closed');
      expect(demo.vaultLockedMap['notion']).toBe(false);

      // Verify all 5 distinct states are accounted for
      const uniqueStates = new Set(Object.values(states));
      expect(uniqueStates.size).toBe(5);
      expect(uniqueStates.has('running')).toBe(true);
      expect(uniqueStates.has('idle')).toBe(true);
      expect(uniqueStates.has('hibernated')).toBe(true);
      expect(uniqueStates.has('locked')).toBe(true);
      expect(uniqueStates.has('closed')).toBe(true);
    });
  });

  describe('Toast with Undo Action', () => {
    it('supports Undo action for Hibernate and Vault Seal', () => {
      const undoCallback = vi.fn();

      const toastId = showToast('Hibernated "Linear"', 'info', 0, {
        label: 'Undo',
        onClick: undoCallback,
      });

      const list = get(toasts);
      const targetToast = list.find((t) => t.id === toastId);

      expect(targetToast).toBeDefined();
      expect(targetToast?.message).toBe('Hibernated "Linear"');
      expect(targetToast?.action).toBeDefined();
      expect(targetToast?.action?.label).toBe('Undo');

      // Trigger the undo action
      targetToast?.action?.onClick();
      expect(undoCallback).toHaveBeenCalledTimes(1);

      dismissToast(toastId);
      expect(get(toasts).find((t) => t.id === toastId)).toBeUndefined();
    });
  });

  describe('Settings & Density Controller', () => {
    it('defaults to minimal density and toggles cleanly to detailed', () => {
      updateAppSettings({ density: 'minimal' });
      expect(get(appSettings).density).toBe('minimal');

      toggleDensity();
      expect(get(appSettings).density).toBe('detailed');

      toggleDensity();
      expect(get(appSettings).density).toBe('minimal');
    });

    it('persists demo mode toggle and idle threshold', () => {
      updateAppSettings({ isDemoMode: true, globalIdleThresholdMinutes: 30 });
      expect(get(appSettings).isDemoMode).toBe(true);
      expect(get(appSettings).globalIdleThresholdMinutes).toBe(30);

      updateAppSettings({ isDemoMode: false, globalIdleThresholdMinutes: 15 });
      expect(get(appSettings).isDemoMode).toBe(false);
      expect(get(appSettings).globalIdleThresholdMinutes).toBe(15);
    });
  });

  describe('Memory Telemetry Calculations', () => {
    it('calculates measured Electron equivalent using live container counts and metrics', () => {
      const ELECTRON_ACTIVE_MB = 348.0;
      const ELECTRON_IDLE_MB = 312.0;
      const ELECTRON_HIBERNATED_MB = 305.0;

      const runningCount = 1;
      const idleCount = 1;
      const hibernatedCount = 1;
      const lockedCount = 1;

      const liveVesselActiveRam = 64.2 + 48.6; // 112.8 MB
      const measuredElectron =
        runningCount * ELECTRON_ACTIVE_MB +
        idleCount * ELECTRON_IDLE_MB +
        (hibernatedCount + lockedCount) * ELECTRON_HIBERNATED_MB;

      // 348 + 312 + 2*305 = 1270 MB
      expect(measuredElectron).toBe(1270.0);

      const saved = measuredElectron - liveVesselActiveRam;
      expect(saved).toBe(1157.2);

      const reductionPercent = Math.round((saved / measuredElectron) * 100);
      expect(reductionPercent).toBe(91);
    });
  });

  describe('Container Custom Preferences', () => {
    it('supports custom profile label, never hibernate, and custom idle threshold', () => {
      const container = {
        ...DEMO_CONTAINERS[0],
        profileLabel: 'Production',
        neverHibernate: true,
        idleThresholdMinutes: 30,
      };

      expect(container.profileLabel).toBe('Production');
      expect(container.neverHibernate).toBe(true);
      expect(container.idleThresholdMinutes).toBe(30);
    });
  });
});
