use actix_web::{web, HttpRequest, HttpResponse};
use actix_multipart::Multipart;
use futures_util::{StreamExt, TryStreamExt};
use uuid::Uuid;
use std::sync::mpsc::{self, Receiver, Sender};
use std::io::Write;
use std::fs::{File, copy};

use crate::notifier::{craft_type_notify_message, Notifier, NotifierCommInfo, NotifierComms};
use crate::database::{challenge, DbConnection};
use crate::utils::{is_time_schedule_valid, MAGIC_TIME};

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
        // id: Uuid::new_v4().as_u128(),
        name: "challenge_upload_handler".to_string(),
        broadcast_channel: notifier_sender
    };
    notifier.slaves.comm_infos.push(comm_info);
}

pub(crate) async fn handle_challenge(slaves: web::Data<NotifierComms>, db_conn: web::Data<DbConnection>, req: HttpRequest, mut payload: Multipart) -> Result<HttpResponse, actix_web::Error> {

    let start_time_header = req.headers().get("X-start");
    let end_time_header = req.headers().get("X-end");

    let start_time = match start_time_header {
        Some(time) => match i128::from_str_radix(time.to_str().unwrap(), 10) {
            Ok(epoch) => epoch-MAGIC_TIME,
            Err(_) => return Ok(HttpResponse::BadRequest().body(format!("Invalid start time")))
        },
        None => return Ok(HttpResponse::BadRequest().body(format!("Missing start time")))
    };

    let end_time = match end_time_header {
        Some(time) => match i128::from_str_radix(time.to_str().unwrap(), 10) {
            Ok(epoch) => epoch-MAGIC_TIME,
            Err(_) => return Ok(HttpResponse::BadRequest().body(format!("Invalid end time")))
        },
        None => return Ok(HttpResponse::BadRequest().body(format!("Missing end time")))
    };

    if !is_time_schedule_valid(start_time, end_time) {
        return Ok(HttpResponse::BadRequest().body(format!("Please adjust start_time/end_time")));
    }

    while let Some(mut field) = payload.try_next().await? {
        let data_part = field.content_disposition().unwrap();
        if let Some(_) = data_part.get_filename() {

            let filename = Uuid::new_v4();
            let filepath = format!("./archives/{}.tar.gz", filename);

            let mut f = File::create(&filepath).unwrap();
            while let Some(chunk) = field.next().await {
                let data = chunk.unwrap();
                f.write_all(&data).unwrap();
            }

            let target_module = String::from("deployer");
            let data = craft_type_notify_message(&target_module, &["schedule", &filename.to_string(), &start_time.to_string(), &end_time.to_string()]);
            slaves.notify(target_module, data);

            let chall = challenge::ChallengeData {
                id: 0,
                challenge_name: filename.to_string(),
                score: 500,
                category: "Pwn".to_string(),
                solved_by: Vec::new(),
                running: false,
                connection_string: "".to_string()
            };
            if db_conn.store_challenge_metadata(chall).await {
                if copy(filepath.to_string(), format!("./attachments/{}.tar.gz", filename))
                    .expect("cannot copy to attachments") > 0 {
                        return Ok(HttpResponse::Ok().body(format!("Failed to copy to attachments: {}", filepath)));
                };
                return Ok(HttpResponse::Ok().body(format!("File uploaded successfully: {}", filepath)));
            }

            return Ok(HttpResponse::Ok().body(format!("Failed to store challenge to database: {}", filename)));
        }
    }

    Ok(HttpResponse::BadRequest().body("No file uploaded"))
}