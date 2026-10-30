use sea_orm::entity::prelude::*;
use sea_orm::DeriveEntityModel;

#[sea_orm::model]
#[derive(Clone,Debug,DeriveEntityModel)]
#[sea_orm(table_name="changelog_entries")]
pub struct Model{
    #[sea_orm(primary_key,auto_increment=false)]
    pub version:String,
    pub title:String,
    pub body:String,
    pub published_at:i64,
    pub updated_at:i64,
}
impl ActiveModelBehavior for ActiveModel {}
