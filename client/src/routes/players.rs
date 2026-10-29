use dioxus::prelude::*;
use crate::api::players::{self,Player};
use super::{Route,layout::Header};

#[component]
pub fn Players() -> Element {
    let session=use_context::<Signal<Option<String>>>();
    let nav=use_navigator();
    let token=session.read().clone();
    let fetch=token.clone();
    let mut data=use_resource(move||{let token=fetch.clone();async move {
        match token {Some(t)=>players::list(&t).await,None=>Err("Connexion requise.".into())}
    }});
    let mut adding=use_signal(||false);
    let mut error=use_signal(||None::<String>);
    if token.is_none(){return rsx!{div{class:"centered",p{"Connectez-vous."}
        button{class:"button solid",onclick:move |_|{nav.push(Route::Login{});},"Connexion"}}};}
    let token=token.unwrap_or_default();
    rsx!{
        div{class:"app",
            Header{}
            main{class:"page",
                div{class:"page-heading",
                    div{h1{"Joueuses"} p{"Gérez l'effectif et les numéros de maillot."}}
                    button{class:"button solid",onclick:move |_|adding.set(true),"+ Ajouter une joueuse"}
                }
                if let Some(message)=error.read().as_ref(){p{class:"form-error",role:"alert","{message}"}}
                div{class:"sheet players-sheet",
                    div{class:"sheet-toolbar",strong{"Effectif"}}
                    match data.read().clone(){
                        Some(Ok(list))=>rsx!{
                            if list.is_empty(){
                                div{class:"empty-state",p{"Aucune joueuse pour le moment."}}
                            }else{
                                table{class:"roster-table",
                                    thead{tr{th{"Prénom"} th{"Nom"} th{"Numéro"} th{"Actions"}}}
                                    tbody{
                                        for player in list{
                                            PlayerRow{key:"{player.id}",player,token:token.clone(),
                                                on_saved:move |_|data.restart(),
                                                on_error:move |message|error.set(Some(message))}
                                        }
                                    }
                                }
                            }
                        },
                        Some(Err(e))=>rsx!{div{class:"load-state",p{"{e}"} button{class:"button outline",onclick:move |_|data.restart(),"Réessayer"}}},
                        None=>rsx!{div{class:"load-state","Chargement…"}},
                    }
                }
            }
            if *adding.read(){
                PlayerEditor{token:token.clone(),player:None,
                    on_cancel:move |_|adding.set(false),
                    on_saved:move |_|{adding.set(false);data.restart();},
                    on_error:move |message|error.set(Some(message))}
            }
        }
    }
}

#[component]
fn PlayerRow(player:Player,token:String,on_saved:EventHandler<()>,on_error:EventHandler<String>)->Element{
    let mut editing=use_signal(||false);
    let mut confirming=use_signal(||false);
    let mut busy=use_signal(||false);
    rsx!{
        tr{
            td{"{player.firstname}"}
            td{"{player.lastname}"}
            td{"{player.number}"}
            td{class:"roster-actions",
                button{class:"button outline small",onclick:move |_|editing.set(true),"Modifier"}
                if *confirming.read(){
                    button{class:"button danger small",disabled:*busy.read(),
                        onclick:{
                            let token=token.clone();let id=player.id.clone();
                            move |_|{
                                let token=token.clone();let id=id.clone();
                                busy.set(true);
                                spawn(async move{
                                    match players::delete(&token,&id).await{
                                        Ok(())=>on_saved.call(()),
                                        Err(e)=>on_error.call(e),
                                    }
                                    busy.set(false);confirming.set(false);
                                });
                            }
                        },
                        "Confirmer"
                    }
                    button{class:"button quiet small",onclick:move |_|confirming.set(false),"Annuler"}
                }else{
                    button{class:"button quiet small",onclick:move |_|confirming.set(true),"Supprimer"}
                }
            }
        }
        if *editing.read(){
            PlayerEditor{token:token.clone(),player:Some(player.clone()),
                on_cancel:move |_|editing.set(false),
                on_saved:move |_|{editing.set(false);on_saved.call(());},
                on_error}
        }
    }
}

#[component]
fn PlayerEditor(
    token:String,player:Option<Player>,
    on_cancel:EventHandler<()>,on_saved:EventHandler<()>,on_error:EventHandler<String>
)->Element{
    let mut firstname=use_signal(||player.as_ref().map(|p|p.firstname.clone()).unwrap_or_default());
    let mut lastname=use_signal(||player.as_ref().map(|p|p.lastname.clone()).unwrap_or_default());
    let mut number=use_signal(||player.as_ref().map(|p|p.number.to_string()).unwrap_or_default());
    let mut busy=use_signal(||false);
    let mut error=use_signal(||None::<String>);
    let editing=player.is_some();
    let id=player.as_ref().map(|p|p.id.clone());
    rsx!{
        div{class:"dialog-backdrop",
            div{class:"dialog-panel",role:"dialog","aria-modal":"true",
                div{class:"dialog-header",div{h2{if editing{"Modifier une joueuse"}else{"Ajouter une joueuse"}}}
                    button{class:"icon-button",onclick:move |_|on_cancel.call(()),"×"}
                }
                form{
                    onsubmit:move |e|{
                        e.prevent_default();
                        if *busy.read(){return;}
                        let first=firstname.read().trim().to_string();
                        let last=lastname.read().trim().to_string();
                        let parsed=number.read().parse::<u32>();
                        if first.is_empty()||last.is_empty()||parsed.is_err(){
                            error.set(Some("Renseignez un prénom, un nom et un numéro valide.".into()));return;
                        }
                        let token=token.clone();let id=id.clone();let num=parsed.unwrap();
                        busy.set(true);
                        spawn(async move{
                            match players::save(&token,id.as_deref(),&first,&last,num).await{
                                Ok(())=>on_saved.call(()),
                                Err(e)=>{error.set(Some(e.clone()));on_error.call(e);}
                            }
                            busy.set(false);
                        });
                    },
                    div{class:"dialog-body",
                        div{class:"field-grid",
                            div{class:"field",label{"Prénom"}input{value:"{firstname}",required:true,oninput:move |e|firstname.set(e.value())}}
                            div{class:"field",label{"Nom"}input{value:"{lastname}",required:true,oninput:move |e|lastname.set(e.value())}}
                        }
                        div{class:"field",label{"Numéro"}
                            input{r#type:"number",min:"0",max:"9999",value:"{number}",required:true,oninput:move |e|number.set(e.value())}
                        }
                        if let Some(message)=error.read().as_ref(){p{class:"form-error",role:"alert","{message}"}}
                    }
                    div{class:"dialog-footer",
                        button{class:"button quiet",r#type:"button",onclick:move |_|on_cancel.call(()),"Annuler"}
                        button{class:"button solid",r#type:"submit",disabled:*busy.read(),
                            if *busy.read(){"Enregistrement…"}else{"Enregistrer"}}
                    }
                }
            }
        }
    }
}
