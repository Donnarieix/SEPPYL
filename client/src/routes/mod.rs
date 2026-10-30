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
mod tactics;
mod organization;
mod club_information;
mod logo_picker;
mod administration;
mod membership;
mod team_management;
mod public_directory;
mod audit;

use home::Home;
use login::Login;
use training_detail::TrainingDetail;
use training_print::TrainingPrint;
use trainings::Trainings;
use players::Players;
use activities::Activities;
use account::AccountPage;
use tactics::Tactics;
use organization::Organization;
use administration::Administration;
use public_directory::PublicDirectory;
use audit::AuditLog;

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
    #[route("/tactics")]
    Tactics {},
    #[route("/club")]
    Organization {},
    #[route("/admin")]
    Administration {},
    #[route("/audit")]
    AuditLog {},
    #[route("/settings")]
    AccountPage {},
    #[route("/public")]
    PublicDirectory {},
    #[route("/login")]
    Login {},
    #[route("/trainings/:id")]
    TrainingDetail { id: String },
    #[route("/trainings/:id/print")]
    TrainingPrint { id: String },
}
