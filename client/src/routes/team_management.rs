use dioxus::prelude::*;
use crate::api::organization as org;

#[component]
pub fn TeamManagerRow(team:org::Team,token:String,on_saved:EventHandler<()>,
    on_error:EventHandler<String>)->Element{
    let mut editing=use_signal(||false);
    let mut confirming=use_signal(||false);
    let mut label=use_signal(||team.name.clone());
    let mut busy=use_signal(||false);
    rsx!{
        div{class:"workspace-roster-item team-manage-row",
            span{class:"workspace-team-icon","⚽"}
            if *editing.read(){
                form{class:"team-rename-form",
                    onsubmit:{
                        let token=token.clone();
                        let id=team.id.clone();
                        move |e|{
                            e.prevent_default();
                            let value=label.read().trim().to_string();
                            if value.is_empty()||*busy.read(){return;}
                            let token=token.clone();
                            let id=id.clone();
                            busy.set(true);
                            spawn(async move{
                                match org::rename_team(&token,&id,&value).await{
                                    Ok(())=>{editing.set(false);on_saved.call(());},
                                    Err(error)=>on_error.call(error),
                                }
                                busy.set(false);
                            });
                        }
                    },
                    input{value:"{label}",required:true,maxlength:"120",
                        "aria-label":"Nouveau nom de l'équipe",
                        oninput:move |e|label.set(e.value())}
                    button{class:"button solid small",r#type:"submit",disabled:*busy.read(),"Enregistrer"}
                    button{class:"button quiet small",r#type:"button",onclick:move |_|editing.set(false),"Annuler"}
                }
            }else{
                strong{"{team.name}"}
                if *confirming.read(){
                    span{class:"muted-small","Supprimer cette équipe ?"}
                    button{class:"button danger small",disabled:*busy.read(),
                        onclick:{
                            let token=token.clone();
                            let id=team.id.clone();
                            move |_|{
                                let token=token.clone();
                                let id=id.clone();
                                busy.set(true);
                                spawn(async move{
                                    match org::delete_team(&token,&id).await{
                                        Ok(())=>on_saved.call(()),
                                        Err(error)=>on_error.call(error),
                                    }
                                    busy.set(false);
                                    confirming.set(false);
                                });
                            }
                        },"Confirmer"
                    }
                    button{class:"button quiet small",onclick:move |_|confirming.set(false),"Annuler"}
                }else{
                    button{class:"button outline small",onclick:move |_|editing.set(true),"Renommer"}
                    button{class:"button quiet small",onclick:move |_|confirming.set(true),"Supprimer"}
                }
            }
        }
    }
}

#[component]
pub fn CreateTeamPlayer(token:String,team_id:String,on_saved:EventHandler<()>,
    on_error:EventHandler<String>)->Element{
    let mut firstname=use_signal(String::new);
    let mut lastname=use_signal(String::new);
    let mut number=use_signal(||0_u32);
    let mut busy=use_signal(||false);
    rsx!{
        form{class:"workspace-form roster-create-form",
            onsubmit:{
                let token=token.clone();
                let team_id=team_id.clone();
                move |e|{
                    e.prevent_default();
                    let first=firstname.read().trim().to_string();
                    let last=lastname.read().trim().to_string();
                    let num=*number.read();
                    if first.is_empty()||last.is_empty()||*busy.read(){return;}
                    let token=token.clone();let team_id=team_id.clone();
                    busy.set(true);
                    spawn(async move{
                        match org::create_team_player(&token,&team_id,&first,&last,num).await{
                            Ok(())=>{
                                firstname.set(String::new());
                                lastname.set(String::new());
                                number.set(0);
                                on_saved.call(());
                            }
                            Err(error)=>on_error.call(error),
                        }
                        busy.set(false);
                    });
                }
            },
            div{class:"field",label{"Prénom"},
                input{value:"{firstname}",required:true,maxlength:"100",
                    placeholder:"Prénom",oninput:move |e|firstname.set(e.value())}}
            div{class:"field",label{"Nom"},
                input{value:"{lastname}",required:true,maxlength:"100",
                    placeholder:"Nom",oninput:move |e|lastname.set(e.value())}}
            div{class:"field",label{"N° de maillot"},
                input{r#type:"number",min:"0",max:"999",value:"{number}",
                    oninput:move |e|if let Ok(n)=e.value().parse::<u32>(){number.set(n)}}}
            button{class:"button solid",r#type:"submit",disabled:*busy.read(),
                if *busy.read(){"Enregistrement…"}else{"Créer dans l’équipe"}}
        }
    }
}
