use dioxus::prelude::*;

use super::Route;

#[component]
pub fn Header() -> Element {
    let mut session = use_context::<Signal<Option<String>>>();
    let navigator = use_navigator();

    rsx! {
        header { class: "topbar",
            div { class: "topbar-inner",
                Link { to: Route::Trainings {}, class: "logo", "SEPPYL" }
                nav { class: "topbar-nav",
                    Link { to: Route::Trainings {}, "Entraînements" }
                    Link { to: Route::Home {}, "Tableur" }
                    Link { to: Route::Players {}, "Joueuses" }
                    Link { to: Route::Activities {}, "Activités" }
                }
                button { class: "logout-link", onclick: move |_| {
                    crate::session::clear();
                    session.set(None);
                    navigator.push(Route::Login {});
                }, "Déconnexion" }
            }
        }
    }
}
