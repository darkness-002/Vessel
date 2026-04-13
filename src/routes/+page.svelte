
<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event"; 
  import { load } from "@tauri-apps/plugin-store";
  import { onMount } from "svelte";

  let activeId = '';
  let apps: any[] = [];
  let store: any;
  
  // UI State
  let showAddModal = false;
  let showSettingsModal = false;
  let editingApp: any = null;
  
  let notifications: any[] = [];
  let showNotifications = false;
  let unreadCount = 0;

  // Form State (Add App)
  let newAppName = ''; let newAppUrl = 'https://'; let newAppIcon = '';
  
  // THE THEME STORE
  const themes: Record<string, string> = {
    default: "",
    dark_invert: "body { filter: invert(1) hue-rotate(180deg); background: black !important; }",
    midnight: "body { background-color: #0f172a !important; color: #e2e8f0 !important; } * { border-color: #334155 !important; }",
    oled: "body { background-color: #000000 !important; color: #ffffff !important; }"
  };

  onMount(async () => {
    store = await load('vessel_settings.json', { autoSave: true, defaults: {} });
    const savedApps = await store.get('apps');
    apps = savedApps || [{ id: 'linear', icon: 'L', url: 'https://linear.app', features: { theme: 'default', adblock: false, accountProfile: 'default' } }];
    if (!savedApps) await store.set('apps', apps);

    const savedActiveId = await store.get('lastActiveApp');
    if (savedActiveId) {
      const appToLaunch = apps.find(a => a.id === savedActiveId);
      if (appToLaunch) {
        launch(appToLaunch);
      }
    }

    listen("vessel-notification", (event: any) => {
      notifications = [event.payload, ...notifications];
      if (!showNotifications) unreadCount++;
    });
  });

  async function launch(app: any) {
    activeId = app.id;
    if (store) {
      await store.set('lastActiveApp', activeId);
    }
    
    let jsToInject = "";
    if (app.features?.adblock && app.url.includes("youtube.com")) {
      jsToInject = `
        setInterval(() => {
          const skipBtn = document.querySelector('.ytp-skip-ad-button, .ytp-ad-skip-button');
          if (skipBtn) skipBtn.click();
          const banners = document.querySelectorAll('.ytp-ad-overlay-container, #player-ads');
          banners.forEach(b => b.style.display = 'none');
        }, 500);
      `;
    }
    
    const cssToInject = themes[app.features?.theme || 'default'];
    const profileToUse = app.features?.accountProfile || 'default';
    await invoke("open_app", { id: app.id, url: app.url, css: cssToInject, js: jsToInject, profile: profileToUse });
  }

  function toggleNotifications() { 
    showNotifications = !showNotifications; 
    if (showNotifications) unreadCount = 0; 
  }

  async function addNewApp() {
    if (!newAppName || !newAppUrl) return;
    const newApp = { 
      id: newAppName.toLowerCase().replace(/\s+/g, '-'), 
      icon: newAppIcon || newAppName.charAt(0).toUpperCase(), 
      url: newAppUrl, 
      features: { theme: 'default', adblock: false, accountProfile: 'default' } 
    };
    apps = [...apps, newApp]; 
    await store.set('apps', apps);
    
    newAppName = ''; newAppUrl = 'https://'; newAppIcon = ''; 
    showAddModal = false;
  }

  // SETTINGS LOGIC
  function openSettings(app: any) {
    editingApp = JSON.parse(JSON.stringify(app));
    if (!editingApp.features) editingApp.features = {};
    if (!editingApp.features.accountProfile) editingApp.features.accountProfile = 'default';
    showSettingsModal = true;
  }

  async function saveSettings() {
    const index = apps.findIndex(a => a.id === editingApp.id);
    if (index !== -1) {
      apps[index] = editingApp;
      await store.set('apps', apps);
      
      // Re-inject if currently open
      if (activeId === editingApp.id) {
        launch(editingApp);
      }
    }
    showSettingsModal = false;
  }
</script>

<div class="layout">
  <nav class="sidebar">
    <div data-tauri-drag-region class="drag-bar"></div>
    <div class="app-list">
      {#each apps as app}
        <button 
          type="button"
          class:active={activeId === app.id} 
          on:click={() => launch(app)} 
          on:contextmenu|preventDefault={() => openSettings(app)}
          title="Right-click for settings"
        >
          {app.icon}
        </button>
      {/each}
    </div>
    <div class="spacer"></div>
    
    {#if activeId}
      <button type="button" class="settings-btn" title="Active App Settings" on:click={() => {
        const activeApp = apps.find(a => a.id === activeId);
        if (activeApp) openSettings(activeApp);
      }}>
        ⚙️
      </button>
    {/if}

    <button type="button" class="bell-btn" class:active={showNotifications} on:click={toggleNotifications} title="Notifications">
      🔔{#if unreadCount > 0}<span class="badge">{unreadCount}</span>{/if}
    </button>
    <button type="button" class="add-btn" on:click={() => showAddModal = true} title="Add New App">+</button>
  </nav>

  {#if showNotifications}
    <aside class="notification-drawer">
      <h2>Global Feed</h2>
      {#if notifications.length === 0}
        <p class="empty-state">No new notifications.</p>
      {:else}
        {#each notifications as note}
          <div class="notif-card">
            <div class="notif-header">
              <span class="app-badge">{note.appId.toUpperCase()}</span>
              <span class="time">{note.time}</span>
            </div>
            <h4>{note.title}</h4>
            <p>{note.body}</p>
          </div>
        {/each}
      {/if}
    </aside>
  {/if}

  <main class="welcome-screen">
    <div class="logo">V</div>
    <h1>Welcome to Vessel</h1>
    <p>Left-click to open. Right-click an app to edit settings.</p>
  </main>

  {#if showAddModal}
    <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
    <div class="modal-backdrop" on:click|self={() => showAddModal = false}>
      <div class="modal">
        <h2>Add New Vessel</h2>
        <div class="input-group">
          <input type="text" placeholder="App Name (e.g., YouTube)" bind:value={newAppName} />
          <input type="url" placeholder="https://" bind:value={newAppUrl} />
          <input type="text" maxlength="1" placeholder="Icon (e.g., Y)" bind:value={newAppIcon} />
        </div>
        <div class="modal-actions">
          <button type="button" class="cancel-btn" on:click={() => showAddModal = false}>Cancel</button>
          <button type="button" class="save-btn" on:click={addNewApp}>Add App</button>
        </div>
      </div>
    </div>
  {/if}

  {#if showSettingsModal && editingApp}
    <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
    <div class="modal-backdrop" on:click|self={() => showSettingsModal = false}>
      <div class="modal">
        <h2>{editingApp.id.toUpperCase()} Settings</h2>
        
        <div class="settings-section">
          <label for="theme-select">Visual Theme</label>
          <select id="theme-select" bind:value={editingApp.features.theme}>
            <option value="default">Default Theme</option>
            <option value="dark_invert">Force Dark (Invert)</option>
            <option value="midnight">Midnight Blue</option>
            <option value="oled">True OLED Black</option>
          </select>
        </div>

        {#if editingApp.url.includes("youtube.com")}
          <div class="settings-section specific-feature">
            <div class="toggle-row">
              <label for="adblock-toggle">YouTube Ad-Assassin</label>
              <input id="adblock-toggle" type="checkbox" bind:checked={editingApp.features.adblock} />
            </div>
            <p class="hint">Automatically clicks 'Skip Ad' and hides banners.</p>
          </div>
        {/if}

        <div class="settings-section account-section">
          <label for="profile-select">Account Management</label>
          <select id="profile-select" bind:value={editingApp.features.accountProfile}>
            <option value="default">Default Profile</option>
            <option value="work">Work Profile</option>
            <option value="personal">Personal Profile</option>
            <option value="alt1">Alternative 1</option>
            <option value="alt2">Alternative 2</option>
          </select>
          <p class="hint">Isolates cookies so you can log into a different account.</p>
        </div>

        <div class="modal-actions">
          <button type="button" class="cancel-btn" on:click={() => showSettingsModal = false}>Cancel</button>
          <button type="button" class="save-btn" on:click={saveSettings}>Apply Settings</button>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  :global(body) { margin: 0; background: #0f0f11; overflow: hidden; font-family: sans-serif; }
  .layout { display: flex; width: 100vw; height: 100vh; }
  
  .sidebar { display: flex; flex-direction: column; width: 70px; height: 100vh; background: #18181b; border-right: 1px solid #27272a; align-items: center; z-index: 20; }
  .drag-bar { width: 100%; height: 30px; cursor: grab; background: rgba(255,255,255,0.02); }
  .app-list { display: flex; flex-direction: column; gap: 12px; padding-top: 15px;}
  .spacer { flex-grow: 1; }
  button { width: 44px; height: 44px; border-radius: 12px; background: #27272a; color: #a1a1aa; border: none; cursor: pointer; font-weight: bold; font-size: 16px; position: relative; transition: all 0.2s; }
  button:hover { background: #3f3f46; color: white; transform: translateY(-2px); }
  button.active { background: #2563eb; color: white; box-shadow: 0 0 15px rgba(37,99,235,0.3); }
  .add-btn { margin-bottom: 20px; background: transparent; border: 1px dashed #52525b; }
  
  .welcome-screen { flex-grow: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; color: white; z-index: 1;}
  .logo { width: 60px; height: 60px; background: #2563eb; border-radius: 16px; display: flex; align-items: center; justify-content: center; font-size: 32px; font-weight: bold; margin-bottom: 20px; }

  /* Notifications */
  .bell-btn { font-size: 18px; margin-bottom: 10px; }
  .badge { position: absolute; top: -5px; right: -5px; background: #ef4444; color: white; font-size: 10px; width: 18px; height: 18px; border-radius: 50%; display: flex; align-items: center; justify-content: center; font-weight: bold;}
  .notification-drawer { width: 300px; height: 100vh; background: #18181b; border-right: 1px solid #27272a; padding: 20px; box-sizing: border-box; overflow-y: auto; z-index: 15; animation: slideIn 0.2s ease-out; }
  @keyframes slideIn { from { margin-left: -300px; } to { margin-left: 0; } }
  .notification-drawer h2 { color: white; margin-top: 0; font-size: 18px; padding-bottom: 15px; border-bottom: 1px solid #27272a;}
  .empty-state { color: #a1a1aa; font-size: 14px; text-align: center; margin-top: 50px; }
  .notif-card { background: #0f0f11; padding: 15px; border-radius: 10px; margin-bottom: 15px; border: 1px solid #27272a; }
  .notif-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px;}
  .app-badge { font-size: 10px; background: #3f3f46; color: white; padding: 2px 6px; border-radius: 4px; font-weight: bold;}
  .time { font-size: 10px; color: #a1a1aa; }
  .notif-card h4 { color: white; margin: 0 0 5px 0; font-size: 14px; }
  .notif-card p { color: #a1a1aa; margin: 0; font-size: 12px; line-height: 1.4; }

  /* Modals */
  .modal-backdrop { position: fixed; top: 0; left: 0; width: 100vw; height: 100vh; background: rgba(0, 0, 0, 0.7); backdrop-filter: blur(4px); display: flex; align-items: center; justify-content: center; z-index: 100; }
  .modal { background: #18181b; padding: 25px; border-radius: 16px; width: 380px; border: 1px solid #27272a; box-shadow: 0 20px 40px rgba(0,0,0,0.5); display: flex; flex-direction: column; gap: 20px; color: white; }
  .modal h2 { margin: 0; font-size: 20px; text-transform: capitalize; }
  .input-group { display: flex; flex-direction: column; gap: 10px; }
  .modal input[type="text"], .modal input[type="url"], .modal select { background: #0f0f11; border: 1px solid #27272a; padding: 12px; border-radius: 8px; color: white; font-size: 14px; outline: none; width: 100%; box-sizing: border-box; }
  .modal input:focus, .modal select:focus { border-color: #2563eb; }
  
  /* Settings Panel */
  .settings-section { background: #0f0f11; padding: 15px; border-radius: 8px; border: 1px solid #27272a; }
  .settings-section label { font-size: 12px; color: #a1a1aa; text-transform: uppercase; letter-spacing: 0.5px; display: block; margin-bottom: 10px; }
  .specific-feature { border-left: 3px solid #ef4444; }
  .account-section { border-left: 3px solid #10b981; }
  .toggle-row { display: flex; justify-content: space-between; align-items: center; }
  .toggle-row label { margin: 0; color: white; font-weight: bold; text-transform: none; font-size: 14px;}
  .toggle-row input[type="checkbox"] { width: 18px; height: 18px; cursor: pointer; }
  .hint { margin: 8px 0 0 0; font-size: 12px; color: #71717a; line-height: 1.4; }
  
  .modal-actions { display: flex; justify-content: flex-end; gap: 10px; }
  .modal-actions button { width: auto; padding: 0 16px; font-size: 14px; height: 40px; border-radius: 8px; font-weight: bold; cursor: pointer; }
  .cancel-btn { background: transparent; border: 1px solid #52525b; color: white; }
  .save-btn { background: #2563eb; color: white; border: none; }
</style>
```