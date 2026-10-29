//! API endpoint configuration shared by all browser requests.
//! reqwest requires absolute URLs, including on wasm32.
//! SEPPYL_API_URL can override the default at compile time (dev, staging, CI).
pub(crate) fn base_url() -> &'static str {
    option_env!("SEPPYL_API_URL").unwrap_or(if cfg!(debug_assertions) {
        "http://127.0.0.1:3000"
    } else {
        "https://seppyl.donnarieix.fr/api"
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn api_endpoint_is_an_absolute_http_url() {
        let url = reqwest::Url::parse(&format!("{}/login", super::base_url()))
            .expect("reqwest needs an absolute API URL");
        assert!(matches!(url.scheme(), "http" | "https"));
        assert_eq!(url.path().ends_with("/login"), true);
    }
}

pub mod auth;
pub mod matrix;
pub mod trainings;
pub mod catalog;
pub mod players;
pub mod preferences;
pub mod account;
