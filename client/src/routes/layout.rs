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
                }
                button { class: "logout-link", onclick: move |_| {
                    session.set(None);
                    navigator.push(Route::Login {});
                }, "Déconnexion" }
            }
        }
    }
}
