use dioxus::prelude::*;

fn escape(value: &str) -> String {
    value.replace('&', "&amp;").replace('<', "&lt;")
        .replace('>', "&gt;").replace('"', "&quot;").replace('\n', "<br>")
}

/// Direct editing with bold, italic, underline, headings and lists. The API
/// sanitizes all submitted HTML. This component never trusts raw paste HTML.
#[component]
pub fn RichTextEditor(
    initial_html: Option<String>,
    initial_plain: String,
    on_html: EventHandler<String>,
    on_plain: EventHandler<String>,
) -> Element {
    let rendered = initial_html.clone()
        .unwrap_or_else(|| escape(&initial_plain));
    rsx! {
        div { class: "seppyl-rich-editor",
            div { class: "rich-toolbar", role:"toolbar", "aria-label":"Mise en forme",
                button { r#type:"button", "data-rich-command":"bold", title:"Gras", "aria-label":"Gras", strong { "B" } }
                button { r#type:"button", "data-rich-command":"italic", title:"Italique", "aria-label":"Italique", i { "I" } }
                button { r#type:"button", "data-rich-command":"underline", title:"Souligné", "aria-label":"Souligné", u { "U" } }
                span { class:"rich-separator" }
                button { r#type:"button", "data-rich-command":"formatBlock",
                    "data-rich-value":"h3", title:"Titre", "Titre" }
                button { r#type:"button", "data-rich-command":"insertUnorderedList",
                    title:"Liste à puces", "• Liste" }
                button { r#type:"button", "data-rich-command":"insertOrderedList",
                    title:"Liste numérotée", "1. Liste" }
                span { class:"rich-separator" }
                button { r#type:"button", "data-rich-command":"removeFormat", title:"Effacer la mise en forme", "Effacer" }
            }
            div { class: "rich-surface", "data-rich-area":"true",
                contenteditable:"true", role:"textbox",
                "aria-multiline":"true",
                "aria-label":"Description de l'activité",
                "data-placeholder":"Objectifs, déroulement et consignes…",
                dangerous_inner_html: rendered
            }
            textarea { "data-rich-html":"true", class:"rich-hidden-input",
                "aria-hidden":"true", tabindex:"-1",
                oninput: move |e| on_html.call(e.value()) }
            textarea { "data-rich-plain":"true", class:"rich-hidden-input",
                "aria-hidden":"true", tabindex:"-1",
                oninput: move |e| on_plain.call(e.value()) }
        }
    }
}
