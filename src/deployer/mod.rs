use core::fmt;
use std::{collections::HashMap, fmt::Display, fs::File, io::Write, process::Command, str::FromStr, sync::mpsc::{self, Receiver, Sender}, thread::spawn};

use rand::Rng;
use uuid::Uuid;
use tokio::runtime::Runtime;

use crate::{database::DbConnection, notifier::{self, craft_type_notify_message, NotifierCommInfo}, Notifier};

#[derive(Clone)]
struct Challenge {
    challenge_filename: String,
    challenge_image: String,
    flag: String,
    port: u16
}

#[derive(Debug)]
enum Error {
    BuildFail(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::BuildFail(err) => write!(f, "Error: {}", err)
        }
    }
}

impl std::error::Error for Error {}

struct DeployerCtx {
    // main comm channel
    sender: Sender<(String, Vec<u8>)>,
    listener: Receiver<Vec<u8>>,
    
    db_conn: DbConnection,
    challenges: Vec<Challenge>
}

impl DeployerCtx {
    fn is_port_used(&self, port: u16) -> bool {
        for challenge in &self.challenges {
            if challenge.port == port {
                return true;
            }
        }
        return false;
    }

    fn set_challenge_port(&mut self, challenge_filename: &String, port: u16) {
        let mut exist: bool = false;
        for challenge in self.challenges.iter_mut() {
            if challenge.challenge_filename == challenge_filename.to_string() {
                challenge.port = port;
                exist = true;
            }
        }

        if !exist { panic!("something went wrong! must not reach here!") };
    }

    fn get_challenge(&mut self, challenge_filename: &String) -> Challenge {
        for challenge in self.challenges.clone() {
            if challenge.challenge_filename == challenge_filename.to_string() {
                return challenge;
            }
        }

        panic!("something went wrong! must not reach here!")
    }
}

pub fn init(notifier: &mut Notifier, my_sender: Sender<(String, Vec<u8>)>, db_conn: DbConnection) {
    let (notifier_sender, my_receiver) : (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel();
    let ctx = DeployerCtx {
        sender: my_sender,
        listener: my_receiver,
        db_conn,
        challenges: Vec::new(),
    };

    
    let comm_info = NotifierCommInfo {
        // id: Uuid::new_v4().as_u128(),
        name: "deployer".to_string(),
        broadcast_channel: notifier_sender
    };
    notifier.slaves.comm_infos.push(comm_info);


    spawn(move || {
        deployer_loop(ctx)
    });
}

fn deployer_loop(mut ctx: DeployerCtx) {
    loop {
        let serialized_data = ctx.listener.recv().expect("deployer channel communication exited");
        let data = deserialize_data(&serialized_data);
        match data.get("cmd").expect("missing cmd").as_str() {
            "deploy" => cmd_deploy(&mut ctx, &data),
            "schedule" => cmd_schedule(&mut ctx, &data),
            "destroy" => cmd_destroy(&mut ctx, &data),
            _ => panic!("unknown cmd")
        }
    }
}

fn cmd_schedule(ctx: &mut DeployerCtx, data: &HashMap<&str, String>) -> Result<(), Error> {

    let challenge_filename = data.get("challenge_filename").expect("missing challenge_filename");
    let start_time = data.get("start_time").expect("missing start_time");
    let end_time = data.get("end_time").expect("missing end_time");

        let unpack_success = unpack_challenge(challenge_filename);

        if unpack_success {
            let flag = generate_challenge_flag(challenge_filename);
            let challenge_image = build_challenge(challenge_filename)?;
            let challenge_image = String::from_utf8(challenge_image)
                                                    .expect("failed converting docker image name")
                                                    .trim().replace("sha256:", "");
                
                ctx.challenges.push(Challenge {
                    challenge_image,
                    challenge_filename: challenge_filename.to_string(),
                    flag,
                    port: 0,
                });

                let target_module = String::from("timer");
                let data = craft_type_notify_message(&target_module, &["enqueue", &challenge_filename.to_string(), &start_time.to_string(), &end_time.to_string()]);
                ctx.sender.send((target_module, data)).expect("deployer cannot send");

        } else {
            println!("unpack failed");
        }
        Ok(())
}

fn cmd_deploy(ctx: &mut DeployerCtx, data: &HashMap<&str, String>) {
    let challenge_filename = data.get("challenge_filename").expect("missing challenge_filename");
    let rt = Runtime::new().expect("failed creating tokio runtime");

    let mut rng = rand::thread_rng();
    let mut port: u16 = rng.gen_range(0x1000..0xffff);
    loop {
        if ctx.is_port_used(port) {
            port = rng.gen_range(0x1000..0xffff);
        }
        else {
            break
        }
    }
    ctx.set_challenge_port(challenge_filename, port);
    let challenge = ctx.get_challenge(challenge_filename);
    
    let deploy_success = deploy_challenge(&challenge.challenge_filename, &challenge.challenge_image, challenge.port);

    if deploy_success {

        let target_module = String::from_str("flag_receiver").unwrap();
        let cmd = String::from_str("flag_info").unwrap();
        let data = notifier::craft_type_notify_message(&target_module, &[cmd, challenge_filename.to_string(), challenge.flag]);
        ctx.sender.send((target_module, data)).expect("deployer cannot send");
        
        let conn_string = format!("nc localhost {}", challenge.port);
        rt.block_on(ctx.db_conn.set_challenge_connection_string(challenge_filename.to_string(), conn_string));
        rt.block_on(ctx.db_conn.set_challenge_running(challenge_filename.to_string(), true));

        println!("Deploy success {}", challenge_filename);
    } else {

        let target_module = String::from_str("timer").unwrap();
        let cmd = String::from_str("deploy_info").unwrap();
        let data = notifier::craft_type_notify_message(&target_module, &[cmd, challenge_filename.to_string(), "fail".to_string()]);
        ctx.sender.send((target_module, data)).expect("deployer cannot send");
        ctx.challenges.retain(|challenge| &challenge.challenge_filename != challenge_filename);
        println!("Deploy failed {}", challenge_filename);
    }
}

fn cmd_destroy(ctx: &mut DeployerCtx, data: &HashMap<&str, String>) {
    let challenge_filename = data.get("challenge_filename").expect("missing challenge_filename");
    let rt = Runtime::new().expect("failed creating tokio runtime");
    let destroy_success = destroy_challenge(challenge_filename); 
    if destroy_success {
        let target_module = String::from_str("flag_receiver").unwrap();
        let data = notifier::craft_type_notify_message(&target_module, &["cleanup", challenge_filename]);
        ctx.sender.send((target_module, data)).expect("deployer cannot send");

        ctx.challenges.retain(|challenge| &challenge.challenge_filename != challenge_filename);
        rt.block_on(ctx.db_conn.set_challenge_running(challenge_filename.to_string(), false));
    }
    else {
        println!("destroy failed");
    }
}

fn destroy_challenge(challenge_filename: &String) -> bool {
    let output = Command::new("docker")
                                .args(["rm", "-f", challenge_filename])
                                .output()
                                .expect("failed running bash shell");
    return output.status.success();
}

fn deserialize_data(serialized_data: &Vec<u8>) -> HashMap<&str, String> {
    let data: HashMap<&str, String> = serde_json::from_slice(serialized_data.as_slice()).expect("deserialize failed!");
    return data;
}

fn unpack_challenge(challenge_filename: &String) -> bool {
    let output = Command::new("tar")
                                .args(["-xf", &format!("{}.tar.gz", challenge_filename), "--one-top-level"])
                                .current_dir("./archives")
                                .output()
                                .expect("failed running bash shell");
    return output.status.success();
}

fn build_challenge(challenge_filename: &String) -> Result<Vec<u8>, Error> {

    let build_path = format!("./archives/{}/chall", &challenge_filename);

    let output = Command::new("docker")
                                .args(["build", "-q", "."])
                                .current_dir(format!("{}", build_path))
                                .output()
                                .expect("failed running bash shell");

    if !output.status.success() {
        return Err(Error::BuildFail(String::from_utf8(output.stderr).unwrap()))
    }
    Ok(output.stdout)
}

fn deploy_challenge(challenge_filename: &String, challenge_image: &String, port: u16) -> bool {
    let portmap = format!("{}:5000", port);
    let output = Command::new("docker")
                                .args(["run", "-p", &portmap, "-d", "--name", challenge_filename, "--privileged", challenge_image])
                                .output().map_err(|e| )?;
    return output.status.success();
}

fn generate_challenge_flag(challenge_filename: &String) -> String {
    let flag = format!("coslivectf{{{}}}", Uuid::new_v4());
    let flag_file_path = format!("./archives/{}/chall/dist/flag", challenge_filename);
    let mut flag_file = File::create(flag_file_path).expect("failed creating flag file");
    flag_file.write_all(flag.as_bytes()).unwrap();
    return flag;
}