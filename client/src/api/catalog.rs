use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
pub fn api_base() -> &'static str {
    super::base_url()
}
pub fn image_src(url: &str) -> String {
    if url.starts_with('/') { format!("{}{}", api_base(), url) } else { url.to_owned() }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Material { pub id:String, pub activity_id:String, pub name:String, pub quantity:i32 }
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Activity {
    pub id: String, pub user_id:String, pub name:String, pub description:String,
    pub image_url:Option<String>, pub materials:Vec<Material>,
}
#[derive(Serialize)]
struct Payload<'a> { name:&'a str, description:&'a str, image_url:Option<&'a str> }
#[derive(Serialize)]
struct MaterialPayload<'a> { name:&'a str, quantity:i32 }
#[derive(Serialize)]
struct Selected<'a> { activity_id:&'a str }
#[derive(Deserialize)]
struct ImageResponse { image_url:String }

pub async fn check(response:reqwest::Response)->Result<(),String>{
    let status=response.status();
    match status {
        StatusCode::UNAUTHORIZED=>Err("Votre session a expiré. Reconnectez-vous.".into()),
        StatusCode::FORBIDDEN=>Err("Accès refusé.".into()),
        StatusCode::CONFLICT=>Err("Cette activité est déjà utilisée dans une séance et ne peut pas être supprimée.".into()),
        _ if status.is_success()=>Ok(()),
        _=>Err(format!("Erreur HTTP {status}")),
    }
}
pub async fn list(token:&str)->Result<Vec<Activity>,String> {
    let response=Client::new().get(format!("{}/catalog",api_base())).bearer_auth(token)
        .send().await.map_err(|e|e.to_string())?;
    if !response.status().is_success() {check(response).await?;return Err("Réponse inattendue.".into());}
    response.json().await.map_err(|e|e.to_string())
}
#[derive(Deserialize)]
struct CreatedActivity { id: String }

/// Return the canonical activity ID, including when creating from the spreadsheet.
pub async fn upsert(token:&str,id:Option<&str>,name:&str,description:&str,image_url:Option<&str>)->Result<String,String>{
    let client=Client::new();
    let url=match id {Some(id)=>format!("{}/catalog/{id}",api_base()),None=>format!("{}/catalog",api_base())};
    let request=if id.is_some() {client.put(url)} else {client.post(url)};
    let response=request.bearer_auth(token).json(&Payload{name,description,image_url})
        .send().await.map_err(|e|e.to_string())?;
    if !response.status().is_success() {
        check(response).await?;
        return Err("Réponse inattendue lors de la sauvegarde.".into());
    }
    match id {
        Some(existing)=>Ok(existing.to_owned()),
        None=>response.json::<CreatedActivity>().await.map(|created|created.id)
            .map_err(|e|format!("L'activité a été créée mais sa réponse est illisible : {e}")),
    }
}
pub async fn remove(token:&str,id:&str)->Result<(),String>{
    check(Client::new().delete(format!("{}/catalog/{id}",api_base())).bearer_auth(token)
        .send().await.map_err(|e|e.to_string())?).await
}
pub async fn add_material(token:&str,id:&str,name:&str,quantity:i32)->Result<(),String>{
    check(Client::new().post(format!("{}/catalog/{id}/materials",api_base())).bearer_auth(token)
        .json(&MaterialPayload{name,quantity}).send().await.map_err(|e|e.to_string())?).await
}
pub async fn remove_material(token:&str,id:&str,material_id:&str)->Result<(),String>{
    check(Client::new().delete(format!("{}/catalog/{id}/materials/{material_id}",api_base()))
        .bearer_auth(token).send().await.map_err(|e|e.to_string())?).await
}
pub async fn upload(token:&str,bytes:Vec<u8>)->Result<String,String>{
    if bytes.len()>4*1024*1024 {return Err("Image trop volumineuse (maximum 4 Mo).".into());}
    let response=Client::new().post(format!("{}/catalog/upload",api_base()))
        .bearer_auth(token).header("Content-Type","application/octet-stream")
        .body(bytes).send().await.map_err(|e|e.to_string())?;
    if !response.status().is_success(){check(response).await?;return Err("Image non reconnue.".into());}
    response.json::<ImageResponse>().await.map(|r|r.image_url).map_err(|e|e.to_string())
}
pub async fn select_for_table(token:&str,id:&str)->Result<(),String>{
    check(Client::new().post(format!("{}/matrix/select",api_base())).bearer_auth(token)
        .json(&Selected{activity_id:id}).send().await.map_err(|e|e.to_string())?).await
}
pub async fn attach_to_training(token:&str,training_id:&str,id:&str)->Result<(),String>{
    check(Client::new().post(format!("{}/trainings/{training_id}/catalog/{id}",api_base()))
        .bearer_auth(token).send().await.map_err(|e|e.to_string())?).await
}
