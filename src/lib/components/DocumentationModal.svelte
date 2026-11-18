<script lang="ts">
  import Icon from '$lib/components/Icon.svelte';
  import { openUrl } from '@tauri-apps/plugin-opener';

  interface Props {
    isOpen: boolean;
    onClose: () => void;
  }

  let { isOpen = false, onClose }: Props = $props();

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && isOpen) {
      onClose();
    }
  }

  async function handleOpenDocs() {
    try {
      await openUrl('https://darkness-002.github.io/Vessel/docs.html');
    } catch {
      if (typeof window !== 'undefined') {
        window.open('https://darkness-002.github.io/Vessel/docs.html', '_blank');
      }
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if isOpen}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/80 animate-in fade-in"
    onclick={(e) => {
      if (e.target === e.currentTarget) onClose();
    }}
    role="presentation"
  >
    <div
      class="bg-[var(--bg-surface)] border border-[var(--border-subtle)] rounded-2xl w-full max-w-2xl shadow-2xl overflow-hidden flex flex-col animate-in zoom-in-95 select-none"
      role="dialog"
      aria-modal="true"
      aria-labelledby="doc-modal-title"
    >
      <!-- Header -->
      <div class="px-6 py-4 border-b border-[var(--border-subtle)] flex items-center justify-between">
        <div class="flex items-center gap-2.5">
          <div
            class="w-8 h-8 rounded-lg bg-[var(--bg-canvas)] border border-[var(--border-subtle)] flex items-center justify-center text-[var(--accent-primary)]"
          >
            <Icon name="info" size={16} />
          </div>
          <div>
            <h2 id="doc-modal-title" class="text-sm font-semibold text-[var(--text-primary)]">
              Vessel Architecture & Documentation
            </h2>
            <p class="text-[11px] text-[var(--text-muted)] mt-0.5">Zero-capability containers & native performance</p>
          </div>
        </div>

        <button
          type="button"
          onclick={onClose}
          class="inline-flex items-center justify-center min-w-[36px] min-h-[36px] rounded-lg text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-surface-hover)] focus-visible:outline-none transition-colors"
          aria-label="Close documentation"
        >
          <Icon name="close" size={16} />
        </button>
      </div>

      <!-- Content -->
      <div class="p-6 space-y-6 max-h-[70vh] overflow-y-auto vessel-scroll text-xs text-[var(--text-muted)] leading-relaxed">
        <!-- Section 1 -->
        <div class="space-y-2">
          <h3 class="text-xs font-semibold text-[var(--text-primary)] uppercase tracking-wider font-mono">
            Zero-Capability Sandbox Policy
          </h3>
          <p>
            Vessel strictly enforces a Zero-Capability Container policy. Container webview windows running untrusted third-party web applications have zero access to Tauri internal IPC commands, filesystem APIs, or shell execution. All backend operations are restricted to the local management window.
          </p>
        </div>

        <!-- Section 2 -->
        <div class="space-y-2">
          <h3 class="text-xs font-semibold text-[var(--text-primary)] uppercase tracking-wider font-mono">
            Encrypted Profile Vaults (AES-256-GCM)
          </h3>
          <p>
            Each container stores its cookies, local storage, and session tokens in an isolated profile directory. When a container is closed or locked, its entire profile directory is encrypted into a sealed vault file using authenticated AES-256-GCM encryption with keys stored in the OS Keyring or derived from a Master Passphrase via Argon2id.
          </p>
        </div>

        <!-- Section 3 -->
        <div class="space-y-2">
          <h3 class="text-xs font-semibold text-[var(--text-primary)] uppercase tracking-wider font-mono">
            Adaptive Idle Hibernation
          </h3>
          <p>
            Unfocused container windows are monitored by Vessel's background process supervisor. After an inactivity threshold (default 15 minutes), the heavy webview window is gracefully hibernated back to the host operating system, reclaiming working set memory while preserving full session state for instantaneous recovery.
          </p>
        </div>

        <!-- Section 4: Keyboard Shortcuts -->
        <div class="space-y-2 pt-2 border-t border-[var(--border-subtle)]">
          <h3 class="text-xs font-semibold text-[var(--text-primary)] uppercase tracking-wider font-mono">
            Global Keyboard Shortcuts
          </h3>
          <div class="grid grid-cols-2 gap-2.5 pt-1">
            <div class="flex items-center justify-between p-2.5 rounded-lg bg-[var(--bg-canvas)] border border-[var(--border-subtle)]">
              <span class="text-[var(--text-primary)] font-medium">Quick Switcher</span>
              <kbd class="px-2 py-0.5 rounded bg-[var(--bg-surface)] border border-[var(--border-subtle)] font-mono text-[10px] text-[var(--text-muted)]">Ctrl + K</kbd>
            </div>
            <div class="flex items-center justify-between p-2.5 rounded-lg bg-[var(--bg-canvas)] border border-[var(--border-subtle)]">
              <span class="text-[var(--text-primary)] font-medium">Add New App</span>
              <kbd class="px-2 py-0.5 rounded bg-[var(--bg-surface)] border border-[var(--border-subtle)] font-mono text-[10px] text-[var(--text-muted)]">Ctrl + N</kbd>
            </div>
            <div class="flex items-center justify-between p-2.5 rounded-lg bg-[var(--bg-canvas)] border border-[var(--border-subtle)]">
              <span class="text-[var(--text-primary)] font-medium">Focus Search Filter</span>
              <kbd class="px-2 py-0.5 rounded bg-[var(--bg-surface)] border border-[var(--border-subtle)] font-mono text-[10px] text-[var(--text-muted)]">/</kbd>
            </div>
            <div class="flex items-center justify-between p-2.5 rounded-lg bg-[var(--bg-canvas)] border border-[var(--border-subtle)]">
              <span class="text-[var(--text-primary)] font-medium">Lock All Vaults</span>
              <kbd class="px-2 py-0.5 rounded bg-[var(--bg-surface)] border border-[var(--border-subtle)] font-mono text-[10px] text-[var(--text-muted)]">Ctrl + Shift + L</kbd>
            </div>
          </div>
        </div>
      </div>

      <!-- Footer -->
      <div class="px-6 py-4 bg-[var(--bg-canvas)]/40 border-t border-[var(--border-subtle)] flex items-center justify-between">
        <button
          type="button"
          onclick={handleOpenDocs}
          class="min-h-[38px] px-4 rounded-xl border border-[var(--border-subtle)] bg-[var(--bg-canvas)] hover:bg-[var(--bg-surface-hover)] text-[var(--text-primary)] text-xs font-medium flex items-center gap-1.5 transition-colors cursor-pointer"
        >
          <span>Online Docs & Benchmarks</span>
          <span class="text-[11px] leading-none">↗</span>
        </button>
        <button
          type="button"
          onclick={onClose}
          class="min-h-[38px] px-5 rounded-xl bg-[var(--accent-primary)] hover:opacity-95 text-[var(--accent-fg)] text-xs font-semibold active:scale-[0.98] transition-colors shadow-sm cursor-pointer"
        >
          Close
        </button>
      </div>
    </div>
  </div>
{/if}
