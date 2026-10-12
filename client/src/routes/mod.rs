use dioxus::prelude::*;

mod home;
mod login;

use home::{Home, Counter};
use login::Login;

#[derive(Routable, Clone)]
pub enum Route {
    #[route("/")]
    Home,
    #[route("/counter")]
    Counter,
    #[route("/login")]
    Login,
}