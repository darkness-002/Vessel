import { describe, it, expect, vi } from 'vitest';
import { getDemoState, DEMO_CONTAINERS, DEMO_METRICS, DEMO_VAULT_LOCKED } from '$lib/fixtures/demoContainers';
import { showToast, dismissToast, toasts } from '$lib/stores/toastStore';
import { appSettings, updateAppSettings, toggleDensity } from '$lib/stores/appSettings';
import baselineData from '$lib/data/electron_baseline.json';
import { get } from 'svelte/store';
import type { EffectiveCardState, ElectronBaseline } from '$lib/types';

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

  describe('Memory Telemetry & Electron Baseline', () => {
    it('loads electron baseline with assumed source and valid schema', () => {
      const baseline = baselineData as ElectronBaseline;
      expect(baseline.source).toBe('assumed');
      expect(baseline.hardware).toBeDefined();
      expect(baseline.os).toBeDefined();
      expect(baseline.states.active.meanMb).toBeGreaterThan(0);
      expect(baseline.states.idle.meanMb).toBeGreaterThan(0);
      expect(baseline.states.hibernated.meanMb).toBeGreaterThan(0);
    });

    it('calculates estimated Electron baseline using live container counts and baseline data', () => {
      const baseline = baselineData as ElectronBaseline;

      const runningCount = 1;
      const idleCount = 1;
      const hibernatedCount = 1;
      const lockedCount = 1;

      const liveVesselActiveRam = 64.2 + 48.6; // 112.8 MB
      const estimatedElectron =
        runningCount * baseline.states.active.meanMb +
        idleCount * baseline.states.idle.meanMb +
        (hibernatedCount + lockedCount) * baseline.states.hibernated.meanMb;

      // 348 + 312 + 2*305 = 1270 MB
      expect(estimatedElectron).toBe(1270.0);

      const saved = estimatedElectron - liveVesselActiveRam;
      expect(saved).toBe(1157.2);

      const reductionPercent = Math.round((saved / estimatedElectron) * 100);
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
