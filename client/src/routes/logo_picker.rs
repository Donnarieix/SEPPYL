use dioxus::prelude::*;
use crate::api::{catalog,organization as org};

#[derive(Clone,PartialEq,Debug)]
pub struct LogoSelection{
    pub url:String,
    pub x:i32,
    pub y:i32,
    pub zoom:i32,
}
pub fn image_style(x:i32,y:i32,zoom:i32)->String{
    // Render cropped imagery without reading third-party pixel data in a canvas.
    // Remote HTTPS imagery therefore works even without cross-origin canvas access.
    let z=zoom.clamp(100,300);
    format!("object-position:{}% {}%;transform:translate({:.1}%,{:.1}%) scale({});",
        50+x.clamp(-100,100)/2,50+y.clamp(-100,100)/2,
        x.clamp(-100,100) as f32*(z-100) as f32/200.0,
        y.clamp(-100,100) as f32*(z-100) as f32/200.0,
        z as f32 / 100.0)
}
#[component]
pub fn LogoPicker(
    token:String,
    logo:String,
    crop_x:i32,
    crop_y:i32,
    crop_zoom:i32,
    on_change:EventHandler<LogoSelection>,
    on_error:EventHandler<String>,
)->Element{
    let mut url=use_signal(||logo.clone());
    let mut x=use_signal(||crop_x);
    let mut y=use_signal(||crop_y);
    let mut zoom=use_signal(||crop_zoom);
    let mut editing=use_signal(||false);
    let mut uploading=use_signal(||false);
    let preview=org::club_logo_src(Some(url.read().as_str()));
    let style=image_style(*x.read(),*y.read(),*zoom.read());
    rsx!{
        div{class:"logo-picker",
            div{class:"club-logo-controls",
                label{"Image / logo"}
                input{class:"logo-link-input",r#type:"url",placeholder:"https://exemple.fr/logo.png",
                    value:if url.read().starts_with("https://"){"{url}"}else{""},
                    oninput:move |e|url.set(e.value()),
                    onchange:move |e|{
                        let value=e.value();
                        if value.starts_with("https://"){
                            url.set(value);x.set(0);y.set(0);zoom.set(100);editing.set(true);
                        }
                    }}
                div{class:"logo-picker-actions",
                    label{class:"button outline small logo-upload-button",
                        input{r#type:"file",accept:".png,.jpg,.jpeg,.webp,.gif,image/png,image/jpeg,image/webp,image/gif",
                            disabled:*uploading.read(),
                            onchange:{
                                let token=token.clone();
                                move |e|{
                                    let token=token.clone();
                                    async move{
                                        for file in e.files(){
                                            uploading.set(true);
                                            let result=match file.read_bytes().await{
                                                Ok(data)=>catalog::upload(&token,data.to_vec()).await,
                                                Err(error)=>Err(format!("Fichier illisible : {error}"))
                                            };
                                            match result{
                                                Ok(image)=>{
                                                    url.set(image);x.set(0);y.set(0);
                                                    zoom.set(100);editing.set(true);
                                                },
                                                Err(error)=>on_error.call(error)
                                            }
                                            uploading.set(false);
                                        }
                                    }
                                }
                            }
                        }
                        if *uploading.read(){"Importation…"}else{"Choisir une image"}
                    }
                    if preview.is_some(){
                        button{class:"button outline small",r#type:"button",
                            onclick:move |_|editing.set(true),"Ajuster le cadrage"}
                        button{class:"button quiet small",r#type:"button",
                            onclick:move |_|{
                                url.set(String::new());x.set(0);y.set(0);zoom.set(100);
                                on_change.call(LogoSelection{url:String::new(),x:0,y:0,zoom:100});
                            },"Retirer"}
                    }
                }
                small{"PNG, JPEG, WebP ou GIF · 4 Mo maximum. Cadrage centré par défaut."}
            }
            if *editing.read(){
                div{class:"logo-crop-backdrop",
                    div{class:"logo-crop-dialog",role:"dialog","aria-modal":"true",
                        "aria-label":"Cadrer l'image",
                        div{class:"logo-crop-heading",
                            div{h2{"Ajuster le cadrage"}
                                p{"Déplacez l'image dans le cadre ou utilisez les réglages."}}
                            button{class:"icon-button",r#type:"button",
                                "aria-label":"Fermer",onclick:move |_|editing.set(false),"×"}
                        }
                        div{class:"logo-crop-stage","data-logo-crop-stage":"true",
                            if let Some(src)=preview.clone(){
                                img{src:"{src}",alt:"Aperçu du cadrage",style:"{style}"}
                            }
                            div{class:"logo-crop-outline"}
                        }
                        div{class:"logo-crop-controls",
                            div{class:"field",label{"Déplacement horizontal"},
                                input{r#type:"range",min:"-100",max:"100",step:"1",
                                    "data-logo-crop-x":"true",value:"{x}",
                                    oninput:move |e|if let Ok(v)=e.value().parse::<i32>(){x.set(v)}}}
                            div{class:"field",label{"Déplacement vertical"},
                                input{r#type:"range",min:"-100",max:"100",step:"1",
                                    "data-logo-crop-y":"true",value:"{y}",
                                    oninput:move |e|if let Ok(v)=e.value().parse::<i32>(){y.set(v)}}}
                            div{class:"field",label{"Zoom — {zoom}%"},
                                input{r#type:"range",min:"100",max:"300",step:"5",
                                    "data-logo-crop-zoom":"true",value:"{zoom}",
                                    oninput:move |e|if let Ok(v)=e.value().parse::<i32>(){zoom.set(v)}}}
                        }
                        div{class:"logo-crop-footer",
                            button{class:"button quiet",r#type:"button",
                                onclick:move |_|{x.set(0);y.set(0);zoom.set(100);},"Recentrer"}
                            button{class:"button outline",r#type:"button",
                                onclick:move |_|editing.set(false),"Annuler"}
                            button{class:"button solid",r#type:"button",
                                onclick:move |_|{
                                    on_change.call(LogoSelection{
                                        url:url.read().clone(),x:*x.read(),y:*y.read(),zoom:*zoom.read()
                                    });
                                    editing.set(false);
                                },"Valider le cadrage"}
                        }
                    }
                }
            }
        }
    }
}
