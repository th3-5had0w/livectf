use actix_web::{web, HttpResponse};
use actix_multipart::Multipart;
use futures_util::{StreamExt, TryStreamExt};
use uuid::Uuid;
use std::sync::mpsc::{self, Receiver, Sender};
use std::io::Write;
use std::fs::File;

use crate::notifier::{craft_type_notify_message, Notifier, NotifierCommInfo, NotifierComms};
use crate::database::DbConnection;

struct ChallengeUploadHandlerCtx {
    sender: Sender<(String, Vec<u8>)>,
    listener: Receiver<Vec<u8>>,
    
    db_conn: DbConnection
}

pub(crate) fn init(notifier: &mut Notifier, my_sender: Sender<(String, Vec<u8>)>, db_conn: DbConnection) {
    let (notifier_sender, my_receiver) : (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel();
    let ctx = ChallengeUploadHandlerCtx {
        sender: my_sender,
        listener: my_receiver,
        db_conn
    };


    let comm_info = NotifierCommInfo {
        id: Uuid::new_v4().as_u128(),
        name: "challenge_upload_handler".to_string(),
        broadcast_channel: notifier_sender
    };
    notifier.slaves.comm_infos.push(comm_info);


    
}

// remember to add a decrypt function because the author's challenge will be encrypted when uploading!
// I think the decrypt and processing part will be pass to deployer.
// I don't think we need decrypt, let's just finish this :|

pub(crate) async fn handle_challenge(slaves: web::Data<NotifierComms>, _: web::Data<DbConnection>, mut payload: Multipart) -> Result<HttpResponse, actix_web::Error> {
    while let Some(mut field) = payload.try_next().await? {
        let content_type = field.content_disposition().unwrap();
        if let Some(_) = content_type.get_filename() {

            let filename = Uuid::new_v4();
            let filepath = format!("./archives/{}.tar.gz", filename);

            let mut f = File::create(&filepath).unwrap();
            while let Some(chunk) = field.next().await {
                let data = chunk.unwrap();
                f.write_all(&data).unwrap();
            }

            let target_module = String::from("deployer");
            let data = craft_type_notify_message(&target_module, &["DEPLOY", format!("{}", filename).as_str()]);
            slaves.notify(target_module, data);

            return Ok(HttpResponse::Ok().body(format!("File uploaded successfully: {}", filepath)));
        }
    }

    Ok(HttpResponse::BadRequest().body("No file uploaded"))
}