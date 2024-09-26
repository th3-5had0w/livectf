use actix_web::{web, HttpRequest, HttpResponse, cookie::Cookie};
use actix_multipart::Multipart;
use futures_util::{StreamExt, TryStreamExt};
use uuid::Uuid;
use std::sync::mpsc::{self, Receiver, Sender};
use std::io::Write;
use std::fs::{File, copy};
use std::collections::BTreeMap;

use crate::notifier::{CtrlMsg, DeployCmdArgs, Notifier, NotifierCommInfo, NotifierComms};
use crate::database::{challenge, DbConnection};
use crate::utils::{is_time_schedule_valid, MAGIC_TIME};
use crate::web_interface::{get_error, success, get_jwt_claims, forbiden, unauthorized};
// struct ChallengeUploadHandlerCtx {
//     sender: Sender<(String, Vec<u8>)>,
//     listener: Receiver<Vec<u8>>,
    
//     db_conn: DbConnection
// }

pub(crate) fn init(notifier: &mut Notifier, _my_sender: Sender<CtrlMsg>, _db_conn: DbConnection) {
    let (notifier_sender, _my_receiver) : (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel();
    // let ctx = ChallengeUploadHandlerCtx {
    //     sender: my_sender,
    //     listener: my_receiver,
    //     db_conn
    // };


    let comm_info = NotifierCommInfo {
        // id: Uuid::new_v4().as_u128(),
        name: "challenge_upload_handler".to_string(),
        broadcast_channel: notifier_sender
    };
    notifier.slaves.comm_infos.push(comm_info);
}

pub(crate) async fn handle_challenge(slaves: web::Data<NotifierComms>, db_conn: web::Data<DbConnection>, req: HttpRequest, mut payload: Multipart) -> Result<HttpResponse, actix_web::Error> {

    let cookie = req.cookie("auth").unwrap_or(Cookie::build("auth", "").finish());

    let claims: BTreeMap<String, String> = get_jwt_claims(cookie.value()).unwrap_or(BTreeMap::new());

    if claims.len() == 0 {
        return Ok(forbiden("Not authenticated"));
    }

    let is_admin = claims.get("is_admin").unwrap_or(&"false".to_string()).parse::<bool>().unwrap_or(false);
    if !is_admin {
        return Ok(unauthorized("You are not admin"));
    }

    let start_time_header = req.headers().get("X-start");
    let interval_header = req.headers().get("X-interval");
    let pre_announce_header =  req.headers().get("X-preannounce");

    let start_time = match start_time_header.map(|x| i128::from_str_radix(x.to_str().unwrap(), 10)) {
        Some(time) => match time {
            Ok(epoch) => epoch-MAGIC_TIME,
            _ => return Ok(HttpResponse::BadRequest().body("Invalid start time"))
        },
        None => return Ok(HttpResponse::BadRequest().body("Missing start time"))
    };

    let interval = match interval_header.map(|x| i128::from_str_radix(x.to_str().unwrap(), 10)) {
        Some(time) => match time {
            Ok(epoch) => epoch-MAGIC_TIME,
            _ => return Ok(HttpResponse::BadRequest().body("Invalid interval"))
        },
        None => return Ok(HttpResponse::BadRequest().body("Missing interval"))
    };

    let pre_announce = match pre_announce_header.map(|x| i128::from_str_radix(x.to_str().unwrap(), 10)) {
        Some(time) => match time {
            Ok(epoch) => epoch-MAGIC_TIME,
            _ => return Ok(HttpResponse::BadRequest().body("Invalid interval"))
        },
        None => return Ok(HttpResponse::BadRequest().body("Missing interval"))
    };

    if !is_time_schedule_valid(start_time, end_time) {
        todo!("need changes");
        return Ok(get_error("Please adjust start_time/end_time"));
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

            let msg = CtrlMsg::Deployer(
                crate::notifier::DeployerCommand::DeployCmd(DeployCmdArgs {
                    challenge_name: filename.to_string(),
                    start_time,
                    interval,
                    pre_announce
                })
            );
            slaves.notify(msg);

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
                    .expect("cannot copy to attachments") == 0 {
                        return Ok(get_error(&format!("Failed to copy to attachments: {}", filepath)));
                };
                return Ok(success(&format!("File uploaded successfully: {}", filepath)));
            }

            return Ok(success(&format!("Failed to store challenge to database: {}", filename)));
        }
    }

    Ok(HttpResponse::BadRequest().body("No file uploaded"))
}