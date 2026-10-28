use sea_orm::entity::prelude::*;
use sea_orm::DeriveEntityModel;

#[sea_orm::model]
#[derive(Clone, Debug, DeriveEntityModel)]
#[sea_orm(table_name = "catalog_selections")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub user_id: Uuid,
    #[sea_orm(primary_key)]
    pub activity_id: Uuid,
}
impl ActiveModelBehavior for ActiveModel {}
