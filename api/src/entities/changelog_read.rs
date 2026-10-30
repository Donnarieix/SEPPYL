use sea_orm::entity::prelude::*;
use sea_orm::DeriveEntityModel;

#[sea_orm::model]
#[derive(Clone,Debug,DeriveEntityModel)]
#[sea_orm(table_name="changelog_reads")]
pub struct Model{
    #[sea_orm(primary_key,auto_increment=false)]
    pub user_id:Uuid,
    #[sea_orm(primary_key,auto_increment=false)]
    pub version:String,
    pub read_at:i64,
}
impl ActiveModelBehavior for ActiveModel {}
