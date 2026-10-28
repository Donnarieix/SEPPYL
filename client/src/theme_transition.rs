//! Circular View Transition from the click position, with graceful fallback.
use dioxus::prelude::*;
use crate::api::preferences::{Theme, UserPreferences};

pub async fn reveal_theme(
    mut appearance: Signal<UserPreferences>,
    next: Theme,
    x: f64,
    y: f64,
) {
    #[cfg(feature = "web")]
    {
        let script = include_str!("theme_reveal.js")
            .replace("__X__", &x.to_string())
            .replace("__Y__", &y.to_string());
        let mut transition = document::eval(&script);
        // The JavaScript callback captures the old frame before applying theme.
        if transition.recv::<String>().await.is_ok() {
            let before = *appearance.read();
            appearance.set(UserPreferences { theme: next, ..before });
            // Keep the bridge alive until the animation has finished.
            let _ = transition.recv::<String>().await;
            return;
        }
    }
    let before = *appearance.read();
    appearance.set(UserPreferences { theme: next, ..before });
}
