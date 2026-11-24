use dioxus::prelude::*;
use crate::api::discord;

#[component]
pub fn DiscordSettings(token:String)->Element{
    let request_token=token.clone();
    let mut current=use_resource(move ||{
        let token=request_token.clone();
        async move{discord::status(&token).await}
    });
    let mut code=use_signal(||None::<discord::DiscordCode>);
    let mut busy=use_signal(||false);
    let mut error=use_signal(||None::<String>);
    let code_token=token.clone();
    let unlink_token=token.clone();
    rsx!{
        section{class:"settings-panel",
            div{class:"settings-panel-heading",
                h2{"Discord"}
                p{"Associez votre compte à Discord pour traiter les tickets sans quitter Discord."}
            }
            div{class:"settings-panel-content",
                match current.read().clone(){
                    Some(Ok(status))=>rsx!{
                        if status.enabled{
                            if let Some(id)=status.discord_user_id{
                                div{class:"settings-account-meta",
                                    span{"Compte Discord lié"}
                                    strong{"ID {id}"}
                                }
                                button{
                                    class:"button outline",r#type:"button",disabled:*busy.read(),
                                    onclick:move |_|{
                                        if *busy.read(){return;}
                                        let t=unlink_token.clone();
                                        busy.set(true);
                                        error.set(None);
                                        spawn(async move{
                                            match discord::unlink(&t).await{
                                                Ok(())=>{
                                                    code.set(None);
                                                    current.restart();
                                                },
                                                Err(err)=>error.set(Some(err)),
                                            }
                                            busy.set(false);
                                        });
                                    },
                                    "Délier ce compte"
                                }
                            }else{
                                p{"Aucun compte Discord lié pour le moment."}
                            }
                            button{
                                class:"button solid",r#type:"button",disabled:*busy.read(),
                                onclick:move |_|{
                                    if *busy.read(){return;}
                                    let t=code_token.clone();
                                    busy.set(true);
                                    error.set(None);
                                    code.set(None);
                                    spawn(async move{
                                        match discord::new_code(&t).await{
                                            Ok(result)=>code.set(Some(result)),
                                            Err(err)=>error.set(Some(err)),
                                        }
                                        busy.set(false);
                                    });
                                },
                                if *busy.read(){"Chargement…"}else{"Générer un code de liaison"}
                            }
                            if let Some(value)=code.read().as_ref(){
                                div{class:"settings-group",
                                    strong{"Code temporaire (10 minutes, usage unique)"}
                                    code{class:"discord-link-code","{value.code}"}
                                    p{class:"settings-note",
                                        "Depuis le serveur Discord configuré, tapez /seppyl lier "
                                        "et collez ce code. Ne communiquez jamais ce code à un tiers."
                                    }
                                }
                            }
                        }else{
                            p{class:"settings-note",
                                "Les interactions Discord ne sont pas encore activées "
                                "dans la configuration du serveur SEPPYL."
                            }
                        }
                    },
                    Some(Err(err))=>rsx!{p{class:"settings-feedback error","{err}"}},
                    None=>rsx!{p{"Chargement de la liaison Discord…"}},
                }
                if let Some(err)=error.read().as_ref(){
                    p{class:"settings-feedback error",role:"alert","{err}"}
                }
            }
        }
    }
}
