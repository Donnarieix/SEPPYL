use dioxus::prelude::*;

#[component]
pub fn Home() -> Element {
    rsx! {
        h1 { "Home Page" }
        p { "Welcome to the home page!" }
    }
}

#[component]
pub fn Counter() -> Element {
    let mut count = use_signal(|| 0);
    rsx! {
        p { "Count: {count}" }
        button {
            onclick: move |_| count += 1,
            "Increment"
        }
    }
}