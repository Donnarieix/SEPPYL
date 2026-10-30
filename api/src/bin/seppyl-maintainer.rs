//! Offline privileged role promotion. No HTTP endpoint can grant Maintainer.
//! Run this command locally on the server against the same SQLite database.
use sea_orm::{ConnectionTrait,Database,sea_query::{Query,Alias,Expr,ExprTrait}};
#[tokio::main]
async fn main(){
    if let Err(error)=run().await{
        eprintln!("Erreur : {error}");
        std::process::exit(1);
    }
}
async fn run()->Result<(),Box<dyn std::error::Error>>{
    let args:Vec<String>=std::env::args().skip(1).collect();
    if args.len()!=4||args[0]!="promote"||args[2]!="--db"
        ||args[1].trim().is_empty()||args[3].trim().is_empty(){
        return Err("Usage: seppyl-maintainer promote <identifiant> --db <chemin-base-sqlite>".into());
    }
    let username=&args[1];
    let file=&args[3];
    if !std::path::Path::new(file).is_file(){
        return Err("La base SQLite n'existe pas. Le chemin doit désigner une base existante ; aucune base ne sera créée.".into());
    }
    let uri=format!("sqlite://{}?mode=rw",file);
    let db=Database::connect(uri).await?;
    let users=Alias::new("users");
    let query=Query::select()
        .column(Alias::new("role")).from(users.clone())
        .and_where(Expr::col(Alias::new("username")).eq(username.as_str())).to_owned();
    let user=db.query_one(&query).await?;
    let Some(user)=user else{
        return Err("Aucun compte ne correspond à cet identifiant.".into());
    };
    let current:Option<String>=user.try_get("","role")?;
    if current.as_deref()==Some("maintainer"){
        println!("Ce compte est déjà Maintainer ; aucune modification.");
        return Ok(());
    }
    let update=Query::update()
        .table(users)
        .value(Alias::new("role"),"maintainer")
        .and_where(Expr::col(Alias::new("username")).eq(username.as_str())).to_owned();
    let result=db.execute(&update).await?;
    if result.rows_affected()!=1{
        return Err("Aucun compte modifié. Vérifiez les permissions sur la base.".into());
    }
    println!("Compte promu Maintainer : {username}");
    Ok(())
}
