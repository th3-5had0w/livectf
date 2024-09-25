use std::{collections::{BTreeMap, HashMap}, fmt::Display, sync::mpsc::{self, Receiver, Sender}, thread::spawn};

use actix_web::{web, HttpResponse, HttpRequest, cookie::Cookie};
use tokio::runtime::Runtime;
// use uuid::Uuid;

use crate::{notifier::{CleanUpCmdArgs, CtrlMsg, FlagInfoCmdArgs, FlagReceiverCommand, FlagSubmitCmdArgs, NotifierCommInfo, NotifierComms}, Notifier};
use crate::database::{solve_history::SolveHistoryEntry, DbConnection};
use crate::web_interface::{get_jwt_claims, forbiden};

#[derive(Debug)]
enum Error {
    FlagInfo(String),
    CleanUp(String),
    FlagSubmit(String)
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::FlagInfo(err) => write!(f, "FlagReceiver - FlagInfoFail: {}", err),
            Error::CleanUp(err) => write!(f, "FlagReceiver - CleanUpFail: {}", err),
            Error::FlagSubmit(err) => write!(f, "FlagReceiver - FlagSubmitFail: {}", err),
        }
    }
}

impl std::error::Error for Error {}

struct ChallengeInfo {
    challenge_name: String,
    challenge_flag: String
}

struct FlagReceiverCtx {
    // main comm channel
    sender: Sender<CtrlMsg>,
    listener: Receiver<Vec<u8>>,

    challenge_infos: Vec<ChallengeInfo>,
    db_conn: DbConnection
}

pub(crate) fn init(notifier: &mut Notifier, my_sender: Sender<CtrlMsg>, db_conn: DbConnection) {
    let (notifier_sender, my_receiver) : (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel();
    let ctx = FlagReceiverCtx {
        sender: my_sender,
        listener: my_receiver,
        challenge_infos: Vec::new(),
        db_conn
    };


    let comm_info = NotifierCommInfo {
        // id: Uuid::new_v4().as_u128(),
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

        let cmd: FlagReceiverCommand = serde_json::from_slice(
            ctx.listener.recv()
                        .expect("deployer channel communication exited")
                        .as_slice()
        ).expect("deserialize failed");
        println!("flag received recv()");

        match cmd {
            FlagReceiverCommand::CleanUpCmd(args) => if let Err(err) = cmd_cleanup(&mut ctx, args) {
                todo!("handle")
            },
            FlagReceiverCommand::FlagSubmitCmd(args) => if let Err(err) = cmd_flag_submit(&mut ctx, args) {
                todo!("handle")
            },
            FlagReceiverCommand::FlagInfoCmd(args) => if let Err(err) = cmd_flag_info(&mut ctx, args) {
                todo!("handle")
            },
            _ => panic!("unknown cmd")
        }
    }
}

fn deserialize_data(serialized_data: &Vec<u8>) -> HashMap<&str, String> {
    let data: HashMap<&str, String> = serde_json::from_slice(serialized_data.as_slice()).expect("deserialize failed!");
    return data;
}

pub async fn handle_submission(slaves: web::Data<NotifierComms>, path: web::Path<(String,)>, req: HttpRequest) -> Result<HttpResponse, actix_web::Error> {
    let cookie = req.cookie("auth").unwrap_or(Cookie::build("auth", "").finish());

    let claims: BTreeMap<String, String> = get_jwt_claims(cookie.value()).unwrap_or(BTreeMap::new());
    let no_username = "".to_string();
    let username = claims.get("username").unwrap_or(&no_username);

    if claims.len() == 0 || username.len() == 0 {
        return Ok(forbiden("Not authenticated"));
    }

    let submitted_flag = &path.0;

    let msg = CtrlMsg::FlagReceiver(
        crate::notifier::FlagReceiverCommand::FlagSubmitCmd(
            FlagSubmitCmdArgs {
                flag: submitted_flag.to_string(),
                submit_by: username.to_string()
            }
        )
    );

    slaves.notify(msg);
    return Ok(HttpResponse::Ok().body(format!("flag submitted successfully")));
}

fn cmd_flag_info(ctx: &mut FlagReceiverCtx, args: FlagInfoCmdArgs) -> Result<(), Error> {

    let challenge_name = args.challenge_name;

    let challenge_flag = args.flag;

    ctx.challenge_infos.push(
        ChallengeInfo { 
            challenge_name,
            challenge_flag 
        }
    );
    Ok(())
}

fn cmd_flag_submit(ctx: &mut FlagReceiverCtx, args: FlagSubmitCmdArgs) -> Result<(), Error> {
    let submitted_flag = args.flag;


    let username = args.submit_by;

    let rt = Runtime::new().expect("failed creating tokio runtime");

    for challenge in &ctx.challenge_infos {
        if submitted_flag == challenge.challenge_flag {
            let solve_history = SolveHistoryEntry::new(
                username.clone(),
                challenge.challenge_name.clone(),
                true,
                submitted_flag
            );
            
            rt.block_on(ctx.db_conn.user_add_score(
                username, 
                challenge.challenge_name.clone()
                )
            );
            
            rt.block_on(ctx.db_conn.log_solve_result(solve_history));    
            return Ok(());
        }
    }

    let solve_history = SolveHistoryEntry::new(
        username,
        String::from("None"),
        false,
        submitted_flag
    );

    rt.block_on(ctx.db_conn.log_solve_result(solve_history));

    Ok(())
}

fn cmd_cleanup(ctx: &mut FlagReceiverCtx, args: CleanUpCmdArgs) -> Result<(), Error> {

    let challenge_name = args.challenge_name;

    ctx.challenge_infos.retain(|challenge| {
        challenge.challenge_name != challenge_name
    });

    Ok(())
}