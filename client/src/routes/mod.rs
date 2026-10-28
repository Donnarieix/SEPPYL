use dioxus::prelude::*;

mod dialogs;
mod home;
mod login;

use home::Home;
use login::Login;

#[derive(Routable, Clone, PartialEq)]
pub enum Route {
    #[route("/")]
    Home {},
    #[route("/login")]
    Login {},
}
