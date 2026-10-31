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
mod team_information;
mod administration;
mod membership;
mod team_management;
mod public_directory;
mod matches;
mod tickets;
mod announcements;
mod audit;
pub mod changelog;
pub mod legal;

use home::Home;

#[component]
fn RootRedirect()->Element{
    let session=use_context::<Signal<Option<String>>>();
    let navigator=use_navigator();
    use_effect(move || {
        let destination=if session.read().is_some(){Route::Trainings{}}else{Route::Login{}};
        navigator.replace(destination);
    });
    rsx! {}
}

#[component]
fn RequireAuth()->Element{
    let session=use_context::<Signal<Option<String>>>();
    let navigator=use_navigator();
    let authenticated=session.read().is_some();
    use_effect(move || {
        if session.read().is_none(){
            navigator.replace(Route::Login{});
        }
    });
    if !authenticated{return rsx!{};}
    rsx!{Outlet::<Route>{}}
}

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
use matches::{Matches,MatchDetail};
use tickets::{Tickets,TicketDetail};
use announcements::AnnouncementAdmin;
use audit::AuditLog;
use legal::{LegalNotice,PrivacyPolicy,TermsOfUse};

#[derive(Routable, Clone, PartialEq)]
pub enum Route {
    #[route("/")]
    RootRedirect {},
    #[route("/login")]
    Login {},
    #[route("/public")]
    PublicDirectory {},
    #[route("/mentions-legales")]
    LegalNotice {},
    #[route("/confidentialite")]
    PrivacyPolicy {},
    #[route("/cgu")]
    TermsOfUse {},

    #[layout(RequireAuth)]
    #[route("/trainings")]
    Trainings {},
    #[route("/matches")]
    Matches {},
    #[route("/matches/:id")]
    MatchDetail { id:String },
    #[route("/tickets")]
    Tickets {},
    #[route("/tickets/:id")]
    TicketDetail {id:String},
    #[route("/announcements/manage")]
    AnnouncementAdmin {},
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
    #[route("/trainings/:id")]
    TrainingDetail { id: String },
    #[route("/trainings/:id/print")]
    TrainingPrint { id: String },
}
