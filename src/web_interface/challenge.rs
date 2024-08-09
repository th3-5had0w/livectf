use actix_web::{HttpResponse, web, HttpRequest, cookie::Cookie};
use std::collections::BTreeMap;

use crate::notifier::{NotifierComms, craft_type_notify_message};
use crate::web_interface::{get_jwt_claims, get_error, success, unauthorized, forbiden};
use crate::utils;


pub async fn api_challenge_action(slaves: web::Data<NotifierComms>, req: HttpRequest, path: web::Path<(String, String)>) -> Result<HttpResponse, actix_web::Error> {
    let cookie = req.cookie("auth").unwrap_or(Cookie::build("auth", "").finish());

    let claims: BTreeMap<String, String> = get_jwt_claims(cookie.value()).unwrap_or(BTreeMap::new());

    if claims.len() == 0 {
        return Ok(forbiden("Not authenticated"));
    }

    let is_admin = claims.get("is_admin").unwrap_or(&"false".to_string()).parse::<bool>().unwrap();
    if is_admin == false {
        return Ok(unauthorized("You are not admin"));
    }

    let challenge_name = &path.0;
    let action = &path.1;

    match action.as_str() {
        "deploy" => {
            if !utils::is_challenge_exists(&challenge_name) {
                return Ok(get_error("Challenge does not exist"));
            } else if utils::check_if_challenge_is_up(&challenge_name) {
                return Ok(get_error("Challenge already started"));
            }

            let start_time_header = req.headers().get("X-start");
    let end_time_header = req.headers().get("X-end");

            let start_time = match start_time_header {
                Some(time) => match i128::from_str_radix(time.to_str().unwrap(), 10) {
                    Ok(epoch) => epoch,
                    Err(_) => return Ok(get_error("Invalid start time"))
                },
                None => return Ok(get_error("Missing start time"))
            };

            let end_time = match end_time_header {
                Some(time) => match i128::from_str_radix(time.to_str().unwrap(), 10) {
                    Ok(epoch) => epoch,
                    Err(_) => return Ok(get_error("Invalid end time"))
                },
                None => return Ok(get_error("Missing end time"))
            };

            let target_module = String::from("deployer");
            let data = craft_type_notify_message(&target_module, &["schedule", challenge_name, &start_time.to_string(), &end_time.to_string()]);
            slaves.notify(target_module, data);
            return Ok(success("Challenge scheduled"))

        },
        "destroy" => {
            if !utils::is_challenge_exists(challenge_name) {
                return Ok(get_error("Challenge does not exist"));
            } else if utils::check_if_challenge_is_up(&challenge_name) {
                return Ok(get_error("Challenge already started"));
            }

            let target_module = String::from("deployer");
            let data = craft_type_notify_message(&target_module, &["destroy", &challenge_name]);
            slaves.notify(target_module, data);
            return Ok(success("Challenge destroyed"))

        },
        _ => Ok(get_error("Unknown action"))
    }

}