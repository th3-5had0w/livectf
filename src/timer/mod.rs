use core::time;
use std::{collections::{BinaryHeap, HashMap}, sync::{mpsc::{self, Receiver, Sender}, Arc, Mutex}, thread::{sleep, spawn}, time::{SystemTime, UNIX_EPOCH}};

use uuid::Uuid;

use crate::{database::DbConnection, notifier::{craft_type_notify_message, Notifier, NotifierCommInfo}};

#[derive(PartialEq, Eq)]
struct ChallengeTimer(String, i128, i128);

#[derive(PartialEq, Eq)]
struct StartedChallenge(String, i128);

impl Ord for ChallengeTimer {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.1.cmp(&self.1)
    }
}

impl PartialOrd for ChallengeTimer {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for StartedChallenge {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.1.cmp(&self.1)
    }
}

impl PartialOrd for StartedChallenge {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

struct TimerCtx {
    // main comm channel
    sender: Sender<(String, Vec<u8>)>,
    listener: Receiver<Vec<u8>>,
    db_conn: DbConnection
}

pub(crate) fn init(notifier: &mut Notifier, my_sender: Sender<(String, Vec<u8>)>, db_conn: DbConnection) {
    let (notifier_sender, my_receiver) : (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel();
    
    let ctx = TimerCtx {
        sender: my_sender,
        listener: my_receiver,
        db_conn
    };

    let comm_info = NotifierCommInfo {
        id: Uuid::new_v4().as_u128(),
        name: "timer".to_string(),
        broadcast_channel: notifier_sender
    };
    notifier.slaves.comm_infos.push(comm_info);
    spawn(move || {
        timer_loop(ctx)
    });
}

fn timer_loop(mut ctx: TimerCtx) {
    let timer_queue: Arc<Mutex<BinaryHeap<ChallengeTimer>>> = Arc::new(Mutex::new(BinaryHeap::new()));
    let timer_queue_clone = Arc::clone(&timer_queue);
    let countdown_sender = ctx.sender.clone();
    spawn(move || {
        countdown(timer_queue_clone, countdown_sender)
    });
    loop {
        let serialized_data = ctx.listener.recv().expect("timer channel communication exited");
        let data = deserialize_data(&serialized_data);
        match data.get("cmd").expect("missing cmd").as_str() {

            "enqueue" => cmd_enqueue(&mut ctx, timer_queue.clone(), &data),

            _ => panic!("unknown cmd")
        }
    }
}

fn deserialize_data(serialized_data: &Vec<u8>) -> HashMap<&str, String> {
    let data: HashMap<&str, String> = serde_json::from_slice(serialized_data.as_slice()).expect("deserialize failed!");
    return data;
}

fn cmd_enqueue(ctx: &mut TimerCtx, timer_queue: Arc<Mutex<BinaryHeap<ChallengeTimer>>>, data: &HashMap<&str, String>) {
    let mut queue = timer_queue.lock().expect("failed acquiring lock");
    let challenge_name = data.get("challenge_name").expect("missing challenge_name");
    let start_time = i128::from_str_radix(
        data.get("start_time").expect("missing start_time"),
        10).expect("invalid start_time");
    let end_time = i128::from_str_radix(
                            data.get("end_time").expect("missing end_time"),
                            10).expect("invalid end_time");
    queue.push(ChallengeTimer(challenge_name.to_string(), start_time, end_time));
}

fn countdown(mut timer_queue: Arc<Mutex<BinaryHeap<ChallengeTimer>>>, sender: Sender<(String, Vec<u8>)>) {
    let mut started_challenge_queue: BinaryHeap<StartedChallenge> = BinaryHeap::new();
    loop {
        sleep(time::Duration::from_secs(60));
        let mut queue = timer_queue.lock().expect("failed acquiring lock");
        let now_epoch = i128::try_from(
            SystemTime::now().duration_since(UNIX_EPOCH).expect("back to the future!!!").as_secs()
        ).expect("Cannot convert current epoch to i128");

        if (queue.len() != 0) {

            let challenge_start_time = queue.peek().expect("failed peeking timer queue").1;

            if now_epoch >= challenge_start_time {

                let challenge_name = &queue.peek().expect("failed peeking timer queue").0;
                let challenge_end_time = queue.peek().expect("failed peeking timer queue").2;
                started_challenge_queue.push(StartedChallenge(challenge_name.to_string(), challenge_end_time));
                let target_module = String::from("deployer");
                let data = craft_type_notify_message(&target_module, &["deploy", challenge_name]);
                sender.send((target_module, data)).expect("deployer cannot send");
                queue.pop();
                
            } else if now_epoch < challenge_start_time {
                // TODO:
                // Make announce through telgram bot or discord bot or sth!!!!!
            }
        }

        if (started_challenge_queue.len() != 0) {
            if now_epoch >= started_challenge_queue.peek().expect("failed peeking timer queue").1 {

                let target_module = String::from("deployer");
                let data = craft_type_notify_message(&target_module, &["destroy", &started_challenge_queue.peek().expect("failed peeking timer queue").0]);
                sender.send((target_module, data)).expect("deployer cannot send");
                started_challenge_queue.pop();
            }
        }
    }
}