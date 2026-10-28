use dioxus::prelude::*;

mod dialogs;
mod home;
mod layout;
mod login;
mod training_detail;
mod training_print;
mod trainings;

use home::Home;
use login::Login;
use training_detail::TrainingDetail;
use training_print::TrainingPrint;
use trainings::Trainings;

#[derive(Routable, Clone, PartialEq)]
pub enum Route {
    #[route("/")]
    Trainings {},
    #[route("/tableau")]
    Home {},
    #[route("/login")]
    Login {},
    #[route("/entrainements/:id")]
    TrainingDetail { id: String },
    #[route("/entrainements/:id/fiche")]
    TrainingPrint { id: String },
}
