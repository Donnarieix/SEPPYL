//! Shared API base used by every browser request.
//! Release artifacts are served behind https://seppyl.donnarieix.fr/api;
//! local development keeps the existing loopback endpoint.
pub(crate) fn base_url()->&'static str{
    option_env!("SEPPYL_API_URL").unwrap_or(if cfg!(debug_assertions){
        "http://127.0.0.1:3000"
    }else{
        "https://seppyl.donnarieix.fr/api"
    })
}
#[cfg(test)]
mod endpoint_tests{
    #[test]
    fn release_base_is_absolute(){
        let uri=reqwest::Url::parse(&format!("{}/health",super::base_url())).unwrap();
        assert!(matches!(uri.scheme(),"http"|"https"));
    }
}

pub mod auth;
pub mod matrix;
pub mod trainings;
pub mod catalog;
pub mod players;
pub mod preferences;
pub mod account;
pub mod tactics;
pub mod organization;
pub mod audit;

pub mod changelog;
