use dioxus::prelude::*;
use crate::api::{organization as org,catalog};

/// Reused in the operational "Mon club" and independent global administration.
/// Stores only safe text and an approved HTTPS or uploaded image URL.
#[component]
pub fn ClubInformationEditor(
    token:String,club:org::Club,on_saved:EventHandler<()>,on_error:EventHandler<String>,
)->Element{
    let mut name=use_signal(||club.name.clone());
    let mut description=use_signal(||club.description.clone().unwrap_or_default());
    let mut logo=use_signal(||club.logo_url.clone().unwrap_or_default());
    let mut public=use_signal(||club.is_public==Some(true));
    let mut saving=use_signal(||false);
    let mut uploading=use_signal(||false);
    let preview=org::club_logo_src(Some(logo.read().as_str()));
    rsx!{
        section{class:"panel club-information-panel",
            div{class:"panel-head",
                div{h2{"Informations du club"}
                    p{"Identité, présentation et visibilité publique du club."}}
            }
            form{class:"workspace-form club-information-form",
                onsubmit:{
                    let token=token.clone();let id=club.id.clone();
                    move |event|{
                        event.prevent_default();
                        if *saving.read()||*uploading.read(){return;}
                        let label=name.read().trim().to_string();
                        let text=description.read().trim().to_string();
                        let image=logo.read().trim().to_string();
                        if label.is_empty(){on_error.call("Le nom du club est obligatoire.".into());return;}
                        let visible=*public.read();
                        let token=token.clone();let id=id.clone();
                        saving.set(true);
                        spawn(async move{
                            let result=org::update_club_information(
                                &token,&id,&label,&text,
                                if image.is_empty(){None}else{Some(&image)},visible
                            ).await;
                            match result{
                                Ok(())=>on_saved.call(()),
                                Err(error)=>on_error.call(error),
                            }
                            saving.set(false);
                        });
                    }
                },
                div{class:"club-information-identity",
                    div{class:"club-logo-preview",
                        if let Some(url)=preview{
                            img{src:"{url}",alt:"Logo de {club.name}"}
                        }else{
                            span{"{club.name.chars().next().unwrap_or('C').to_uppercase()}"}
                        }
                    }
                    div{class:"club-logo-controls",
                        label{"Logo du club"}
                        input{r#type:"url",placeholder:"https://exemple.fr/logo.png",
                            value:if logo.read().starts_with("https://"){"{logo}"}else{""},
                            oninput:move |e|logo.set(e.value())}
                        input{r#type:"file",accept:".png,.jpg,.jpeg,.webp,.gif,image/png,image/jpeg,image/webp,image/gif",
                            disabled:*uploading.read(),
                            onchange:{
                                let token=token.clone();
                                move |event|{
                                    let token=token.clone();
                                    async move{
                                        for file in event.files(){
                                            uploading.set(true);
                                            let result=match file.read_bytes().await{
                                                Ok(bytes)=>catalog::upload(&token,bytes.to_vec()).await,
                                                Err(error)=>Err(format!("Lecture impossible : {error}")),
                                            };
                                            match result{
                                                Ok(image)=>logo.set(image),
                                                Err(error)=>on_error.call(error),
                                            }
                                            uploading.set(false);
                                        }
                                    }
                                }
                            }
                        }
                        if *uploading.read(){small{"Téléversement…"}}
                        small{"PNG, JPEG, WebP ou GIF, 4 Mo maximum."}
                        if !logo.read().is_empty(){
                            button{class:"button quiet small",r#type:"button",
                                onclick:move |_|logo.set(String::new()),"Retirer le logo"}
                        }
                    }
                }
                div{class:"field",label{"Nom du club"},
                    input{required:true,maxlength:"120",value:"{name}",
                        oninput:move |e|name.set(e.value())}}
                div{class:"field",label{"Description"},
                    textarea{rows:"4",maxlength:"3000",
                        placeholder:"Présentez votre club, ses activités et son identité.",
                        value:"{description}",oninput:move |e|description.set(e.value())}}
                div{class:"club-public-setting",
                    div{strong{"Visible dans l'annuaire"}
                        p{"Seuls le nom, le logo, la description et les équipes seront publics. Les joueuses et données sportives restent privées."}}
                    input{r#type:"checkbox",checked:*public.read(),
                        "aria-label":"Rendre ce club public",
                        onchange:move |e|public.set(e.checked())}
                }
                div{class:"club-information-actions",
                    button{class:"button solid",r#type:"submit",
                        disabled:*saving.read()||*uploading.read(),
                        if *saving.read(){"Enregistrement…"}else{"Enregistrer les informations"}}
                }
            }
        }
    }
}
