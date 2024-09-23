use std::{collections::HashMap, fmt::Display, fs::File, io::Write, process::Command, str::FromStr, sync::mpsc::{self, Receiver, Sender}, thread::spawn};

use rand::Rng;
use uuid::Uuid;
use tokio::runtime::Runtime;

use crate::{database::DbConnection, notifier::{self, craft_type_notify_message, NotifierCommInfo}, Notifier};

#[derive(Clone)]
struct Challenge {
    challenge_filename: String,
    challenge_image: String,
    container_id: String,
    port: u16,
    running: bool
}

#[derive(Debug)]
pub enum Error {
    Build(String),
    Unpack(String),
    GenerateFlag(String),
    Deploy(String),
    Destroy(String),
    Firewall(String),
    PublicChallenge(String)
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Build(err) => write!(f, "Deployer - BuildFail: {}", err),
            Error::Unpack(err) => write!(f, "Deployer - UnpackFail: {}", err),
            Error::GenerateFlag(err) => write!(f, "Deployer - GenerateFlagFail: {}", err),
            Error::Deploy(err) => write!(f, "Deployer - DeployFail: {}", err),
            Error::Destroy(err) => write!(f, "Deployer - DestroyFail: {}", err),
            Error::Firewall(err) => write!(f, "Deployer - FirewallConfigFail: {}", err),
            Error::PublicChallenge(err) => write!(f, "Deployer - PublicChallengeFail: {}", err)
        }
    }
}

impl std::error::Error for Error {}

struct DeployerCtx {
    // main comm channel
    sender: Sender<(String, Vec<u8>)>,
    listener: Receiver<Vec<u8>>,
    
    db_conn: DbConnection,
    running_deployments: Vec<Challenge>
}

impl DeployerCtx {
    fn is_port_used(&self, port: u16) -> bool {
        for challenge in &self.running_deployments {
            if challenge.port == port {
                return true;
            }
        }
        return false;
    }
}

pub fn init(notifier: &mut Notifier, my_sender: Sender<(String, Vec<u8>)>, db_conn: DbConnection) {
    let (notifier_sender, my_receiver) : (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel();
    let ctx = DeployerCtx {
        sender: my_sender,
        listener: my_receiver,
        db_conn,
        running_deployments: Vec::new(),
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
            "deploy" => if let Err(err) = cmd_deploy(&mut ctx, &data) {
                todo!("handle!")
            },
            "destroy" => if let Err(err) = cmd_destroy(&mut ctx, &data) {
                todo!("handle!")
            },
            "public" => if let Err(err) = cmd_public(&mut ctx, &data) {
                todo!("handle!")
            }
            _ => panic!("unknown cmd")
        }
    }
}

fn cmd_deploy(ctx: &mut DeployerCtx, data: &HashMap<&str, String>) -> Result<(), Error> {

    let challenge_filename = data.get("challenge_filename")
                                                .ok_or(Error::Deploy(
                                                    String::from("invalid challenge filename")
                                                ))?.to_owned();

    let start_time = data.get("start_time")
                                                .ok_or(Error::Deploy(
                                                    String::from("invalid start time")
                                                ))?;

    let interval = data.get("interval")
                                                .ok_or(Error::Deploy(
                                                    String::from("invalid interval")
                                                ))?;

    unpack_challenge(&challenge_filename)?;
    let flag = generate_challenge_flag(&challenge_filename)?;
    let challenge_image = build_challenge(&challenge_filename)?;                                                

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

    let container_id = deploy_challenge(&challenge_filename, &challenge_image, port)?;
    let target_module = String::from("flag_receiver");
    let cmd = String::from("flag_info");
    let data = notifier::craft_type_notify_message(&target_module, &[&cmd, &challenge_filename.to_string(), &flag]);
    ctx.sender.send((target_module, data)).expect("deployer cannot send");
    let conn_string = format!("nc localhost {}", port);
    rt.block_on(ctx.db_conn.set_challenge_connection_string(challenge_filename.to_string(), conn_string));
    rt.block_on(ctx.db_conn.set_challenge_running(challenge_filename.to_string(), true));

    {
        let target_module = String::from("timer");
        let data = craft_type_notify_message(&target_module, &["enqueue", &challenge_filename.to_string(), &start_time.to_string(), &interval.to_string()]);
        ctx.sender.send((target_module, data)).expect("deployer cannot send");
    }

    ctx.running_deployments.push(
        Challenge { 
            challenge_filename,
            challenge_image,
            container_id,
            port,
            running: false
        }
    );

    Ok(())
}

enum PortAccess {
    Add(u16),
    Remove(u16)
}

fn set_port_access(status: PortAccess) -> Result<(), Error> {
    match status {
        PortAccess::Add(port) => {
            let output = Command::new("firewall-cmd")
                                        .args([format!("--add-port={}/tcp", port)])
                                        .output()
                                        .expect("failed running bash shell");
            
            if !output.status.success() {
                return Err(Error::Firewall(format!("allow {}", port)));
            }
        }
        PortAccess::Remove(port) => {
            let output = Command::new("firewall-cmd")
                                        .args([format!("--remove-port={}/tcp", port)])
                                        .output()
                                        .expect("failed running bash shell");
            
            if !output.status.success() {
                return Err(Error::Firewall(format!("remove {}", port)));
            }
        }
    }
    Ok(())
}

fn cmd_destroy(ctx: &mut DeployerCtx, data: &HashMap<&str, String>) -> Result<(), Error> {

    let challenge_filename = data.get("challenge_filename")
                                        .ok_or(Error::Deploy(
                                            String::from("invalid challenge filename")
                                        ))?.to_owned();

    let challenge = |challenge_name| -> Option<&Challenge> {
        for challenge in &ctx.running_deployments {
            if challenge_name == &challenge.challenge_filename {
                return Some(challenge)
            }
        }
        None
    }(&challenge_filename);
    if challenge.is_none() {
        return Err(Error::Destroy(
            format!("invalid challenge {}", &challenge_filename)
        ));
    }

    let challenge = challenge.unwrap();
    
    if challenge.running {
        set_port_access(PortAccess::Remove(challenge.port))?;
    }

    let rt = Runtime::new().expect("failed creating tokio runtime");
    destroy_challenge(challenge)?;

    let target_module = String::from("flag_receiver");
    let data = notifier::craft_type_notify_message(&target_module, &["cleanup", &challenge_filename]);

    ctx.sender.send((target_module, data)).expect("deployer cannot send");

    ctx.running_deployments.retain(|challenge| challenge.challenge_filename != challenge_filename);
    rt.block_on(ctx.db_conn.set_challenge_running(challenge_filename.to_string(), false));
    Ok(())
}

fn destroy_challenge(challenge: &Challenge) -> Result<(), Error> {
    let output = Command::new("docker")
                                .args(["container", "rm", "-f", &challenge.container_id])
                                .output()
                                .expect("failed running bash shell");

    if !output.status.success() {
        return Err(Error::Destroy(
            String::from_utf8(output.stderr).unwrap()
        ))
    }

    let output = Command::new("docker")
                                .args(["image", "rm", "-f", &challenge.challenge_image])
                                .output()
                                .expect("failed running bash shell");

    if !output.status.success() {
        return Err(Error::Destroy(
            String::from_utf8(output.stderr).unwrap()
        ))
    }

    Ok(())
}

fn deserialize_data(serialized_data: &Vec<u8>) -> HashMap<&str, String> {
    let data: HashMap<&str, String> = serde_json::from_slice(serialized_data.as_slice()).expect("deserialize failed!");
    return data;
}

fn unpack_challenge(challenge_filename: &String) -> Result<(), Error> {
    let output = Command::new("tar")
                                .args(["-xf", &format!("{}.tar.gz", challenge_filename), "--one-top-level"])
                                .current_dir("./archives")
                                .output()
                                .expect("failed running bash shell");

    if !output.status.success() {
        return Err(Error::Unpack(
            String::from_utf8(output.stderr).unwrap()
        ))
    }

    Ok(())
}

fn build_challenge(challenge_filename: &String) -> Result<String, Error> {

    let build_path = format!("./archives/{}/chall", &challenge_filename);

    let output = Command::new("docker")
                                .args(["build", "-q", "."])
                                .current_dir(format!("{}", build_path))
                                .output()
                                .expect("failed running bash shell");

    if !output.status.success() {
        return Err(Error::Build(
            String::from_utf8(output.stderr).unwrap()
        ))
    }
    
    let image_hash = String::from_utf8(output.stdout)
                                .map_err(|e| Error::Build(format!("{}", e)))?
                                .trim().replace("sha256:", "");
    Ok(image_hash)
}

fn deploy_challenge(challenge_filename: &String, challenge_image: &String, port: u16) -> Result<String, Error> {
    let portmap = format!("{}:5000", port);
    let output = Command::new("docker")
                                .args(["run", "-p", &portmap, "-d", "--name", challenge_filename, "--privileged", challenge_image])
                                .output()
                                .expect("failed running bash shell");
    
    if !output.status.success() {
        return Err(
            Error::Deploy(
                String::from_utf8(output.stderr).unwrap()
            )
        )
    }

    let container_id = String::from_utf8(output.stdout)
                                                            .map_err(|e| Error::Deploy(format!("{}", e)))?;

    Ok(container_id)
}

fn cmd_public(ctx: &mut DeployerCtx, data: &HashMap<&str, String>) -> Result<(), Error> {

    let challenge_name = data.get("challenge_filename")
                                        .ok_or(Error::PublicChallenge(
                                            String::from("invalid challenge filename")
                                        ))?.to_owned();


    for challenge in &mut ctx.running_deployments {
        if challenge.challenge_filename == challenge_name && !challenge.running {
            set_port_access(PortAccess::Add(challenge.port))?;
            challenge.running = true;
        }
    }
    Ok(())
}

fn generate_challenge_flag(challenge_filename: &String) -> Result<String, Error> {
    let flag = format!("coslivectf{{{}}}", Uuid::new_v4());
    let flag_file_path = format!("./archives/{}/chall/dist/flag", challenge_filename);

    let mut flag_file = File::create(flag_file_path)
                                    .map_err(|e| Error::GenerateFlag(format!("{}", e)))?;

    flag_file.write_all(flag.as_bytes()).map_err(|e| Error::GenerateFlag(format!("{}", e)))?;

    Ok(flag)
}