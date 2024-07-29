use std::{collections::HashMap, fs::File, io::Write, process::Command, str::FromStr, sync::mpsc::{self, Receiver, Sender}, thread::spawn};

use rand::Rng;
use uuid::Uuid;

use crate::{notifier::{self, NotifierCommInfo}, Notifier, database::DbConnection};

struct DeployerCtx {
    // main comm channel
    sender: Sender<(String, Vec<u8>)>,
    listener: Receiver<Vec<u8>>,
    
    db_conn: DbConnection,
    used_port: Vec<u16>,
}

impl DeployerCtx {
    fn is_port_used(&self, port: u16) -> bool {
        return self.used_port.contains(&port);
    }
}

pub fn init(notifier: &mut Notifier, my_sender: Sender<(String, Vec<u8>)>, db_conn: DbConnection) {
    let (notifier_sender, my_receiver) : (Sender<Vec<u8>>, Receiver<Vec<u8>>) = mpsc::channel();
    let ctx = DeployerCtx {
        sender: my_sender,
        listener: my_receiver,
        used_port: Vec::new(),
        db_conn
    };

    
    let comm_info = NotifierCommInfo {
        id: Uuid::new_v4().as_u128(),
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
            "DEPLOY" => cmd_deploy(&mut ctx, &data),
            _ => panic!("unknown cmd")
        }
    }
}

fn cmd_deploy(ctx: &mut DeployerCtx, data: &HashMap<&str, String>) {
    let challenge_filename = data.get("challenge_filename").expect("missing challenge_filename");
        let unpack_success = unpack_challenge(challenge_filename);

        if unpack_success {
            let flag = generate_challenge_flag(challenge_filename);
            let (build_success, challenge_image) = build_challenge(challenge_filename);
            if build_success {
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

                let deploy_success = deploy_challenge(&String::from_utf8(challenge_image).unwrap(), port);
                ctx.used_port.push(port);
                if deploy_success {
                    let target_module = String::from_str("flag_receiver").unwrap();
                    let cmd = String::from_str("flag_info").unwrap();
                    let data = notifier::craft_type_notify_message(&target_module, &[cmd, challenge_filename.to_string(), flag, "".to_string()]);
                    ctx.sender.send((target_module, data)).expect("deployer cannot send");
                } else {
                    println!("deploy failed");
                }

            } else {
                println!("build failed");
            }

        } else {
            println!("unpack failed");
        }
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

fn build_challenge(challenge_filename: &String) -> (bool, Vec<u8>) {
    let build_path = format!("./archives/{}/chall", &challenge_filename);
    let output = Command::new("docker")
                                .args(["build", "-q", "."])
                                .current_dir(format!("{}", build_path))
                                .output()
                                .expect("failed running bash shell");
    return (output.status.success(), output.stdout);
}

fn deploy_challenge(challenge_image: &String, port: u16) -> bool {
    let portmap = format!("{}:5000", port);
    let challenge_image_name = &challenge_image.trim().replace("sha256:", "");
    let output = Command::new("docker")
                                .args(["run", "-p", &portmap, "-d", "--privileged", challenge_image_name])
                                .output()
                                .expect("failed running bash shell");
    
    println!("{} {} {}", challenge_image_name, portmap ,String::from_utf8(output.stderr).expect("msg"));
    return output.status.success();
}

fn generate_challenge_flag(challenge_filename: &String) -> String {
    let flag = format!("coslivectf{{{}}}", Uuid::new_v4());
    let flag_file_path = format!("./archives/{}/chall/dist/flag", challenge_filename);
    let mut flag_file = File::create(flag_file_path).expect("failed creating flag file");
    flag_file.write_all(flag.as_bytes()).unwrap();
    return flag;
}