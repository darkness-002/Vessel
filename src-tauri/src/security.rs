use url::Url;

pub const MAX_INJECTION_BYTES: usize = 32 * 1024;

pub fn normalize_allowlist(raw: Option<Vec<String>>) -> Vec<String> {
    raw.unwrap_or_default()
        .into_iter()
        .flat_map(|entry| {
            entry
                .split(|c| c == ',' || c == '\n' || c == ' ')
                .map(str::trim)
                .filter(|part| !part.is_empty())
                .map(|part| part.trim_start_matches("*.").trim_start_matches('.').to_lowercase())
                .collect::<Vec<String>>()
        })
        .collect()
}

pub fn is_host_allowed(url: &Url, allowlist: &[String]) -> bool {
    if allowlist.is_empty() {
        return false;
    }

    let Some(host) = url.host_str().map(|h| h.to_lowercase()) else {
        return false;
    };

    allowlist
        .iter()
        .any(|domain| host == *domain || host.ends_with(&format!(".{domain}")))
}

pub fn validate_css_payload(css: &str) -> Result<(), String> {
    if css.len() > MAX_INJECTION_BYTES {
        return Err(format!("css payload exceeds {} bytes", MAX_INJECTION_BYTES));
    }
    Ok(())
}

pub fn validate_js_payload(js: &str) -> Result<(), String> {
    if js.len() > MAX_INJECTION_BYTES {
        return Err(format!("js payload exceeds {} bytes", MAX_INJECTION_BYTES));
    }

    let lowered = js.to_lowercase();
    let blocked_patterns = [
        "window.__tauri__",
        "__tauri_internals__",
        "core.invoke(",
        "fetch('file://",
        "fetch(\"file://",
        "xmlhttprequest",
        "websocket(",
        "import(",
        "function(",
        "eval(",
    ];

    for pattern in blocked_patterns {
        if lowered.contains(pattern) {
            return Err(format!("js payload contains blocked pattern: {pattern}"));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowlist_matches_subdomain() {
        let url = Url::parse("https://sub.example.com/path").expect("valid url");
        let allowlist = normalize_allowlist(Some(vec!["example.com".to_string()]));
        assert!(is_host_allowed(&url, &allowlist));
    }

    #[test]
    fn blocked_js_pattern_rejected() {
        let js = "window.__TAURI__.core.invoke('x')";
        assert!(validate_js_payload(js).is_err());
    }

    #[test]
    fn css_limit_enforced() {
        let css = "a".repeat(MAX_INJECTION_BYTES + 1);
        assert!(validate_css_payload(&css).is_err());
    }
}
