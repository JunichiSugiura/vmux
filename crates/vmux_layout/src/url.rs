//! URL scheme checks for persisted / navigable strings.

use crate::LastVisitedUrl;

const MAX_URL_LEN: usize = 4096;

/// Allow only navigable schemes for persisted URLs.
pub fn allowed_navigation_url(url: &str) -> bool {
    let url = url.trim();
    if url.is_empty() || url.len() > MAX_URL_LEN {
        return false;
    }
    let Some((scheme, _)) = url.split_once(':') else {
        return false;
    };
    matches!(
        scheme.to_ascii_lowercase().as_str(),
        "http" | "https" | "cef"
    )
}

/// Initial `WebviewSource` URL: last session if valid, else `fallback`.
pub fn initial_webview_url(last: Option<&LastVisitedUrl>, fallback: &str) -> String {
    let Some(last) = last else {
        return fallback.to_string();
    };
    let u = last.0.trim();
    if u.is_empty() || !allowed_navigation_url(u) {
        fallback.to_string()
    } else {
        u.to_string()
    }
}
