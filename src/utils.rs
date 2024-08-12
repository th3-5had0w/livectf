use std::{process::Command, fs};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::database::user::UserInstance;
use crate::database::DbConnection;

#[derive(Clone)]
pub struct ScoreBoardUser {
    pub place: i32,
    pub username: String,
    pub score: u64
}

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

pub async fn get_scoreboard_from_user_vec(db_conn: DbConnection, users: Vec<UserInstance>) -> Vec<ScoreBoardUser> {

    let mut scoreboard_users: Vec<ScoreBoardUser> = vec![];

    for user in users {
        let mut total_score: u64 = 0;
        for chall_name in user.challenge_solved {
            let chall = db_conn.get_challenge_by_name(chall_name).await;
            if chall.running {
                total_score += u64::try_from(chall.score).unwrap();
            }
        }

        scoreboard_users.push(ScoreBoardUser {
            place: 0,
            username: user.username,
            score: total_score
        });
    }

    scoreboard_users.sort_by(|a, b| a.score.cmp(&b.score).reverse());

    let mut i: usize = 1;
    let mut final_scoreboard_users: Vec<ScoreBoardUser> = vec![];
    while i <= scoreboard_users.len() {
        let user = scoreboard_users.get(i-1).unwrap();
        let mut user = user.clone();

        user.place = i32::try_from(i).unwrap();
        final_scoreboard_users.push(user);
        i += 1;
    }

    final_scoreboard_users.to_vec()
}