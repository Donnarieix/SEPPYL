//! Temporary local JWT session adapter.
//! Replace this module with an OIDC/SSO session provider when Donnarieix SSO is ready.
//! This does not store passwords. For production, prefer same-site HttpOnly cookies.
#[cfg(feature = "web")]
use gloo_storage::{LocalStorage, Storage};
const STORAGE_KEY: &str = "seppyl.session.v1";

pub fn restore() -> Option<String> {
    #[cfg(feature = "web")]
    {
        let saved: String = LocalStorage::get(STORAGE_KEY).ok()?;
        if saved.is_empty() { None } else { Some(saved) }
    }
    #[cfg(not(feature = "web"))]
    { None }
}

pub fn save(token: &str) {
    #[cfg(feature = "web")]
    { let _ = LocalStorage::set(STORAGE_KEY, token); }
    #[cfg(not(feature = "web"))]
    { let _ = token; }
}
pub fn clear() {
    #[cfg(feature = "web")]
    LocalStorage::delete(STORAGE_KEY);
}
