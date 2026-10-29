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

pub fn observe_status(status:reqwest::StatusCode)->reqwest::StatusCode{
    if status==reqwest::StatusCode::UNAUTHORIZED {
        // An API 401 means an invalid/expired session, never just a form error.
        clear();
        #[cfg(feature="web")]
        if let Some(window)=web_sys::window(){
            let _=window.location().replace("/login");
        }
    }
    status
}
