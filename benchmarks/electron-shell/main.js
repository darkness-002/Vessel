const { app, BrowserWindow } = require('electron');

// Parse target URL and options from command line flags
function getTargetUrl() {
  for (const arg of process.argv) {
    if (arg.startsWith('--url=')) {
      return arg.slice(6);
    }
  }
  return 'https://example.com';
}

function parseDuration() {
  for (const arg of process.argv) {
    if (arg.startsWith('--duration=')) {
      const parsed = parseInt(arg.slice(11), 10);
      if (!isNaN(parsed) && parsed > 0) return parsed;
    }
  }
  return 0; // 0 means wait for process termination / SIGTERM
}

app.whenReady().then(() => {
  const targetUrl = getTargetUrl();
  const durationSec = parseDuration();

  const win = new BrowserWindow({
    width: 1200,
    height: 800,
    show: false,
    webPreferences: {
      sandbox: true,
      contextIsolation: true,
    },
  });

  const startTime = Date.now();

  win.loadURL(targetUrl);

  win.once('ready-to-show', () => {
    win.show();
    const readyLatency = Date.now() - startTime;
    console.log(`[ELECTRON_BENCHMARK_READY] ${readyLatency}ms`);
  });

  if (durationSec > 0) {
    setTimeout(() => {
      app.quit();
    }, durationSec * 1000);
  }
});

app.on('window-all-closed', () => {
  app.quit();
});
