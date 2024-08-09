use std::{process::Command, fs};

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