use std::{collections::HashMap, fmt::Display, sync::mpsc::{Receiver, Sender}};

#[derive(Clone)]
pub struct NotifierCommInfo {
    pub id: u128,
    pub name: String,
    pub broadcast_channel: Sender<Vec<u8>>,
}

#[derive(Clone)]
pub struct NotifierComms {
    pub comm_infos: Vec<NotifierCommInfo>
}

pub struct Notifier {
    pub listen_master: Receiver<(String, Vec<u8>)>,
    pub slaves: NotifierComms
}

impl NotifierComms {
    pub fn notify(&self, target_module: String, data: Vec<u8>) {
        if let Some(comm_info) = self.comm_infos.iter().find(|&comm| &comm.name == &target_module) {
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
            let (target_module, data) = self.listen_master.recv().expect("notifier dead!!!");
            if let Some(comm_info) = self.slaves.comm_infos.iter().find(|&comm| &comm.name == &target_module) {
                println!("sending signal to {}", &target_module);
                comm_info.broadcast_channel.send(data).expect("notifier failed to broadcast!");
            } else {
                panic!("notifier failure, refering to non-existent module!");
            }
        }
    }
}

pub fn craft_type_notify_message<T: Display>(target_module: &String, args: &[T]) -> Vec<u8> {
    let mut data: HashMap<&str, String> = HashMap::new();
    match target_module.as_str() {

        "deployer" => {
            data.insert("cmd", args[0].to_string());
            match args[0].to_string().as_str() {

                "schedule" => {
                    data.insert("challenge_filename", args[1].to_string());
                    data.insert("start_time", args[2].to_string());
                    data.insert("end_time", args[3].to_string());
                },

                "deploy" => {
                    data.insert("challenge_filename", args[1].to_string());
                },

                "destroy" => {
                    data.insert("challenge_filename", args[1].to_string());
                },

                _ => panic!("unknown command")

            }
            let serialized_data = serde_json::to_vec(&data).expect("failed converting data");
            return serialized_data;
        },


        "flag_receiver" => {
            data.insert("cmd", args[0].to_string());
            match args[0].to_string().as_str() {

                "flag_submit" => {
                    data.insert("flag", args[1].to_string());
                    data.insert("submit_by", args[2].to_string());        
                },

                "flag_info" => {
                    data.insert("challenge_filename", args[1].to_string());
                    data.insert("flag", args[2].to_string());        
                },

                "cleanup" => {
                    data.insert("challenge_filename", args[1].to_string());
                },

                _ => panic!("unknown command")
            }
            let serialized_data = serde_json::to_vec(&data).expect("failed converting data");
            return serialized_data;
        },


        "database" => {
            data.insert("cmd", args[0].to_string());
            data.insert("sender", args[1].to_string());
            data.insert("data", args[2].to_string());
            let serialized_data = serde_json::to_vec(&data).expect("failed converting data");
            return serialized_data;
        },


        "database_response" => {
            data.insert("data", args[0].to_string());
            data.insert("type", "response".to_string());
            data.insert("sender", "database".to_string());
            let serialized_data = serde_json::to_vec(&data).expect("failed converting data");
            return serialized_data;
        },


        "database_error" => {
            data.insert("data", args[0].to_string());
            data.insert("type", "error".to_string());
            data.insert("sender", "database".to_string());
            let serialized_data = serde_json::to_vec(&data).expect("failed converting data");
            return serialized_data;
        },


        "timer" => {
            data.insert("cmd", args[0].to_string());
            match args[0].to_string().as_str() {

                "enqueue" => {
                    data.insert("challenge_name", args[1].to_string());
                    data.insert("start_time", args[2].to_string());
                    data.insert("end_time", args[3].to_string());
                },

                _ => {
                    panic!("unknown command");
                }
            }
            let serialized_data = serde_json::to_vec(&data).expect("failed converting data");
            return serialized_data;
        },

        _ => {
            panic!("unknown module.");
        }
    }
}