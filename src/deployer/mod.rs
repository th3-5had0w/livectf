use std::{fmt::Display, fs::File, io::Write, process::Command, sync::mpsc::{self, Receiver, Sender}, thread::spawn};

use rand::Rng;
use uuid::Uuid;
use tokio::runtime::Runtime;

use crate::{database::DbConnection, notifier::{self, CleanUpCmdArgs, CtrlMsg, DeployCmdArgs, DeployerCommand, DestroyCmdArgs, EnqueueCmdArgs, FlagInfoCmdArgs, NotifierCommInfo, PublicCmdArgs}, Notifier};

#[derive(Clone)]
struct Challenge {
    challenge_name: String,
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
    sender: Sender<CtrlMsg>,
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

pub fn init(notifier: &mut Notifier, my_sender: Sender<CtrlMsg>, db_conn: DbConnection) {
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
        let cmd: DeployerCommand = serde_json::from_slice(
            ctx.listener.recv()
                        .expect("deployer channel communication exited")
                        .as_slice()
        ).expect("deserialize failed");
        
        match cmd {
            DeployerCommand::DeployCmd(args) => if let Err(err) = cmd_deploy(&mut ctx, args) {
                todo!("handle!")
            },
            DeployerCommand::DestroyCmd(args) => if let Err(err) = cmd_destroy(&mut ctx, args) {
                todo!("handle!")
            },
            DeployerCommand::PublicCmd(args) => if let Err(err) = cmd_public(&mut ctx, args) {
                todo!("handle!")
            }
            _ => panic!("unknown cmd")
        }
    }
}

fn cmd_deploy(ctx: &mut DeployerCtx, args: DeployCmdArgs) -> Result<(), Error> {

    let challenge_name = args.challenge_name;

    let start_time = args.start_time;

    let interval = args.interval;

    let pre_announce = args.pre_announce;

    unpack_challenge(&challenge_name)?;
    let flag = generate_challenge_flag(&challenge_name)?;
    let challenge_image = build_challenge(&challenge_name)?;                                                

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

    let container_id = deploy_challenge(&challenge_name, &challenge_image, port)?;

    let msg = CtrlMsg::FlagReceiver(
        notifier::FlagReceiverCommand::FlagInfoCmd(
            FlagInfoCmdArgs {
                challenge_name : challenge_name.clone(),
                flag
            }
        )
    );

    ctx.sender.send(msg).expect("deployer cannot send");

    let conn_string = format!("nc localhost {}", port);
    rt.block_on(ctx.db_conn.set_challenge_connection_string(challenge_name.to_string(), conn_string));
    rt.block_on(ctx.db_conn.set_challenge_running(challenge_name.to_string(), true));

    {

        let msg = CtrlMsg::Timer(
            notifier::TimerCommand::EnqueueCmd(
                EnqueueCmdArgs {
                    challenge_name: challenge_name.clone(),
                    public_time: start_time,
                    interval,
                    pre_announce
                }
            )
        );

        ctx.sender.send(msg).expect("deployer cannot send");
    }

    ctx.running_deployments.push(
        Challenge { 
            challenge_name,
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

fn cmd_destroy(ctx: &mut DeployerCtx, args: DestroyCmdArgs) -> Result<(), Error> {

    let challenge_name = args.challenge_name;

    let challenge = |challenge_name| -> Option<&Challenge> {
        for challenge in &ctx.running_deployments {
            if challenge_name == &challenge.challenge_name {
                return Some(challenge)
            }
        }
        None
    }(&challenge_name);
    if challenge.is_none() {
        return Err(Error::Destroy(
            format!("invalid challenge {}", &challenge_name)
        ));
    }

    let challenge = challenge.unwrap();
    
    if challenge.running {
        set_port_access(PortAccess::Remove(challenge.port))?;
    }

    let rt = Runtime::new().expect("failed creating tokio runtime");
    destroy_challenge(challenge)?;

    let msg = CtrlMsg::FlagReceiver(
        notifier::FlagReceiverCommand::CleanUpCmd(
            CleanUpCmdArgs {
                challenge_name: challenge_name.clone()
            }
        )
    );

    ctx.sender.send(msg).expect("deployer cannot send");

    ctx.running_deployments.retain(|challenge| challenge.challenge_name != challenge_name);
    rt.block_on(ctx.db_conn.set_challenge_running(challenge_name.to_string(), false));
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

fn cmd_public(ctx: &mut DeployerCtx, args: PublicCmdArgs) -> Result<(), Error> {

    let challenge_name = args.challenge_name;

    for challenge in &mut ctx.running_deployments {
        if challenge.challenge_name == challenge_name && !challenge.running {
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