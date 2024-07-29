use std::{collections::HashMap, str::FromStr, sync::mpsc::{self, Receiver, Sender}, thread::spawn, collections::BTreeMap};

use actix_web::{web, HttpResponse, HttpRequest, cookie::Cookie};
use tokio::runtime::Runtime;
use uuid::Uuid;

use crate::{notifier::{craft_type_notify_message, NotifierCommInfo, NotifierComms}, Notifier};
use crate::database::{solve_history::SolveHistoryEntry, DbConnection};
use crate::web_interface::{get_jwt_claims, forbiden};

struct FlagReceiverCtx {
    // main comm channel
    sender: Sender<(String, Vec<u8>)>,
    listener: Receiver<Vec<u8>>,

    challenge_infos: HashMap<String, String>,
    db_conn: DbConnection
}

pub(crate) fn init(notifier: &mut Notifier, my_sender: Sender<(String, Vec<u8>)>, db_conn: DbConnection) {
    let (notifier_sender, my_receiver) : (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel();
    let ctx = FlagReceiverCtx {
        sender: my_sender,
        listener: my_receiver,
        challenge_infos: HashMap::new(),
        db_conn
    };


    let comm_info = NotifierCommInfo {
        id: Uuid::new_v4().as_u128(),
        name: "flag_receiver".to_string(),
        broadcast_channel: notifier_sender
    };
    notifier.slaves.comm_infos.push(comm_info);

    
    spawn(move || {
        flag_receiver_loop(ctx)
    });
}

fn flag_receiver_loop(mut ctx: FlagReceiverCtx) {
    loop {
        let serialized_data = ctx.listener.recv().expect("flag receiver channel communication exited");
        println!("flag received recv()");
        let data = deserialize_data(&serialized_data);
        match data.get("cmd").expect("missing cmd").as_str() {
            "flag_info" => cmd_flag_info(&mut ctx, &data),
            "flag_submit" => {
                cmd_flag_submit(&mut ctx, &data);
            },
            _ => panic!("unknown cmd")
        };
    }
}

fn deserialize_data(serialized_data: &Vec<u8>) -> HashMap<&str, String> {
    let data: HashMap<&str, String> = serde_json::from_slice(serialized_data.as_slice()).expect("deserialize failed!");
    return data;
}

pub async fn handle_submission(slaves: web::Data<NotifierComms>, path: web::Path<(String,)>, req: HttpRequest) -> Result<HttpResponse, actix_web::Error> {
    let cookie = req.cookie("auth").unwrap_or(Cookie::build("auth", "").finish());

    let claims: BTreeMap<String, String> = get_jwt_claims(cookie.value()).unwrap_or(BTreeMap::new());
    let no_id = String::from("-1");
    let user_id = claims.get("id").unwrap_or(&no_id);

    if claims.len() == 0 {
        return Ok(forbiden("Not authenticated"));
    }
    let submitted_flag = &path.0;
    
    let target_module = String::from_str("flag_receiver").unwrap();
    let data = craft_type_notify_message(&target_module, &["flag_submit", "", submitted_flag, user_id]);

    slaves.notify(target_module, data);
    return Ok(HttpResponse::Ok().body(format!("File uploaded successfully: {}", "d")));
}

fn cmd_flag_info(ctx: &mut FlagReceiverCtx, data: &HashMap<&str, String>) {
    let challenge_filename = data.get("challenge_filename").expect("missing challenge_filename").to_string();
    let flag = data.get("flag").expect("missing flag").to_string();
    ctx.challenge_infos.insert(flag, challenge_filename);
}

fn cmd_flag_submit(ctx: &mut FlagReceiverCtx, data: &HashMap<&str, String>) {
    let submitted_flag = data.get("flag").expect("missing flag").to_string();
    let user_id = data.get("submit_by").expect("missing user_id").to_string();
    if ctx.challenge_infos.contains_key(&submitted_flag) {
        // dung flag
        let solve_history = SolveHistoryEntry::new(
            user_id.parse::<i32>().expect("user_id must be of type `i32`"),
            true,
            submitted_flag
        );

        println!("saving history");
        let db = ctx.db_conn.clone();

        let rt = Runtime::new().expect("failed creating tokio runtime");
        rt.block_on(ctx.db_conn.log_solve_result(solve_history));
        
    } else {
        // sai flag 
        let solve_history = SolveHistoryEntry::new(
            user_id.parse::<i32>().expect("user_id must be of type `i32`"),
            false,
            submitted_flag
        );

        println!("saving history");

        let rt = Runtime::new().expect("failed creating tokio runtime");
        rt.block_on(ctx.db_conn.log_solve_result(solve_history));
    }
}