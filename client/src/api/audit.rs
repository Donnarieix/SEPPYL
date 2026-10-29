use reqwest::Client;
use serde::Deserialize;
fn base()->&'static str{option_env!("SEPPYL_API_URL").unwrap_or("http://127.0.0.1:3000")}
#[derive(Clone,Debug,PartialEq,Deserialize)]
pub struct Event{
    pub id:String,pub author:String,pub action:String,
    pub description:String,pub status:i32,pub status_label:String,pub timestamp:i64,
}
#[derive(Clone,Debug,PartialEq,Deserialize)]
pub struct AuditPage{
    pub items:Vec<Event>,pub total:u64,pub page:u64,
    pub page_size:u64,pub total_pages:u64,
}
pub async fn list(token:&str,page:u64,page_size:u64)->Result<AuditPage,String>{
    let r=Client::new().get(format!("{}/admin/audit?page={page}&page_size={page_size}",base()))
        .bearer_auth(token)
        .send().await.map_err(|e|e.to_string())?;
    if !r.status().is_success(){return Err(format!("Accès au journal impossible (HTTP {}).",r.status()));}
    r.json::<AuditPage>().await.map_err(|e|e.to_string())
}
