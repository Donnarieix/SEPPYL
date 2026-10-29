use dioxus::prelude::*;

mod dialogs;
mod home;
mod layout;
mod login;
mod training_detail;
mod training_print;
mod trainings;
mod players;
mod activities;
mod account;
mod rich_editor;

use home::Home;
use login::Login;
use training_detail::TrainingDetail;
use training_print::TrainingPrint;
use trainings::Trainings;
use players::Players;
use activities::Activities;
use account::AccountPage;

#[derive(Routable, Clone, PartialEq)]
pub enum Route {
    #[route("/")]
    Trainings {},
    #[route("/spreadsheet")]
    Home {},
    #[route("/players")]
    Players {},
    #[route("/activities")]
    Activities {},
    #[route("/settings")]
    AccountPage {},
    #[route("/login")]
    Login {},
    #[route("/trainings/:id")]
    TrainingDetail { id: String },
    #[route("/trainings/:id/print")]
    TrainingPrint { id: String },
}
