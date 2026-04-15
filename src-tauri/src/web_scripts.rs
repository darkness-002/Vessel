pub fn hibernate_script() -> &'static str {
    r#"
    if (!window.__vesselHibernate) {
      window.__vesselHibernate = true;
      document.querySelectorAll('video,audio').forEach((m) => {
        try {
          if (!m.paused) {
            m.dataset.vesselWasPlaying = '1';
            m.pause();
          }
        } catch (_) {}
      });
      document.documentElement.style.setProperty('caret-color', 'transparent');
    }
    "#
}

pub fn wake_script() -> &'static str {
    r#"
    if (window.__vesselHibernate) {
      window.__vesselHibernate = false;
      document.querySelectorAll('video,audio').forEach((m) => {
        try {
          if (m.dataset.vesselWasPlaying === '1') {
            m.play().catch(() => {});
            delete m.dataset.vesselWasPlaying;
          }
        } catch (_) {}
      });
      document.documentElement.style.removeProperty('caret-color');
    }
    "#
}

pub fn notification_hijack_script(app_id: &str) -> String {
    let app_id_json = serde_json::to_string(app_id).unwrap_or_else(|_| "\"\"".to_string());
    format!(
        r#"
        (function() {{
            if (window.__vesselNotificationHijackInstalled) return;
            window.__vesselNotificationHijackInstalled = true;

            const __appId = {app_id_json};
            const __invoke = (payload) => {{
                try {{
                    if (window.__TAURI__ && window.__TAURI__.core && typeof window.__TAURI__.core.invoke === 'function') {{
                        return window.__TAURI__.core.invoke('forward_notification', payload);
                    }}
                    if (window.__TAURI_INTERNALS__ && typeof window.__TAURI_INTERNALS__.invoke === 'function') {{
                        return window.__TAURI_INTERNALS__.invoke('forward_notification', payload);
                    }}
                }} catch (_err) {{}}
                return Promise.resolve(null);
            }};

            const OriginalNotification = window.Notification;
            if (typeof OriginalNotification !== 'function') {{
                window.Notification = class {{
                    constructor(title, options) {{
                        const safeTitle = title == null ? '' : String(title);
                        const safeBody = options && options.body != null ? String(options.body) : '';
                        __invoke({{ appId: __appId, title: safeTitle, body: safeBody }});
                    }}
                    static requestPermission() {{ return Promise.resolve('granted'); }}
                    static get permission() {{ return 'granted'; }}
                }};
                return;
            }}

            const ForwardingNotification = class extends OriginalNotification {{
                constructor(title, options) {{
                    const safeTitle = title == null ? '' : String(title);
                    const safeBody = options && options.body != null ? String(options.body) : '';
                    __invoke({{ appId: __appId, title: safeTitle, body: safeBody }});
                    super(title, options);
                }}
                static requestPermission(cb) {{
                    const p = OriginalNotification.requestPermission
                        ? OriginalNotification.requestPermission.call(OriginalNotification, cb)
                        : Promise.resolve('granted');
                    return Promise.resolve(p).catch(() => 'granted');
                }}
                static get permission() {{
                    return (OriginalNotification && OriginalNotification.permission) || 'granted';
                }}
            }};

            try {{
                Object.defineProperty(window, 'Notification', {{
                    configurable: true,
                    writable: true,
                    value: ForwardingNotification,
                }});
            }} catch (_err) {{
                window.Notification = ForwardingNotification;
            }}
        }})();
        "#
    )
}

pub fn stealth_injection_script() -> &'static str {
    r#"
    (function() {
        if (window.__vesselStealthInstalled) return;
        window.__vesselStealthInstalled = true;
        try {
            Object.defineProperty(navigator, 'webdriver', {
                get: () => undefined,
                set: () => {},
                configurable: true
            });
        } catch (_) {}

        if (!window.chrome) {
            window.chrome = {};
        }
        if (!window.chrome.runtime) {
            window.chrome.runtime = {};
        }

        try {
            Object.defineProperty(navigator, 'permissions', {
                get: () => ({ query: () => Promise.resolve({ state: 'granted' }) }),
                configurable: true
            });
        } catch (_) {}

        try { delete navigator.__proto__.webdriver; } catch (_) {}
        try {
            if (window.__TAURI_INTERNALS__) {
                window.__VESSEL_EMBEDDED = true;
            }
        } catch (_) {}
    })();
    "#
}

pub fn new_tab_bridge_script(app_id: &str) -> String {
    let app_id_json = serde_json::to_string(app_id).unwrap_or_else(|_| "\"\"".to_string());
    r#"
        (function() {
            if (window.__vesselTabBridgeInstalled) return;
            window.__vesselTabBridgeInstalled = true;

            const __appId = __APP_ID_JSON__;

            const __absoluteUrl = (candidate) => {
                try {
                    if (!candidate) return null;
                    return new URL(String(candidate), window.location.href).href;
                } catch (_err) {
                    return null;
                }
            };

            const __invoke = (command, payload) => {
                try {
                    if (window.__TAURI__ && window.__TAURI__.core && typeof window.__TAURI__.core.invoke === 'function') {
                        return window.__TAURI__.core.invoke(command, payload);
                    }
                    if (window.__TAURI_INTERNALS__ && typeof window.__TAURI_INTERNALS__.invoke === 'function') {
                        return window.__TAURI_INTERNALS__.invoke(command, payload);
                    }
                } catch (_err) {}
                return Promise.resolve(null);
            };

            const __vesselOpenTab = (targetUrl, targetTitle) => {
                const resolved = __absoluteUrl(targetUrl);
                if (!resolved) return;
                __invoke('request_new_tab', {
                    appId: __appId,
                    url: resolved,
                    title: targetTitle ? String(targetTitle) : null
                });
            };

            const originalOpen = window.open;
            window.open = function(url, name) {
                if (url) {
                    __vesselOpenTab(url, name || null);
                    return null;
                }
                return originalOpen ? originalOpen.apply(window, arguments) : null;
            };

            const findAnchor = (e) => {
                if (e.target && e.target.closest) {
                    const viaClosest = e.target.closest('a[href]');
                    if (viaClosest) return viaClosest;
                }

                if (typeof e.composedPath === 'function') {
                    const path = e.composedPath();
                    for (const node of path) {
                        if (!node || typeof node !== 'object') continue;
                        if (node.tagName === 'A' && node.href) return node;
                    }
                }
                return null;
            };

            document.addEventListener('click', function(e) {
                const anchor = findAnchor(e);
                if (!anchor || !anchor.href) return;

                const isModifiedClick = e.metaKey || e.ctrlKey;
                const opensBlank = anchor.target === '_blank';
                if (isModifiedClick || opensBlank) {
                    e.preventDefault();
                    __vesselOpenTab(anchor.href, anchor.title || anchor.textContent || null);
                }
            }, true);

            document.addEventListener('auxclick', function(e) {
                if (e.button !== 1) return;
                const anchor = findAnchor(e);
                if (!anchor || !anchor.href) return;
                e.preventDefault();
                __vesselOpenTab(anchor.href, anchor.title || anchor.textContent || null);
            }, true);
        })();
        "#
    .replace("__APP_ID_JSON__", &app_id_json)
}

pub fn upsert_style_script(style_id: &str, css: &str) -> String {
    let style_id_json = serde_json::to_string(style_id).unwrap_or_else(|_| "\"\"".to_string());
    let css_json = serde_json::to_string(css).unwrap_or_else(|_| "\"\"".to_string());
    format!(
        "(function(){{const id={};const css={};let style=document.getElementById(id);if(!style){{style=document.createElement('style');style.id=id;document.head.appendChild(style);}}style.textContent=css;}})();",
        style_id_json, css_json
    )
}

pub fn upsert_runtime_js_script(script_id: &str, js: &str) -> String {
    let script_id_json = serde_json::to_string(script_id).unwrap_or_else(|_| "\"\"".to_string());
    let js_json = serde_json::to_string(js).unwrap_or_else(|_| "\"\"".to_string());
    format!(
        "(function(){{window.__vesselInjectedScripts=window.__vesselInjectedScripts||{{}};const id={};const payload={};if(window.__vesselInjectedScripts[id]===payload)return;window.__vesselInjectedScripts[id]=payload;(0,eval)(payload);}})();",
        script_id_json, js_json
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_contains_app_id() {
        let s = new_tab_bridge_script("demo-app");
        assert!(s.contains("demo-app"));
    }
}
