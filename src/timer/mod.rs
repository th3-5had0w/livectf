use core::time;
use std::{collections::HashMap, fmt::Display, str::FromStr, sync::{mpsc::{self, Receiver, Sender}, Arc, Mutex}, thread::{sleep, spawn}, time::{SystemTime, UNIX_EPOCH}};

// use uuid::Uuid;

use crate::notifier::{craft_type_notify_message, Notifier, NotifierCommInfo};

#[derive(Debug)]
enum Error {
    Enqueue(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Enqueue(err) => write!(f, "Timer - EnqueueFail: {}", err),
        }
    }
}

impl std::error::Error for Error {}

struct ScheduledChallenge {
    challenge_name: String,
    public_time: i128,
    interval: i128,
    pre_announce: i128,
    is_announced: bool,
    is_running: bool,
}

struct TimerQueue {
    scheduled_challenge_queue: Vec<ScheduledChallenge>
}

struct TimerCtx {
    // main comm channel
    sender: Sender<(String, Vec<u8>)>,
    listener: Receiver<Vec<u8>>,
}

pub(crate) fn init(notifier: &mut Notifier, my_sender: Sender<(String, Vec<u8>)>) {
    let (notifier_sender, my_receiver) : (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel();
    
    let ctx = TimerCtx {
        sender: my_sender,
        listener: my_receiver,
    };

    let comm_info = NotifierCommInfo {
        // id: Uuid::new_v4().as_u128(),
        name: "timer".to_string(),
        broadcast_channel: notifier_sender
    };
    notifier.slaves.comm_infos.push(comm_info);
    spawn(move || {
        timer_loop(ctx)
    });
}

fn timer_loop(mut ctx: TimerCtx) {
    let timer_queue: Arc<Mutex<TimerQueue>> = Arc::new(Mutex::new(TimerQueue { scheduled_challenge_queue: Vec::new() }));
    let timer_queue_clone = Arc::clone(&timer_queue);
    let countdown_sender = ctx.sender.clone();
    spawn(move || {
        countdown(timer_queue_clone, countdown_sender)
    });
    loop {
        let serialized_data = ctx.listener.recv().expect("timer channel communication exited");
        let data = deserialize_data(&serialized_data);
        match data.get("cmd").expect("missing cmd").as_str() {

            "enqueue" => if let Err(err) = cmd_enqueue(&mut ctx, timer_queue.clone(), &data) {
                !todo!("handle it")
            },

            "deploy_info" => cmd_deploy_info(&mut ctx, timer_queue.clone(), &data),

            _ => panic!("unknown cmd")
        }
    }
}

fn deserialize_data(serialized_data: &Vec<u8>) -> HashMap<&str, String> {
    let data: HashMap<&str, String> = serde_json::from_slice(serialized_data.as_slice()).expect("deserialize failed!");
    return data;
}

fn cmd_enqueue(_ctx: &mut TimerCtx, timer_queue_guard: Arc<Mutex<TimerQueue>>, data: &HashMap<&str, String>) -> Result<(), Error>{
    let mut timer_queue = timer_queue_guard.lock().expect("failed acquiring lock");
    let challenge_name = data.get("challenge_name")
                                            .ok_or(Error::Enqueue(
                                                String::from_str("missing challenge name").unwrap()
                                            ))?.to_owned();

    let public_time = i128::from_str_radix(
        data.get("public_time")
                .ok_or(Error::Enqueue(
                    String::from_str("missing public time").unwrap()
                ))?.to_owned().as_str(),
                10
    ).map_err(|e| Error::Enqueue(format!("{}", e)))?;

    let interval = i128::from_str_radix(
        data.get("interval")
                .ok_or(Error::Enqueue(
                    String::from_str("missing interval").unwrap()
                ))?.to_owned().as_str(),
                10
    ).map_err(|e| Error::Enqueue(format!("{}", e)))?;

    let pre_announce = i128::from_str_radix(
        data.get("pre_announce")
                .ok_or(Error::Enqueue(
                    String::from_str("missing pre-announce time").unwrap()
                ))?.to_owned().as_str(),
                10
    ).map_err(|e| Error::Enqueue(format!("{}", e)))?;

    timer_queue.scheduled_challenge_queue.push(
        ScheduledChallenge { 
            challenge_name,
            public_time,
            interval,
            pre_announce,
            is_announced: false,
            is_running: false
        }
    );
    Ok(())
}

fn countdown(timer_queue_guard: Arc<Mutex<TimerQueue>>, sender: Sender<(String, Vec<u8>)>) {

    loop {
        sleep(time::Duration::from_secs(1));
        let mut timer_queue = timer_queue_guard.lock().expect("failed acquiring lock");

        let mut timeout: Option<String> = None;

        let now = SystemTime::now().duration_since(UNIX_EPOCH).expect("back to the future!!!").as_secs() as i128;

        for scheduled_challenge in &mut timer_queue.scheduled_challenge_queue {
            if now >= scheduled_challenge.public_time + scheduled_challenge.interval {
                timeout = Some(scheduled_challenge.challenge_name.clone());
                let target_module = String::from("deployer");
                let data = craft_type_notify_message(&target_module, &["destroy", &scheduled_challenge.challenge_name]);
                sender.send((target_module, data)).expect("deployer cannot send");

            } else if !scheduled_challenge.is_running && now >= scheduled_challenge.public_time {
                scheduled_challenge.is_running = true;
                let target_module = String::from("deployer");
                let data = craft_type_notify_message(&target_module, &["public", &scheduled_challenge.challenge_name]);
                sender.send((target_module, data)).expect("deployer cannot send");
                
            } else if !scheduled_challenge.is_announced && now >= scheduled_challenge.public_time - scheduled_challenge.pre_announce {
                todo!("announce");
                scheduled_challenge.is_announced = true;
            }
        }

        if let Some(challenge_name) = timeout {
            timer_queue.scheduled_challenge_queue.retain(|challenge| challenge.challenge_name != challenge_name);
        }
    }
}

fn cmd_deploy_info(_ctx: &mut TimerCtx, timer_queue_guard: Arc<Mutex<TimerQueue>>, data: &HashMap<&str, String>) {
    let mut timer_queue = timer_queue_guard.lock().expect("failed acquiring lock");
    let challenge_name = data.get("challenge_name").expect("missing challenge_name");
    let deploy_status = data.get("deploy_status").expect("missing deploy_status");

    if deploy_status == "fail" {
        timer_queue.deployed_queue.retain(|deployed_challenge| &deployed_challenge.0 != challenge_name);
    }
}
