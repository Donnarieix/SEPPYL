use dioxus::prelude::*;
use crate::api::organization as org;
use super::logo_picker::{LogoPicker,LogoSelection,image_style};

#[component]
pub fn TeamInformationEditor(
    token:String,team:org::Team,on_saved:EventHandler<()>,on_error:EventHandler<String>,
)->Element {
    let mut name=use_signal(||team.name.clone());
    let mut description=use_signal(||team.description.clone().unwrap_or_default());
    let mut logo=use_signal(||team.logo_url.clone().unwrap_or_default());
    let mut x=use_signal(||team.logo_x.unwrap_or(0));
    let mut y=use_signal(||team.logo_y.unwrap_or(0));
    let mut zoom=use_signal(||team.logo_zoom.unwrap_or(100));
    let mut busy=use_signal(||false);
    let preview=org::club_logo_src(Some(logo.read().as_str()));
    let style=image_style(*x.read(),*y.read(),*zoom.read());
    rsx! {
        section{class:"panel club-information-panel",
            div{class:"panel-head",
                div{h2{"Informations de l'équipe"}
                    p{"Nom, présentation et identité visuelle de cette équipe."}}
            }
            form{class:"club-information-form",
                onsubmit:{
                    let token=token.clone();let id=team.id.clone();
                    move |e|{
                        e.prevent_default();
                        if *busy.read(){return;}
                        let label=name.read().trim().to_string();
                        if label.is_empty(){on_error.call("Le nom de l'équipe est obligatoire.".into());return;}
                        let description=description.read().clone();
                        let logo=logo.read().clone();
                        let token=token.clone();let id=id.clone();
                        let px=*x.read();let py=*y.read();let pzoom=*zoom.read();
                        busy.set(true);
                        spawn(async move{
                            let result=org::update_team_information(
                                &token,&id,&label,&description,
                                if logo.is_empty(){None}else{Some(&logo)},
                                px,py,pzoom
                            ).await;
                            match result{
                                Ok(())=>on_saved.call(()),
                                Err(error)=>on_error.call(error)
                            }
                            busy.set(false);
                        });
                    }
                },
                div{class:"club-information-identity",
                    div{class:"club-logo-preview",
                        if let Some(src)=preview{
                            img{src:"{src}",alt:"Logo de {team.name}",style:"{style}"}
                        }else{
                            span{"⚽"}
                        }
                    }
                    LogoPicker{
                        key:"{team.id}",token:token.clone(),
                        logo:logo.read().clone(),crop_x:*x.read(),crop_y:*y.read(),
                        crop_zoom:*zoom.read(),
                        on_change:move |selection:LogoSelection|{
                            logo.set(selection.url);
                            x.set(selection.x);y.set(selection.y);zoom.set(selection.zoom);
                        },
                        on_error:move |error|on_error.call(error)
                    }
                }
                div{class:"field",label{"Nom de l'équipe"},
                    input{required:true,maxlength:"120",value:"{name}",
                        oninput:move |e|name.set(e.value())}}
                div{class:"field",label{"Description"},
                    textarea{rows:"4",maxlength:"3000",value:"{description}",
                        placeholder:"Objectifs, catégories et informations sur l'équipe…",
                        oninput:move |e|description.set(e.value())}}
                div{class:"club-information-actions",
                    button{class:"button solid",r#type:"submit",disabled:*busy.read(),
                        if *busy.read(){"Enregistrement…"}else{"Enregistrer les informations"}}
                }
            }
        }
    }
}
