use dioxus::prelude::*;

mod dialogs;
mod home;
mod layout;
mod login;
mod not_found;
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
pub mod announcements;
mod audit;
pub mod changelog;
pub mod legal;

use home::Home;
use not_found::NotFound;

/// Never navigate to arbitrary URLs after login. Only allow known, protected
/// SEPPYL routes, without ambiguous host/authority or encoded path separators.
fn valid_internal_destination(candidate: &str) -> bool {
    if candidate.is_empty() || candidate.len() > 2048 || !candidate.starts_with('/')
        || candidate.starts_with("//") || candidate.contains('\\')
        || candidate.chars().any(char::is_control) {return false;}
    let path=candidate.split(['?', '#']).next().unwrap_or_default();
    if path.contains('%') {return false;}
    let static_routes=["/trainings","/matches","/tickets","/spreadsheet",
        "/players","/activities","/tactics","/club","/admin","/audit","/settings"];
    if static_routes.contains(&path){return true;}
    for prefix in ["/tickets/", "/matches/", "/trainings/"] {
        if let Some(tail)=path.strip_prefix(prefix){
            let id=tail.strip_suffix("/print").unwrap_or(tail);
            if prefix!="/trainings/" && id!=tail {return false;}
            if id.len()==36 && id.bytes().enumerate().all(|(i,b)|
                if [8,13,18,23].contains(&i){b==b'-'}else{b.is_ascii_hexdigit()}) {
                return true;
            }
        }
    }
    false
}

/// Return to the requested internal page after authentication, otherwise use
/// the ordinary landing page. Never use an unchecked string navigation target.
pub(crate) fn login_destination(next: &str) -> Route {
    if valid_internal_destination(next) {
        let path=next.split(['?', '#']).next().unwrap_or_default();
        let id=path.rsplit('/').next().unwrap_or_default();
        // Dioxus typed navigation avoids external / javascript: redirects.
        if let Some(id)=path.strip_prefix("/tickets/"){return Route::TicketDetail{id:id.to_owned()};}
        if let Some(id)=path.strip_prefix("/matches/"){return Route::MatchDetail{id:id.to_owned()};}
        if let Some(tail)=path.strip_prefix("/trainings/") {
            if let Some(id)=tail.strip_suffix("/print") {return Route::TrainingPrint{id:id.to_owned()};}
            return Route::TrainingDetail{id:tail.to_owned()};
        }
        let _=id;
        return match path {
            "/trainings"=>Route::Trainings{},
            "/matches"=>Route::Matches{},
            "/tickets"=>Route::Tickets{},
            "/spreadsheet"=>Route::Home{},
            "/players"=>Route::Players{},
            "/activities"=>Route::Activities{},
            "/tactics"=>Route::Tactics{},
            "/club"=>Route::Organization{},
            "/admin"=>Route::Administration{},
            "/audit"=>Route::AuditLog{},
            "/settings"=>Route::AccountPage{},
            _=>Route::Trainings{},
        };
    }
    Route::Trainings{}
}

#[component]
fn RootRedirect()->Element{
    let session=use_context::<Signal<Option<String>>>();
    let navigator=use_navigator();
    use_effect(move || {
        let destination=if session.read().is_some(){Route::Trainings{}}else{Route::Login{next:String::new()}};
        navigator.replace(destination);
    });
    rsx! {}
}

#[component]
fn RequireAuth()->Element{
    let session=use_context::<Signal<Option<String>>>();
    let navigator=use_navigator();
    let authenticated=session.read().is_some();
    let requested=use_route::<Route>().to_string();
    use_effect(move || {
        if session.read().is_none(){
            navigator.replace(Route::Login{next:requested.clone()});
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
use audit::AuditLog;
use legal::{LegalNotice,PrivacyPolicy,TermsOfUse};

#[derive(Routable, Clone, PartialEq)]
pub enum Route {
    #[route("/")]
    RootRedirect {},
    #[route("/login?:next")]
    Login {next:String},
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
    #[end_layout]
    #[route("/:..segments")]
    NotFound { segments: Vec<String> },
}

#[cfg(test)]
mod redirect_safety_tests {
    use super::*;
    #[test]
    fn only_known_internal_routes_are_accepted(){
        for path in ["/tickets/123e4567-e89b-12d3-a456-426614174000",
            "/trainings","/matches/123e4567-e89b-12d3-a456-426614174000",
            "/trainings/123e4567-e89b-12d3-a456-426614174000/print?mode=compact#top",
            "/settings"]{
            assert!(valid_internal_destination(path),"{path}");
        }
        for path in ["https://evil.example","//evil.example","/\\evil.example",
            "javascript:alert(1)","/%2f%2fevil.example","/login","/",
            "/tickets/invalid","/unknown","/admin/other","/tickets/%2F%2Fevil.example",
            "/trainings/123e4567-e89b-12d3-a456-426614174000/garbage"] {
            assert!(!valid_internal_destination(path),"{path}");
        }
        assert!(matches!(login_destination("//evil.example"),Route::Trainings{}));
        assert!(matches!(login_destination("/tickets/123e4567-e89b-12d3-a456-426614174000"),Route::TicketDetail{..}));
    }
}

