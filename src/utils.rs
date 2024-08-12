use std::{process::Command, fs};
use std::time::{self, SystemTime, UNIX_EPOCH};

const MIN_START_TIME: i128 = 60 * 1;
const MAX_START_TIME: i128 = 3600 * 24 * 7;
const MAX_TIME_CHALLENGE: i128 = 60 * 1;
// i'm not sure why but if you subtract the time sent by client by this value, 
// the time will be correct :D
pub const MAGIC_TIME: i128 = 25188;

pub fn check_if_challenge_is_up(challenge_name: &String) -> bool {
    let output = Command::new("docker")
                                .args(["ps"])
                                .output()
                                .expect("failed running bash shell");
    
    if String::from_utf8(output.stdout).unwrap().contains(challenge_name) {
        return true;
    }
    return false;
}

pub fn is_challenge_exists(challenge_name: &String) -> bool {
    let file_entry = fs::read_dir("./archives/").unwrap();

    for entry in file_entry {
        let dir_entry = entry.unwrap();
        let metadata = fs::metadata(dir_entry.path()).unwrap();
        if metadata.is_dir() {
            let name = String::from_utf8(dir_entry.file_name().as_encoded_bytes().to_vec()).unwrap();
            if name == *challenge_name {
                return true;
            }
        }
    } 
    return false;
}

pub fn is_time_schedule_valid(start_time: i128, end_time: i128) -> bool {
    let now_epoch = i128::try_from(
        SystemTime::now().duration_since(UNIX_EPOCH).expect("back to the future!!!").as_secs()
    ).expect("Cannot convert current epoch to i128");

    if start_time < now_epoch + MIN_START_TIME {
        return false;
    }

    if start_time > now_epoch + MAX_START_TIME {
        return false;
    }

    if end_time < start_time + MAX_TIME_CHALLENGE {
        return false;
    }

    return true;
}