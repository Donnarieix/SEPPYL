use sea_orm::entity::prelude::*;
use sea_orm::DeriveEntityModel;
#[sea_orm::model]
#[derive(Clone,Debug,DeriveEntityModel)]
#[sea_orm(table_name="club_players")]
pub struct Model{
    #[sea_orm(primary_key)]
    pub club_id:Uuid,
    #[sea_orm(primary_key)]
    pub player_id:Uuid,
}
impl ActiveModelBehavior for ActiveModel {}
