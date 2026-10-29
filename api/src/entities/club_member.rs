use sea_orm::entity::prelude::*;
use sea_orm::DeriveEntityModel;
use serde::Serialize;
#[sea_orm::model]
#[derive(Clone, Debug, DeriveEntityModel, Serialize)]
#[sea_orm(table_name="club_members")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub club_id: Uuid,
    #[sea_orm(primary_key)]
    pub user_id: Uuid,
    pub role: String,
}
impl ActiveModelBehavior for ActiveModel {}
