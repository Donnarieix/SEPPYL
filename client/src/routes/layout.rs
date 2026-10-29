use dioxus::prelude::*;
use super::Route;

/// Desktop navigation stays in the header. On touch devices, primary routes
/// move to a bottom navigation bar with comfortable touch targets.
#[component]
pub fn Header() -> Element {
    let mut session = use_context::<Signal<Option<String>>>();
    let navigator = use_navigator();
    let current = use_route::<Route>();
    rsx! {
        header { class: "topbar",
            div { class: "topbar-inner",
                Link { to: Route::Trainings {}, class: "logo", "SEPPYL" }
                nav { class: "topbar-nav desktop-nav", "aria-label": "Navigation principale",
                    Link { to: Route::Trainings {},
                        class: if matches!(&current, Route::Trainings {} | Route::TrainingDetail { .. } | Route::TrainingPrint { .. }) { "active" } else { "" },
                        "Entraînements" }
                    Link { to: Route::Home {},
                        class: if matches!(&current, Route::Home {}) { "active" } else { "" },
                        "Tableur" }
                    Link { to: Route::Players {},
                        class: if matches!(&current, Route::Players {}) { "active" } else { "" },
                        "Joueuses" }
                    Link { to: Route::Activities {},
                        class: if matches!(&current, Route::Activities {}) { "active" } else { "" },
                        "Activités" }
                    Link { to: Route::Tactics {},
                        class: if matches!(&current, Route::Tactics {}) { "active" } else { "" },
                        "Tactiques" }
                }
                div { class: "topbar-utilities",
                    Link {
                        to: Route::AccountPage {},
                        class: if matches!(&current, Route::AccountPage {}) { "topbar-account active" } else { "topbar-account" },
                        "Mon compte"
                    }
                    button { class: "logout-link", onclick: move |_| {
                        crate::session::clear();
                        session.set(None);
                        navigator.push(Route::Login {});
                    }, "Déconnexion" }
                }
            }
        }
        nav { class: "mobile-nav", "aria-label": "Navigation mobile",
            Link { to: Route::Trainings {},
                class: if matches!(&current, Route::Trainings {} | Route::TrainingDetail { .. } | Route::TrainingPrint { .. }) { "mobile-nav-link active" } else { "mobile-nav-link" },
                svg { width: "21", height: "21", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "1.8",
                    rect { x: "3", y: "5", width: "18", height: "16", rx: "2" }
                    path { d: "M7 3v4M17 3v4M3 10h18" }
                }
                span { "Séances" }
            }
            Link { to: Route::Home {},
                class: if matches!(&current, Route::Home {}) { "mobile-nav-link active" } else { "mobile-nav-link" },
                svg { width: "21", height: "21", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "1.8",
                    rect { x: "3", y: "3", width: "18", height: "18", rx: "2" }
                    path { d: "M3 9h18M3 15h18M9 3v18" }
                }
                span { "Tableur" }
            }
            Link { to: Route::Players {},
                class: if matches!(&current, Route::Players {}) { "mobile-nav-link active" } else { "mobile-nav-link" },
                svg { width: "21", height: "21", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "1.8",
                    path { d: "M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2" }
                    circle { cx: "9", cy: "7", r: "4" }
                    path { d: "M22 21v-2a4 4 0 0 0-3-3.87M16 3.13a4 4 0 0 1 0 7.75" }
                }
                span { "Joueuses" }
            }
            Link { to: Route::Activities {},
                class: if matches!(&current, Route::Activities {}) { "mobile-nav-link active" } else { "mobile-nav-link" },
                svg { width: "21", height: "21", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "1.8",
                    rect { x: "4", y: "4", width: "16", height: "16", rx: "2" }
                    path { d: "M8 9h8M8 13h8M8 17h5" }
                }
                span { "Activités" }
            }
            Link { to: Route::Tactics {},
                class: if matches!(&current, Route::Tactics {}) { "mobile-nav-link active" } else { "mobile-nav-link" },
                svg { width:"21", height:"21", view_box:"0 0 24 24",
                    fill:"none", stroke:"currentColor", stroke_width:"1.8",
                    circle { cx:"12", cy:"12", r:"9" }
                    path { d:"M12 3v18M3 12h18" }
                }
                span { "Tactiques" }
            }
            Link { to: Route::AccountPage {},
                class: if matches!(&current, Route::AccountPage {}) { "mobile-nav-link active" } else { "mobile-nav-link" },
                svg { width: "21", height: "21", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "1.8",
                    circle { cx: "12", cy: "8", r: "4" }
                    path { d: "M4 21v-2a8 8 0 0 1 16 0v2" }
                }
                span { "Compte" }
            }
        }
    }
}
