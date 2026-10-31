use dioxus::prelude::*;
use crate::api::organization as org;
use super::logo_picker::{LogoPicker,LogoSelection,image_style};

/// Reused in the operational "Mon club" and independent global administration.
/// Stores only safe text and an approved HTTPS or uploaded image URL.
#[component]
pub fn ClubInformationEditor(
    token:String,club:org::Club,on_saved:EventHandler<()>,on_error:EventHandler<String>,
)->Element{
    let mut name=use_signal(||club.name.clone());
    let mut description=use_signal(||club.description.clone().unwrap_or_default());
    let mut logo=use_signal(||club.logo_url.clone().unwrap_or_default());
    let mut logo_x=use_signal(||club.logo_x.unwrap_or(0));
    let mut logo_y=use_signal(||club.logo_y.unwrap_or(0));
    let mut logo_zoom=use_signal(||club.logo_zoom.unwrap_or(100));
    let mut public=use_signal(||club.is_public==Some(true));
    let mut public_teams=use_signal(||club.show_public_teams==Some(true));
    let mut public_roster=use_signal(||club.show_public_roster==Some(true));
    let mut saving=use_signal(||false);

    let preview=org::club_logo_src(Some(logo.read().as_str()));
    let crop_style=image_style(*logo_x.read(),*logo_y.read(),*logo_zoom.read());
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
                        if *saving.read(){return;}
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
                                if image.is_empty(){None}else{Some(&image)},visible,
                                *logo_x.read(),*logo_y.read(),*logo_zoom.read(),
                                *public_teams.read(),*public_roster.read()
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
                        if let Some(src)=preview{
                            img{src:"{src}",alt:"Logo de {club.name}",style:"{crop_style}"}
                        }else{
                            span{"{club.name.chars().next().unwrap_or('C').to_uppercase()}"}
                        }
                    }
                    LogoPicker{
                        key:"{club.id}",token:token.clone(),
                        logo:logo.read().clone(),crop_x:*logo_x.read(),
                        crop_y:*logo_y.read(),crop_zoom:*logo_zoom.read(),
                        on_change:move |value:LogoSelection|{
                            logo.set(value.url);logo_x.set(value.x);logo_y.set(value.y);
                            logo_zoom.set(value.zoom);
                        },
                        on_error:move |error|on_error.call(error)
                    }
                }
                div{class:"field",label{"Nom du club"},
                    input{required:true,maxlength:"120",value:"{name}",
                        oninput:move |e|name.set(e.value())}}
                div{class:"field",label{"Description"},
                    textarea{rows:"4",maxlength:"3000",
                        placeholder:"Présentez votre club, ses activités et son identité.",
                        value:"{description}",oninput:move |e|description.set(e.value())}}
                label{class:"club-public-setting",
                    span{class:"club-public-setting-copy",
                        strong{"Visible dans l'annuaire"}
                        span{"La publication du club ne rend pas ses équipes ou ses effectifs publics automatiquement."}
                    }
                    input{r#type:"checkbox",checked:*public.read(),
                        "aria-label":"Rendre ce club public",
                        onchange:move |e|public.set(e.checked())}
                }
                label{class:"club-public-setting",
                    span{class:"club-public-setting-copy",
                        strong{"Afficher publiquement les équipes"}
                        span{"Noms, logos et descriptions des équipes."}
                    }
                    input{r#type:"checkbox",checked:*public_teams.read(),disabled:!*public.read(),
                        onchange:move |e|public_teams.set(e.checked())}
                }
                label{class:"club-public-setting",
                    span{class:"club-public-setting-copy",
                        strong{"Publier les effectifs"}
                        span{"Les noms et numéros des joueuses seront publics. Désactivé par défaut."}
                    }
                    input{r#type:"checkbox",checked:*public_roster.read(),
                        disabled:!*public.read()||!*public_teams.read(),
                        onchange:move |e|public_roster.set(e.checked())}
                }
                div{class:"club-information-actions",
                    button{class:"button solid",r#type:"submit",
                        disabled:*saving.read(),
                        if *saving.read(){"Enregistrement…"}else{"Enregistrer les informations"}}
                }
            }
        }
    }
}
