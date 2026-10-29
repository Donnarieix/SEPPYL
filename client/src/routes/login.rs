use dioxus::prelude::*;
use crate::api::auth::Api;

#[component]
pub fn Login() -> Element {
    let api = use_context::<Api>();
    let mut username = use_signal(String::new);
    let mut password = use_signal(String::new);

    rsx! {
        h1 { "Login Page" }
        p { "Please enter your credentials to log in." }
        input {
            r#type: "text",
            placeholder: "Username",
            value: "{username}",
            oninput: move |e| username.set(e.value()),
        }
        input {
            r#type: "password",
            placeholder: "Password",
            value: "{password}",
            oninput: move |e| password.set(e.value()),
        }
        button {
            onclick: move |_| {
                let mut api = api.clone();
                async move {
                    let username = username.read().clone();
                    let password = password.read().clone();
                    match api.login(&username, &password).await {
                        Ok(_) => println!("Login successful"),
                        Err(error) => println!("Login failed: {error}"),
                    }
                }
            },
            "Login"
        }
    }
}
