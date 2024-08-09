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

            "flag_submit" => cmd_flag_submit(&mut ctx, &data),

            "cleanup" => cmd_cleanup(&mut ctx, &data),

            _ => panic!("unknown cmd")
        };
    }
}

fn deserialize_data(serialized_data: &Vec<u8>) -> HashMap<&str, String> {
    let data: HashMap<&str, String> = serde_json::from_slice(serialized_data.as_slice()).expect("deserialize failed!");
    return data;
}

pub async fn handle_submission(slaves: web::Data<NotifierComms>, path: web::Path<(String,String)>, req: HttpRequest) -> Result<HttpResponse, actix_web::Error> {
    let cookie = req.cookie("auth").unwrap_or(Cookie::build("auth", "").finish());

    let claims: BTreeMap<String, String> = get_jwt_claims(cookie.value()).unwrap_or(BTreeMap::new());
    let username = claims.get("username").expect("Missing username in JWT");

    if claims.len() == 0 {
        return Ok(forbiden("Not authenticated"));
    }
    let challenge_name = &path.0;
    let submitted_flag = &path.1;
    
    let target_module = String::from_str("flag_receiver").unwrap();
    let data = craft_type_notify_message(&target_module, &["flag_submit", challenge_name, submitted_flag, username]);

    slaves.notify(target_module, data);
    return Ok(HttpResponse::Ok().body(format!("flag submitted successfully")));
}

fn cmd_flag_info(ctx: &mut FlagReceiverCtx, data: &HashMap<&str, String>) {
    let challenge_name = data.get("challenge_name").expect("missing challenge_name").to_string();
    let flag = data.get("flag").expect("missing flag").to_string();
    ctx.challenge_infos.insert(challenge_name, flag);
}

fn cmd_flag_submit(ctx: &mut FlagReceiverCtx, data: &HashMap<&str, String>) {
    let submitted_flag = data.get("flag").expect("missing flag").to_string();
    let username = data.get("submit_by").expect("missing username").to_string();
    let chall_name = data.get("challenge_name").expect("mssing challenge_name").to_string();
    let rt = Runtime::new().expect("failed creating tokio runtime");

    for (challenge_name, flag) in &ctx.challenge_infos {
        if &submitted_flag == flag {
            let solve_history = SolveHistoryEntry::new(
                username,
                challenge_name.clone(),
                true,
                submitted_flag
            );
    
            println!("saving history");
            rt.block_on(ctx.db_conn.log_solve_result(solve_history));    
            return;
        }
    }

    let solve_history = SolveHistoryEntry::new(
        username,
        chall_name,
        false,
        submitted_flag
    );

    println!("saving history");

    rt.block_on(ctx.db_conn.log_solve_result(solve_history));
}

fn cmd_cleanup(ctx: &mut FlagReceiverCtx, data: &HashMap<&str, String>) {
    let challenge_name = data.get("challenge_name").expect("missing challenge_name").to_string();
    ctx.challenge_infos.remove(&challenge_name).expect(&format!("no challenge to cleanup: {}", challenge_name));
}