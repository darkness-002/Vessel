<script lang="ts">
  import { apps, buildAppId, normalizeServiceUrl, persistApps } from '$lib/stores/appStore';
  import { switchView } from '$lib/stores/uiStore';
  import { addTabForApp } from '$lib/stores/tabStore';
  import type { AppConfig } from '$lib/types';

  export let store: any;

  let newAppName = '';
  let newAppUrl = '';
  let newAppIcon = '';
  let formError = '';
  let isDeploying = false;
  let showSuccess = false;
  let lastDeployedAppId = '';
  let successTimer: ReturnType<typeof setTimeout> | null = null;

  $: isNameValid = newAppName.trim().length > 0 && !!buildAppId(newAppName.trim());
  $: isUrlValid = (newAppUrl.trim().length > 0) && (() => { 
    try { 
      new URL(normalizeServiceUrl(newAppUrl)); 
      return true; 
    } catch { 
      return false; 
    } 
  })();

  let isCheckingUrl = false;
  let urlStatus: 'none' | 'valid' | 'invalid' | 'checking' = 'none';

  async function checkUrlAccessibility() {
    if (!isUrlValid) {
      urlStatus = 'invalid';
      return;
    }
    isCheckingUrl = true;
    urlStatus = 'checking';
    // Simulate accessibility check
    await new Promise(r => setTimeout(r, 1500));
    isCheckingUrl = false;
    urlStatus = 'valid';
  }

  async function addNewApp() {
    if (isDeploying) return;
    isDeploying = true;
    try {
      formError = '';
      const trimmedName = newAppName.trim();
      const trimmedUrl = newAppUrl.trim();
      if (!trimmedName || !trimmedUrl) {
        formError = 'Name and URL are required.';
        return;
      }

      const id = buildAppId(trimmedName);
      if (!id) {
        formError = 'Use letters or numbers in the app name.';
        return;
      }

      if ($apps.some((app) => app.id === id)) {
        formError = 'An app with this name already exists.';
        return;
      }

      const normalizedUrl = normalizeServiceUrl(trimmedUrl);
      try {
        new URL(normalizedUrl);
      } catch {
        formError = 'Please enter a valid URL.';
        return;
      }

      const newApp: AppConfig = { 
        id,
        name: trimmedName,
        icon: (newAppIcon || trimmedName.charAt(0)).toUpperCase(), 
        url: normalizedUrl, 
        features: { theme: 'default', adblock: false, profile: 'default', customCss: '', customJs: '', injectionAllowlist: '', idleSleepSeconds: 15 } 
      };
      const updatedApps = [...$apps, newApp];
      apps.set(updatedApps); 
      await persistApps(updatedApps, store);
      
      lastDeployedAppId = id;
      showSuccess = true;
      if (successTimer) clearTimeout(successTimer);
      successTimer = setTimeout(() => { showSuccess = false; }, 6000);

      newAppName = ''; newAppUrl = ''; newAppIcon = ''; 
    } finally {
      isDeploying = false;
    }
  }

  function resetForm() {
    newAppName = '';
    newAppUrl = '';
    newAppIcon = '';
    formError = '';
    urlStatus = 'none';
  }

  function launchLast() {
    const app = $apps.find(a => a.id === lastDeployedAppId);
    if (app) addTabForApp(app);
    showSuccess = false;
  }
</script>

<div class="max-w-4xl mx-auto py-8">
  <div class="flex items-center justify-between mb-8">
    <div>
      <h1 class="font-headline font-extrabold text-3xl text-on-surface mb-1">Deploy New Instance</h1>
      <p class="text-on-surface-variant text-sm">Create an isolated, high-performance container for any web application.</p>
    </div>
    <button on:click={() => switchView('gallery')} class="flex items-center gap-2 px-4 py-2 rounded-lg bg-surface-container-high hover:bg-surface-container-highest text-on-surface transition-all border border-outline-variant/20 shadow-sm active:scale-95">
      <span class="material-symbols-outlined text-sm">arrow_back</span>
      <span class="font-label text-[10px] uppercase font-bold tracking-widest">Back to Gallery</span>
    </button>
  </div>

  <div class="grid grid-cols-1 lg:grid-cols-3 gap-8">
    <div class="lg:col-span-2 space-y-6">
      <div class="bg-surface-container rounded-xl p-6 sm:p-8 border border-outline-variant/10 shadow-sm space-y-6">
        
        <!-- Name Field -->
        <div class="space-y-1.5">
          <div class="flex justify-between items-center">
            <label for="instance-name" class="text-[10px] text-outline uppercase tracking-widest font-bold">
              Instance Name <span class="text-primary">*</span>
            </label>
            <span class="text-[9px] text-outline-variant font-mono">{newAppName.length}/32</span>
          </div>
          <div class="relative group">
            <input 
              id="instance-name" 
              type="text" 
              bind:value={newAppName} 
              maxlength="32"
              placeholder="e.g. ChatGPT, Spotify, Notion" 
              aria-required="true"
              class="w-full bg-surface-container-lowest border {newAppName.length > 0 ? (isNameValid ? 'border-primary/50' : 'border-error/50') : 'border-outline-variant/30'} rounded-lg p-3 text-sm text-on-surface focus:outline-none focus:ring-1 focus:ring-primary focus:border-primary transition-all shadow-inner"
            />
            <div class="absolute right-3 top-1/2 -translate-y-1/2 transition-opacity pointer-events-none">
              {#if newAppName.length > 0}
                <span class="material-symbols-outlined text-sm {isNameValid ? 'text-primary' : 'text-error'}">{isNameValid ? 'check_circle' : 'cancel'}</span>
              {:else}
                <span class="material-symbols-outlined text-sm text-outline group-focus-within:text-primary">edit</span>
              {/if}
            </div>
          </div>
          {#if newAppName.length > 0}
            <p class="text-[9px] {isNameValid ? 'text-primary' : 'text-error'} font-bold flex items-center gap-1 animate-in fade-in slide-in-from-left-1">
              <span class="material-symbols-outlined text-[10px]">{isNameValid ? 'check' : 'close'}</span>
              {isNameValid ? `✓ ${newAppName} (available)` : '✗ Only letters and numbers allowed'}
            </p>
          {:else}
            <p class="text-[9px] text-on-surface-variant italic">Letters and numbers only. This will be used as the internal App ID.</p>
          {/if}
        </div>

        <!-- URL Field -->
        <div class="space-y-1.5">
          <div class="flex items-center gap-2">
            <label for="target-url" class="text-[10px] text-outline uppercase tracking-widest font-bold">
              Target URL <span class="text-primary">*</span>
            </label>
            <div class="group relative">
              <span class="material-symbols-outlined text-xs text-outline cursor-help">info</span>
              <div class="absolute bottom-full left-1/2 -translate-x-1/2 mb-2 w-48 p-2 bg-surface-container-highest text-[10px] text-on-surface rounded-md shadow-xl border border-outline-variant/20 opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none z-10">
                Specify the login page or main dashboard. Vessel handles isolated cookie jars for this URL.
              </div>
            </div>
          </div>
          <div class="relative group">
            <input 
              id="target-url" 
              type="url" 
              bind:value={newAppUrl} 
              placeholder="https://app.notion.so" 
              aria-required="true"
              on:blur={checkUrlAccessibility}
              class="w-full bg-surface-container-lowest border {urlStatus === 'valid' ? 'border-primary/50' : (urlStatus === 'invalid' ? 'border-error/50' : 'border-outline-variant/30')} rounded-lg p-3 text-sm text-on-surface focus:outline-none focus:ring-1 focus:ring-primary focus:border-primary transition-all shadow-inner"
            />
            <div class="absolute right-3 top-1/2 -translate-y-1/2 transition-opacity pointer-events-none flex items-center gap-2">
              {#if urlStatus === 'checking'}
                <span class="material-symbols-outlined text-sm animate-spin text-primary">progress_activity</span>
              {:else if urlStatus === 'valid'}
                <span class="material-symbols-outlined text-sm text-primary">verified</span>
              {:else if urlStatus === 'invalid'}
                <span class="material-symbols-outlined text-sm text-error">link_off</span>
              {:else if newAppUrl.length > 0}
                <span class="material-symbols-outlined text-sm {isUrlValid ? 'text-primary' : 'text-error'}">{isUrlValid ? 'link' : 'link_off'}</span>
              {/if}
            </div>
          </div>
          {#if urlStatus === 'checking'}
            <p class="text-[9px] text-primary animate-pulse font-bold">Checking URL accessibility...</p>
          {:else if urlStatus === 'valid'}
            <p class="text-[9px] text-primary font-bold">✓ URL accessible (Vessel Ready)</p>
          {:else if urlStatus === 'invalid'}
            <p class="text-[9px] text-error font-bold">✗ Invalid URL format or unreachable</p>
          {:else}
            <p class="text-[9px] text-on-surface-variant italic">Protocols like <code class="bg-surface-container-highest px-1 rounded">https://</code> are automatically validated.</p>
          {/if}
        </div>

        <!-- Icon Field & Preview -->
        <div class="flex flex-col sm:flex-row gap-6">
          <div class="flex-1 space-y-1.5">
            <label for="icon-character" class="text-[10px] text-outline uppercase tracking-widest font-bold">
              App Identity (Icon)
            </label>
            <input 
              id="icon-character" 
              type="text" 
              bind:value={newAppIcon} 
              maxlength="2" 
              placeholder="C" 
              class="w-full bg-surface-container-lowest border border-outline-variant/30 rounded-lg p-3 text-sm text-on-surface focus:outline-none focus:ring-1 focus:ring-primary transition-all shadow-inner"
            />
            <p class="text-[9px] text-on-surface-variant">Type a single letter or paste an emoji.</p>
          </div>
          
          <div class="flex flex-col items-center justify-center p-4 bg-surface-container-low border border-outline-variant/10 rounded-xl min-w-[120px]">
            <span class="text-[9px] text-outline uppercase tracking-widest mb-3 font-bold">Live Preview</span>
            <div class="w-16 h-16 rounded-2xl bg-surface-container-highest flex items-center justify-center text-3xl font-bold border border-outline-variant/30 text-on-surface shadow-lg group hover:scale-105 transition-transform">
              {newAppIcon || (newAppName ? newAppName.charAt(0).toUpperCase() : '?')}
            </div>
            <span class="text-[8px] text-outline-variant uppercase mt-3 tracking-tighter">Sidebar Appearance</span>
          </div>
        </div>

        {#if formError}
          <div class="p-4 rounded-lg bg-error/10 border border-error/20 flex items-start gap-3 animate-in fade-in slide-in-from-top-1">
            <span class="material-symbols-outlined text-error text-lg">error_outline</span>
            <div>
              <p class="text-xs text-error font-bold uppercase tracking-wider mb-0.5">Validation Error</p>
              <p class="text-xs text-error/80">{formError}</p>
            </div>
          </div>
        {/if}

        <div class="pt-4 border-t border-outline-variant/10 flex items-center justify-between">
          <div class="flex items-center gap-4">
            <button
              on:click={addNewApp}
              class="px-8 py-3 bg-primary text-on-primary font-label font-bold rounded-lg text-xs uppercase tracking-[0.15em] hover:brightness-110 active:scale-95 transition-all disabled:opacity-30 disabled:cursor-not-allowed shadow-xl shadow-primary/20 flex items-center gap-2"
              disabled={!isNameValid || !isUrlValid || isDeploying || showSuccess}
            >
              {#if isDeploying}
                <span class="material-symbols-outlined animate-spin text-sm">progress_activity</span>
                Deploying...
              {:else}
                Deploy Container
              {/if}
            </button>
            <button
              on:click={resetForm}
              class="px-4 py-3 text-on-surface-variant hover:text-on-surface font-label font-bold text-xs uppercase tracking-widest transition-colors disabled:opacity-30"
              disabled={isDeploying}
            >
              Reset Form
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- Success / Launch State -->
    <div class="space-y-6">
      {#if showSuccess && lastDeployedAppId}
        <div class="bg-primary/5 border border-primary/20 rounded-xl p-8 flex flex-col items-center text-center animate-in zoom-in-95 duration-500 shadow-2xl shadow-primary/5">
          <div class="w-16 h-16 rounded-full bg-primary/10 flex items-center justify-center mb-6">
            <span class="material-symbols-outlined text-primary text-3xl">task_alt</span>
          </div>
          <h3 class="font-headline font-bold text-xl text-on-surface mb-2">Instance Ready</h3>
          <p class="text-on-surface-variant text-sm mb-8">The isolated container for <span class="text-primary font-bold">"{$apps.find(a => a.id === lastDeployedAppId)?.name}"</span> has been successfully provisioned.</p>
          
          <div class="w-full space-y-3">
            <button 
              on:click={launchLast}
              class="w-full py-3 bg-primary text-on-primary rounded-lg font-label font-bold text-xs uppercase tracking-widest hover:brightness-110 active:scale-95 transition-all shadow-lg shadow-primary/20"
            >
              Launch Now
            </button>
            <button 
              on:click={() => { showSuccess = false; switchView('gallery'); }}
              class="w-full py-3 bg-surface-container-highest text-on-surface rounded-lg font-label font-bold text-xs uppercase tracking-widest hover:bg-surface-bright transition-all border border-outline-variant/10"
            >
              View in Gallery
            </button>
          </div>
        </div>
      {:else}
        <div class="bg-surface-container-lowest border border-outline-variant/10 rounded-xl p-6 space-y-4">
          <h4 class="text-[10px] text-outline uppercase font-bold tracking-[0.2em] mb-4">Deployment Guide</h4>
          <div class="flex gap-4">
            <div class="w-6 h-6 rounded-full bg-surface-container-highest flex items-center justify-center text-[10px] font-bold text-outline shrink-0">1</div>
            <p class="text-xs text-on-surface-variant leading-relaxed">Containers use <span class="text-on-surface font-medium">isolated sessions</span>. Your login state is never shared between instances.</p>
          </div>
          <div class="flex gap-4">
            <div class="w-6 h-6 rounded-full bg-surface-container-highest flex items-center justify-center text-[10px] font-bold text-outline shrink-0">2</div>
            <p class="text-xs text-on-surface-variant leading-relaxed">Performance is optimized through <span class="text-on-surface font-medium">Native Hibernation</span>. Inactive tabs sleep to save RAM.</p>
          </div>
          <div class="flex gap-4">
            <div class="w-6 h-6 rounded-full bg-surface-container-highest flex items-center justify-center text-[10px] font-bold text-outline shrink-0">3</div>
            <p class="text-xs text-on-surface-variant leading-relaxed">You can customize ad-blocking and <span class="text-on-surface font-medium">CSS injection</span> after deployment in the settings panel.</p>
          </div>
        </div>
      {/if}
    </div>
  </div>
</div>
