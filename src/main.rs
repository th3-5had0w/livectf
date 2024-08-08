use std::sync::mpsc::{self, Receiver, Sender};

use actix_web::{App, HttpServer, web};
use notifier::{Notifier, NotifierCommInfo, NotifierComms};
use actix_files;

mod challenge_upload_handler;
mod deployer;
mod database;
mod web_interface;
mod flag_receiver;
mod timer;
mod notifier;

// ANYTHING RELATED TO NOTIFIER SHOULD BE CRITICAL, ABORT!!!

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let (slave_sender, listen_master): (Sender<(String, Vec<u8>)>, Receiver<(String, Vec<u8>)>) = mpsc::channel();
    let mut slaves = NotifierComms {
        comm_infos: Vec::new()
    };

    let mut notifier = Notifier {
        slaves: slaves,
        listen_master: listen_master
    };

    let db_conn = database::new_db_connection().await.expect("can't open connection to database");

    challenge_upload_handler::init(&mut notifier, slave_sender.clone(), db_conn.clone());
    deployer::init(&mut notifier, slave_sender.clone(), db_conn.clone());
    flag_receiver::init(&mut notifier, slave_sender.clone(), db_conn.clone());
    timer::init(&mut notifier, slave_sender.clone(), db_conn.clone());
    // database::init(&mut notifier, slave_sender.clone());

    slaves = notifier.slaves.clone();

    tokio::spawn(async move {
        notifier.run();
    });
    return webserver_loop(slaves, db_conn).await;
}

async fn webserver_loop(slaves: NotifierComms, db_conn: database::DbConnection) -> std::io::Result<()> {

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(slaves.clone()))
            .app_data(web::Data::new(db_conn.do_clone()))
            .service(actix_files::Files::new("/static", "./static"))
            .route("/", web::get().to(web_interface::index))
            .route("/login", web::get().to(web_interface::login))
            .route("/register", web::get().to(web_interface::register))
            .route("/sheep_center", web::get().to(web_interface::admin_index))
            .route("/api/login", web::post().to(web_interface::user::api_user_login))
            .route("/api/register", web::post().to(web_interface::user::api_user_register))
            .route("/api/user/create", web::post().to(web_interface::user::api_user_create))
            .route("/api/user/edit", web::post().to(web_interface::user::api_user_edit))
            .route("/api/user/{user_id}", web::get().to(web_interface::user::api_get_user))
            .route("/api/user/{user_id}", web::delete().to(web_interface::user::api_delete_user))
            .route("/api/user/search", web::get().to(web_interface::user::api_filter_user))
            .route("/upload", web::post().to(challenge_upload_handler::handle_challenge))
            .route("/submit/{challenge}/{flag}", web::post().to(flag_receiver::handle_submission))
            .default_service(
                web::route().to(web_interface::not_found)
            )
    })
    .bind("127.0.0.1:31337")?
    .run()
    .await

}
