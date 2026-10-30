use sea_orm::entity::prelude::*;
use sea_orm::DeriveEntityModel;
use serde::Serialize;

#[sea_orm::model]
#[derive(Clone, Debug, DeriveEntityModel, Serialize)]
#[sea_orm(table_name = "catalog_activities")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub user_id: Uuid,
    pub team_id: Option<Uuid>,
    pub name: String,
    pub description: String,
    /// Sanitized HTML, null for legacy plain-text descriptions.
    pub description_html: Option<String>,
    pub image_url: Option<String>,
}
impl ActiveModelBehavior for ActiveModel {}
