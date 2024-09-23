use std::fs;
use std::fs::File;
use std::io::Write;

use actix_web::http::header::ContentType;
use actix_web::{HttpResponse, web, HttpRequest, cookie::Cookie};
use base64::Engine;
use std::collections::BTreeMap;
use base64::engine::general_purpose::STANDARD;
use tempdir::TempDir;

use crate::notifier::{NotifierComms, craft_type_notify_message};
use crate::web_interface::{get_jwt_claims, get_error, success, unauthorized, forbiden};
use crate::utils::{is_time_schedule_valid, MAGIC_TIME, is_challenge_exists, check_if_challenge_is_up,read_dir_to_decompressed_entries, unpack};

#[derive(serde::Serialize)]
pub struct DecompressedEntry {
    pub filename: String,
    pub is_public: bool,
    pub content: Vec<u8>
}

#[derive(serde::Deserialize)]
pub struct DecompressForm {
    pub data: String
}

pub async fn api_challenge_action(slaves: web::Data<NotifierComms>, req: HttpRequest, path: web::Path<(String, String)>) -> Result<HttpResponse, actix_web::Error> {
    let cookie = req.cookie("auth").unwrap_or(Cookie::build("auth", "").finish());

    let claims: BTreeMap<String, String> = get_jwt_claims(cookie.value()).unwrap_or(BTreeMap::new());

    if claims.len() == 0 {
        return Ok(forbiden("Not authenticated"));
    }

    let is_admin = claims.get("is_admin").unwrap_or(&"false".to_string()).parse::<bool>().unwrap_or(false);
    if !is_admin {
        return Ok(unauthorized("You are not admin"));
    }

    let challenge_name = &path.0;
    let action = &path.1;

    match action.as_str() {
        "deploy" => {
            if !is_challenge_exists(&challenge_name) {
                return Ok(get_error("Challenge does not exist"));
            } else if check_if_challenge_is_up(&challenge_name) {
                return Ok(get_error("Challenge already started"));
            }

            let start_time_header = req.headers().get("X-start");
            let end_time_header = req.headers().get("X-end");

            let start_time = match start_time_header {
                Some(time) => match i128::from_str_radix(time.to_str().unwrap(), 10) {
                    Ok(epoch) => epoch-MAGIC_TIME,
                    Err(_) => return Ok(get_error("Invalid start time"))
                },
                None => return Ok(get_error("Missing start time"))
            };

            let end_time = match end_time_header {
                Some(time) => match i128::from_str_radix(time.to_str().unwrap(), 10) {
                    Ok(epoch) => epoch-MAGIC_TIME,
                    Err(_) => return Ok(get_error("Invalid end time"))
                },
                None => return Ok(get_error("Missing end time"))
            };

            if !is_time_schedule_valid(start_time, end_time) {
                return Ok(get_error("Please adjust start_time/end_time"));
            }

            let target_module = String::from("deployer");
            let data = craft_type_notify_message(&target_module, &["schedule", challenge_name, &start_time.to_string(), &end_time.to_string()]);
            slaves.notify(target_module, data);
            return Ok(success("Challenge scheduled"))

        },
        "destroy" => {
            if !is_challenge_exists(challenge_name) {
                return Ok(get_error("Challenge does not exist"));
            } else if !check_if_challenge_is_up(&challenge_name) {
                return Ok(get_error("Challenge is not started"));
            }

            let target_module = String::from("deployer");
            let data = craft_type_notify_message(&target_module, &["destroy", &challenge_name]);
            slaves.notify(target_module, data);

            return Ok(success("Challenge destroyed"));
        },
        _ => Ok(get_error("Unknown action"))
    }

}

pub async fn api_decompress_challenge(form: web::Form<DecompressForm>) -> Result<HttpResponse, actix_web::Error> {
    let tmp_dir = TempDir::new("livectf").unwrap();
    let data = String::from_utf8(form.data.as_bytes().to_vec()).unwrap_or(String::from(""));

    if data.len() == 0 {
        return Ok(get_error("Invalid tarball data"));
    }

    let decoded = STANDARD.decode(data);

    match decoded {
        Ok(decoded) => {
            let tmp_tarball = tmp_dir.path().to_str().unwrap().to_owned() + &String::from("/a.tar.gz");
            let mut file = File::create(&tmp_tarball).unwrap();

            file.write_all(&decoded).unwrap();
            file.flush().unwrap();
            let tmp_extracted_dir = tmp_dir.path().to_str().unwrap().to_owned() + &String::from("/extracted");
            fs::create_dir(&tmp_extracted_dir).unwrap(); 

            unpack(&tmp_tarball, &tmp_extracted_dir).unwrap();

            let resp_entities: Vec<DecompressedEntry> = read_dir_to_decompressed_entries(fs::read_dir(tmp_extracted_dir.to_owned()).unwrap());
            println!("{}", tmp_extracted_dir);
            
            let resp: HttpResponse = HttpResponse::Ok()
                .content_type(ContentType::json())
                .body(serde_json::to_string::<Vec<DecompressedEntry>>(&resp_entities).unwrap());
            Ok(resp)
        },
        Err(_) => Ok(get_error("Invalid Base64"))
    }

}