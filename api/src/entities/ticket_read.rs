//! Per-account ticket read receipts. Message-count cursors avoid timestamp collisions.
use sea_orm::entity::prelude::*;
use sea_orm::DeriveEntityModel;
#[sea_orm::model]
#[derive(Clone,Debug,DeriveEntityModel)]
#[sea_orm(table_name="support_ticket_reads")]
pub struct Model {
    #[sea_orm(primary_key,auto_increment=false)]
    pub user_id:Uuid,
    #[sea_orm(primary_key,auto_increment=false)]
    pub ticket_id:Uuid,
    pub seen_count:i64,
}
impl ActiveModelBehavior for ActiveModel {}
