//! One-time, transactional migration for the original SQLite player table.
//! Older versions stored firstname UNIQUE inline, backed by sqlite_autoindex.
//! SQLite cannot DROP INDEX for this inline constraint: recreate the table while
//! preserving its primary keys and all external references.
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DbBackend, DbErr, QueryResult, Statement,
    TransactionTrait,
};

fn raw(sql:impl Into<String>)->Statement{
    Statement::from_string(DbBackend::Sqlite,sql.into())
}
fn missing(message:&str)->DbErr {DbErr::Custom(message.to_string())}
async fn query_rows(db:&impl ConnectionTrait,sql:impl Into<String>)->Result<Vec<QueryResult>,DbErr>{
    db.query_all_raw(raw(sql)).await
}
async fn integer(db:&impl ConnectionTrait,sql:&str,column:&str)->Result<i64,DbErr>{
    db.query_one_raw(raw(sql)).await?
        .ok_or_else(||missing("Résultat SQLite manquant"))?
        .try_get("",column)
}
fn quote_identifier(value:&str)->String{
    format!("\"{}\"",value.replace('"',"\"\""))
}

pub async fn migrate_legacy_player_firstname(
    db:&DatabaseConnection,backup_path:Option<&str>
)->Result<bool,DbErr>{
    let indexes=query_rows(db,"PRAGMA index_list('players')").await?;
    let mut inline_unique=false;
    let mut named_unique=Vec::<String>::new();
    for row in indexes{
        let unique:i64=row.try_get("","unique")?;
        if unique!=1 {continue;}
        let index:String=row.try_get("","name")?;
        let origin:String=row.try_get("","origin")?;
        let info=query_rows(db,format!("PRAGMA index_info('{}')",index.replace('\'',"''"))).await?;
        let cols=info.iter().map(|r|r.try_get::<Option<String>>("","name"))
            .collect::<Result<Vec<_>,_>>()?;
        if cols!=vec![Some("firstname".into())]{continue;}
        match origin.as_str(){
            "u"=>inline_unique=true,
            "c"=>named_unique.push(index),
            _=>{},
        }
    }
    if !inline_unique {
        // The normal SeaORM index may be removed without rebuilding the table.
        for index in &named_unique{
            db.execute_unprepared(&format!("DROP INDEX IF EXISTS {}",quote_identifier(index)))
                .await?;
        }
        return Ok(!named_unique.is_empty());
    }

    let table_info=query_rows(db,"PRAGMA table_info('players')").await?;
    let names=table_info.iter().map(|r|r.try_get::<String>("","name"))
        .collect::<Result<Vec<_>,_>>()?;
    if names!=["id","firstname","lastname","number"]{
        return Err(missing("Migration annulée : la table players contient des colonnes inattendues"));
    }
    let mut definitions=Vec::new();
    for row in table_info{
        let name:String=row.try_get("","name")?;
        let data_type:String=row.try_get("","type")?;
        let notnull:i64=row.try_get("","notnull")?;
        let pk:i64=row.try_get("","pk")?;
        let default:Option<String>=row.try_get("","dflt_value")?;
        if default.is_some() || !data_type.chars().all(|c|
            c.is_ascii_alphanumeric()||matches!(c,'_'|'('|')'|' ')
        ){
            return Err(missing("Migration annulée : déclaration de colonne inattendue"));
        }
        let mut declaration=format!("{} {}",quote_identifier(&name),data_type);
        if notnull!=0 {declaration.push_str(" NOT NULL");}
        if pk!=0 {declaration.push_str(" PRIMARY KEY");}
        definitions.push(declaration);
    }
    let objects=query_rows(db,
        "SELECT type,name,sql FROM sqlite_master WHERE tbl_name='players' AND sql IS NOT NULL AND type IN ('index','trigger')"
    ).await?;
    let mut recreate=Vec::new();
    for obj in objects{
        let kind:String=obj.try_get("","type")?;
        let name:String=obj.try_get("","name")?;
        let sql:String=obj.try_get("","sql")?;
        if kind=="index" && named_unique.iter().any(|i|i==&name){continue;}
        recreate.push(sql);
    }

    // VACUUM INTO provides a consistent SQLite snapshot (including WAL data).
    // Keep the first snapshot for manual rollback; never overwrite it.
    if let Some(path)=backup_path{
        if !std::path::Path::new(path).exists(){
            db.execute_unprepared(&format!(
                "VACUUM INTO '{}'",path.replace("'", "''")
            )).await?;
            tracing::info!(path=%path,"Created pre-migration SQLite backup");
        }
    }

    // Use a dedicated single-connection pool: PRAGMA foreign_keys is
    // connection-local and must be switched off BEFORE BEGIN.
    let previous_fk=integer(db,"PRAGMA foreign_keys","foreign_keys").await?;
    db.execute_unprepared("PRAGMA foreign_keys=OFF").await?;
    let attempt=async{
        let txn=db.begin().await?;
        let result=async{
            let before=integer(&txn,"SELECT COUNT(*) AS count FROM players","count").await?;
            txn.execute_unprepared(&format!(
                "CREATE TABLE \"players__seppyl_upgrade\" ({})",definitions.join(", ")
            )).await?;
            txn.execute_unprepared(
                "INSERT INTO players__seppyl_upgrade (id,firstname,lastname,number) SELECT id,firstname,lastname,number FROM players"
            ).await?;
            txn.execute_unprepared("DROP TABLE players").await?;
            txn.execute_unprepared(
                "ALTER TABLE players__seppyl_upgrade RENAME TO players"
            ).await?;
            for stmt in &recreate{
                txn.execute_unprepared(stmt).await?;
            }
            let after=integer(&txn,"SELECT COUNT(*) AS count FROM players","count").await?;
            if before!=after {return Err(missing("Migration annulée : nombre de joueuses modifié"));}
            if !query_rows(&txn,"PRAGMA foreign_key_check").await?.is_empty(){
                return Err(missing("Migration annulée : références SQLite invalides"));
            }
            Ok::<_,DbErr>(())
        }.await;
        match result{
            Ok(())=>txn.commit().await,
            Err(err)=>{txn.rollback().await?;Err(err)},
        }
    }.await;
    // Always restore the exact FK setting after commit or rollback.
    let restore=db.execute_unprepared(if previous_fk==0{
        "PRAGMA foreign_keys=OFF"
    }else{"PRAGMA foreign_keys=ON"}).await;
    restore?;
    attempt?;
    tracing::info!("Migrated legacy players table; retained player IDs and references");
    Ok(true)
}


/// Add club information without rewriting clubs, memberships or foreign keys.
/// Works for existing SQLite databases; new installations use the model schema.
pub async fn add_club_information(db:&DatabaseConnection)->Result<(),DbErr>{
    let columns=query_rows(db,"PRAGMA table_info('clubs')").await?;
    if columns.is_empty(){return Ok(());}
    let names=columns.iter()
        .map(|column|column.try_get::<String>("","name"))
        .collect::<Result<Vec<_>,_>>()?;
    for (name,sql) in [
        ("description","ALTER TABLE clubs ADD COLUMN description TEXT"),
        ("logo_url","ALTER TABLE clubs ADD COLUMN logo_url TEXT"),
        ("logo_x","ALTER TABLE clubs ADD COLUMN logo_x INTEGER"),
        ("logo_y","ALTER TABLE clubs ADD COLUMN logo_y INTEGER"),
        ("logo_zoom","ALTER TABLE clubs ADD COLUMN logo_zoom INTEGER"),
    ]{
        if !names.iter().any(|column|column==name){
            db.execute_unprepared(sql).await?;
            tracing::info!(field=name,"Extended clubs with information field");
        }
    }
    Ok(())
}

pub async fn add_team_information(db:&DatabaseConnection)->Result<(),DbErr>{
    let columns=query_rows(db,"PRAGMA table_info('teams')").await?;
    if columns.is_empty(){return Ok(());}
    let names=columns.iter().map(|c|c.try_get::<String>("","name"))
        .collect::<Result<Vec<_>,_>>()?;
    for (name,sql) in [
        ("description","ALTER TABLE teams ADD COLUMN description TEXT"),
        ("logo_url","ALTER TABLE teams ADD COLUMN logo_url TEXT"),
        ("logo_x","ALTER TABLE teams ADD COLUMN logo_x INTEGER"),
        ("logo_y","ALTER TABLE teams ADD COLUMN logo_y INTEGER"),
        ("logo_zoom","ALTER TABLE teams ADD COLUMN logo_zoom INTEGER"),
    ] {
        if !names.iter().any(|n|n==name){
            db.execute_unprepared(sql).await?;
        }
    }
    Ok(())
}

pub async fn enforce_player_identity(db:&DatabaseConnection)->Result<(),DbErr>{
    // A migration must never arbitrarily merge existing player IDs.
    let duplicates=query_rows(db,
        "SELECT firstname,lastname,number,COUNT(*) AS total FROM players GROUP BY firstname,lastname,number HAVING COUNT(*) > 1 LIMIT 1"
    ).await?;
    if let Some(row)=duplicates.first(){
        let firstname:String=row.try_get("","firstname")?;
        let lastname:String=row.try_get("","lastname")?;
        let number:i64=row.try_get("","number")?;
        tracing::error!(%firstname,%lastname,number,
            "Duplicate complete player identity blocks unique-index migration");
        return Err(missing("Identité de joueuse déjà présente plusieurs fois ; aucune fiche n'a été fusionnée"));
    }
    db.execute_unprepared(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_players_complete_identity ON players (firstname, lastname, number)"
    ).await?;
    Ok(())
}

#[cfg(test)]
mod tests{
    use super::*;
    use sea_orm::{ConnectOptions,Database};

    #[tokio::test]
    async fn old_inline_unique_constraint_is_removed_without_losing_links(){
        let mut opt=ConnectOptions::new("sqlite::memory:");
        opt.max_connections(1);
        let db=Database::connect(opt).await.unwrap();
        db.execute_unprepared(
            "CREATE TABLE players (id uuid_text NOT NULL PRIMARY KEY, firstname varchar NOT NULL UNIQUE, lastname varchar NOT NULL, number integer NOT NULL)"
        ).await.unwrap();
        db.execute_unprepared(
            "CREATE TABLE player_refs (player_id uuid_text NOT NULL REFERENCES players(id))"
        ).await.unwrap();
        db.execute_unprepared(
            "INSERT INTO players VALUES ('test-1','Emma','Durand',7)"
        ).await.unwrap();
        db.execute_unprepared(
            "INSERT INTO player_refs VALUES ('test-1')"
        ).await.unwrap();
        assert!(migrate_legacy_player_firstname(&db,None).await.unwrap());
        assert!(!migrate_legacy_player_firstname(&db,None).await.unwrap());
        db.execute_unprepared(
            "INSERT INTO players VALUES ('test-2','Emma','Martin',9)"
        ).await.unwrap();
        assert_eq!(integer(&db,"SELECT COUNT(*) AS count FROM players","count").await.unwrap(),2);
        assert_eq!(integer(&db,"SELECT COUNT(*) AS count FROM player_refs","count").await.unwrap(),1);
        assert!(query_rows(&db,"PRAGMA foreign_key_check").await.unwrap().is_empty());
    }
    #[tokio::test]
    async fn complete_player_identity_is_unique_but_firstnames_can_repeat(){
        let mut opt=ConnectOptions::new("sqlite::memory:");
        opt.max_connections(1);
        let db=Database::connect(opt).await.unwrap();
        db.execute_unprepared(
            "CREATE TABLE players (id TEXT PRIMARY KEY, firstname TEXT NOT NULL, lastname TEXT NOT NULL, number INTEGER NOT NULL)"
        ).await.unwrap();
        enforce_player_identity(&db).await.unwrap();
        db.execute_unprepared("INSERT INTO players VALUES ('a','Lina','Petit',7)")
            .await.unwrap();
        db.execute_unprepared("INSERT INTO players VALUES ('b','Lina','Martin',7)")
            .await.unwrap();
        db.execute_unprepared("INSERT INTO players VALUES ('c','Lina','Petit',8)")
            .await.unwrap();
        assert!(db.execute_unprepared(
            "INSERT INTO players VALUES ('d','Lina','Petit',7)"
        ).await.is_err());
        db.execute_unprepared("UPDATE players SET lastname='Giraud' WHERE id='a'")
            .await.unwrap();
        db.execute_unprepared("INSERT INTO players VALUES ('d','Lina','Petit',7)")
            .await.unwrap();
    }
    #[tokio::test]
    async fn preexisting_duplicate_triples_stop_migration_without_merging(){
        let mut opt=ConnectOptions::new("sqlite::memory:");
        opt.max_connections(1);
        let db=Database::connect(opt).await.unwrap();
        db.execute_unprepared(
            "CREATE TABLE players (id TEXT PRIMARY KEY, firstname TEXT NOT NULL, lastname TEXT NOT NULL, number INTEGER NOT NULL)"
        ).await.unwrap();
        db.execute_unprepared(
            "INSERT INTO players VALUES ('a','Lina','Petit',7), ('b','Lina','Petit',7)"
        ).await.unwrap();
        assert!(enforce_player_identity(&db).await.is_err());
        assert_eq!(integer(&db,"SELECT COUNT(*) AS count FROM players","count")
            .await.unwrap(),2);
    }
}
