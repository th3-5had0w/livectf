use std::sync::mpsc::{Receiver, Sender};

use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct NotifierCommInfo {
    // pub id: u128,
    pub name: String,
    pub broadcast_channel: Sender<Vec<u8>>,
}

#[derive(Clone)]
pub struct NotifierComms {
    pub comm_infos: Vec<NotifierCommInfo>
}

pub struct Notifier {
    pub listen_master: Receiver<CtrlMsg>,
    pub slaves: NotifierComms
}

impl NotifierComms {
    pub fn notify(&self, msg: CtrlMsg) {
        let target_module: String;
        match &msg {
            CtrlMsg::Deployer(_) => target_module = String::from("deployer"),
            CtrlMsg::FlagReceiver(_) => target_module = String::from("flag_receiver"),
            CtrlMsg::Timer(_) => target_module = String::from("timer"),
        }
        let data = msg.craft_and_send();
        if let Some(comm_info) = self.comm_infos.iter().find(|&comm| comm.name == target_module) {
            comm_info.broadcast_channel.send(data).expect("notifier failed to broadcast!");
        } else {
            panic!("notifier failure, refering to non-existent module!");
        }
    }
}

// main functions
impl Notifier {
    pub fn run(&self) {
        loop {
            let msg = self.listen_master.recv().expect("notifier dead!!!");

            let target_module: String;
            match &msg {
                CtrlMsg::Deployer(_) => target_module = String::from("deployer"),
                CtrlMsg::FlagReceiver(_) => target_module = String::from("flag_receiver"),
                CtrlMsg::Timer(_) => target_module = String::from("timer"),
            }
            let data = msg.craft_and_send();

            if let Some(comm_info) = self.slaves.comm_infos.iter().find(|&comm| comm.name == target_module) {
                println!("sending signal to {}", &target_module);
                comm_info.broadcast_channel.send(data).expect("notifier failed to broadcast!");
            } else {
                panic!("notifier failure, refering to non-existent module!");
            }
        }
    }
}

pub trait MsgMethod {
    fn craft_and_send(self) -> Vec<u8>;
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeployCmdArgs {
    pub challenge_name: String,
    pub start_time: i128,
    pub interval: i128,
    pub pre_announce: i128
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DestroyCmdArgs {
    pub challenge_name: String
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicCmdArgs {
    pub challenge_name: String
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeployerCommand {
    DeployCmd(DeployCmdArgs),
    DestroyCmd(DestroyCmdArgs),
    PublicCmd(PublicCmdArgs)
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CleanUpCmdArgs {
    pub challenge_name: String
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlagSubmitCmdArgs {
    pub flag: String,
    pub submit_by: String
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlagInfoCmdArgs {
    pub challenge_name: String,
    pub flag: String
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FlagReceiverCommand {
    CleanUpCmd(CleanUpCmdArgs),
    FlagSubmitCmd(FlagSubmitCmdArgs),
    FlagInfoCmd(FlagInfoCmdArgs)
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnqueueCmdArgs {
    pub challenge_name: String,
    pub public_time: i128,
    pub interval: i128,
    pub pre_announce: i128
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeployInfoCmdArgs {
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimerCommand {
    EnqueueCmd(EnqueueCmdArgs),
    DeployInfoCmd(DeployInfoCmdArgs)
}

#[derive(PartialEq, Eq)]
pub enum CtrlMsg {
    Deployer(DeployerCommand),
    FlagReceiver(FlagReceiverCommand),
    Timer(TimerCommand)
}

impl MsgMethod for CtrlMsg {
    fn craft_and_send(self) -> Vec<u8> {
        let serialized: Result<Vec<u8>, serde_json::Error>;
        match self {
            Self::Deployer(cmd) => {
                match &cmd {
                    DeployerCommand::DeployCmd(_args) => {
                        serialized = serde_json::to_vec(&cmd);
                    },
                    DeployerCommand::DestroyCmd(_args) => {
                        serialized = serde_json::to_vec(&cmd);
                    },
                    DeployerCommand::PublicCmd(_args) => {
                        serialized = serde_json::to_vec(&cmd);
                    },
                }
            },
            Self::Timer(cmd) => {
                match &cmd {
                    TimerCommand::DeployInfoCmd(_args) => {
                        serialized = serde_json::to_vec(&cmd);
                    },
                    TimerCommand::EnqueueCmd(_args) => {
                        serialized = serde_json::to_vec(&cmd);
                    },
                }
            },
            Self::FlagReceiver(cmd) => {
                match &cmd {
                    FlagReceiverCommand::CleanUpCmd(_args) => {
                        serialized = serde_json::to_vec(&cmd);
                    },
                    FlagReceiverCommand::FlagInfoCmd(_args) => {
                        serialized = serde_json::to_vec(&cmd);
                    },
                    FlagReceiverCommand::FlagSubmitCmd(_args) => {
                        serialized = serde_json::to_vec(&cmd);
                    },
                }
            },
        }
        let serialized = serialized.expect("failed");
        return serialized;
    }
}