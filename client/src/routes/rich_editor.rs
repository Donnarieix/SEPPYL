use dioxus::prelude::*;

/// Small WYSIWYG activity editor: HTML is sanitized by the API on save/read.
/// Client-side paste is plain text to avoid importing arbitrary external markup.
#[component]
pub fn RichEditor(initial:String,on_change:EventHandler<String>)->Element{
    let mut content=use_signal(||initial.clone());
    rsx!{
        div { class:"rich-editor", "data-rich-root":"true",
            div { class:"rich-toolbar", role:"toolbar", "aria-label":"Mise en forme",
                button { r#type:"button","data-rich-command":"bold",title:"Gras", "aria-label":"Gras",strong{"G"} }
                button { r#type:"button","data-rich-command":"italic",title:"Italique", "aria-label":"Italique",em{"I"} }
                button { r#type:"button","data-rich-command":"underline",title:"Souligné", "aria-label":"Souligné",u{"S"} }
                span { class:"rich-toolbar-divider" }
                button { r#type:"button","data-rich-command":"formatBlock","data-rich-value":"p",title:"Paragraphe","¶" }
                button { r#type:"button","data-rich-command":"formatBlock","data-rich-value":"h3",title:"Titre","H" }
                span { class:"rich-toolbar-divider" }
                button { r#type:"button","data-rich-command":"insertUnorderedList",title:"Liste à puces","• Liste" }
                button { r#type:"button","data-rich-command":"insertOrderedList",title:"Liste numérotée","1. Liste" }
                button { r#type:"button","data-rich-command":"undo",title:"Annuler","↶" }
                button { r#type:"button","data-rich-command":"redo",title:"Rétablir","↷" }
            }
            div {
                class:"rich-surface",
                "data-rich-input":"true",
                "contenteditable":"true",
                role:"textbox",
                "aria-multiline":"true",
                "aria-label":"Description et consignes de l'activité",
                "data-placeholder":"Rédigez les objectifs, les consignes et le déroulement de l’activité…",
                dangerous_inner_html: "{initial}",
            }
            textarea {
                class:"rich-output",
                "data-rich-output":"true",
                value:"{content}",
                tabindex:"-1",
                "aria-hidden":"true",
                oninput:move |e|{
                    let value=e.value();
                    content.set(value.clone());
                    on_change.call(value);
                }
            }
        }
    }
}
