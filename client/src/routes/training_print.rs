use dioxus::prelude::*;
use std::collections::BTreeMap;
use crate::api::trainings;
use super::Route;

#[component]
pub fn TrainingPrint(id: String) -> Element {
    let session = use_context::<Signal<Option<String>>>();
    let token = session.read().clone();
    let fetch_id = id.clone();
    let mut data = use_resource(move || {
        let id = fetch_id.clone();
        let token = token.clone();
        async move {
            match token {
                Some(token) => trainings::get(&token, &id).await,
                None => Err("Connectez-vous avant d'imprimer la fiche.".into()),
            }
        }
    });

    rsx! {
        main { class: "print-sheet",
            div { class: "print-guidance",
                p { "Fiche d'entraînement — pour la conserver hors connexion : Ctrl+P → Enregistrer au format PDF." }
                Link { to: Route::TrainingDetail { id: id.clone() }, "← Retour à l'entraînement" }
            }
            match data.read().clone() {
                Some(Ok(detail)) => rsx! {
                    header { class: "print-heading",
                        div { class: "print-logo", "SEPPYL" }
                        h1 { "{detail.training.name}" }
                        p { "Date : {detail.training.date.get(..10).unwrap_or(&detail.training.date)}" }
                    }
                    if !detail.training.notes.trim().is_empty() {
                        section { class: "print-section",
                            h2 { "Informations" }
                            p { class: "preformatted", "{detail.training.notes}" }
                        }
                    }
                    section { class: "print-section",
                        h2 { "Déroulement" }
                        if detail.activities.is_empty() {
                            p { "Aucune activité prévue." }
                        }
                        for (index, activity) in detail.activities.iter().enumerate() {
                            article { class: "print-activity", key: "{activity.id}",
                                h3 { "{index + 1}. {activity.name}" }
                                if !activity.description.trim().is_empty() {
                                    p { class: "preformatted", "{activity.description}" }
                                }
                                if let Some(url) = activity.image_url.as_ref() {
                                    img { class: "print-picture", src: "{url}", alt: "Illustration de {activity.name}" }
                                }
                                {
                                    let materials: Vec<_> = detail.materials.iter()
                                        .filter(|m| m.activity_id == activity.id).collect();
                                    rsx! {
                                        if !materials.is_empty() {
                                            div { class: "print-materials",
                                                strong { "Matériel : " }
                                                for material in materials {
                                                    span { "{material.name} × {material.quantity}  " }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    {
                        let mut totals = BTreeMap::<String, i32>::new();
                        for material in &detail.materials {
                            *totals.entry(material.name.trim().to_string()).or_default() += material.quantity;
                        }
                        rsx! {
                            if !totals.is_empty() {
                                section { class: "print-section",
                                    h2 { "Matériel à préparer" }
                                    ul { class: "print-totals",
                                        for (name, total) in totals {
                                            li { "{name} : {total}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
                Some(Err(error)) => rsx! { p { role: "alert", "{error}" } },
                None => rsx! { p { "Chargement de la fiche…" } },
            }
        }
    }
}
