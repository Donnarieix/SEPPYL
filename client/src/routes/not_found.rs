use dioxus::prelude::*;
use super::Route;

#[component]
pub fn NotFound(segments:Vec<String>)->Element{
    // Never echo an untrusted URL into the error page.
    let _=segments;
    rsx!{
        document::Title{"Page introuvable | SEPPYL"}
        document::Meta{name:"robots",content:"noindex,nofollow,noarchive"}
        main{class:"not-found-page",
            div{class:"not-found-content",
                span{class:"not-found-code","404"}
                h1{"Page introuvable"}
                p{"Cette adresse n'existe pas ou cette page n'est plus disponible."}
                div{class:"not-found-actions",
                    Link{to:Route::RootRedirect{},class:"button solid","Retour à l'accueil"}
                    Link{to:Route::PublicDirectory{},class:"button outline","Explorer les clubs"}
                }
            }
        }
    }
}
