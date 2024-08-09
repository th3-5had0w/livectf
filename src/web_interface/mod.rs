// use std::{sync::{mpsc::{self, Receiver, Sender}, Arc}, thread::spawn};
use actix_web::{http::header::ContentType, web, HttpResponse, Result as ActixResult, cookie::Cookie, HttpRequest};
use maud::{html, Markup};
use jwt::{SignWithKey, VerifyWithKey, Error as JWT_Error};
use hmac::{Hmac, Mac};
use crate::database::{solve_history::SolveHistoryEntry, user::UserInstance, DbFilter};
use std::{collections::BTreeMap, os::unix::fs::MetadataExt, vec};
use sha2::Sha256;
use std::fs;
use chrono::{DateTime, offset::Utc};
// use futures_util::lock::Mutex;
// use uuid::Uuid;

use crate::{database::DbConnection, utils};

pub mod user;
pub mod challenge;

const USER_PATH: &str = "users";
const SOLVE_LOG_PATH: &str = "solve-logs";
const CHALLENGE_PATH: &str = "challenges";

#[derive(serde::Serialize)]
pub struct JsonResponse {
    is_error: bool,
    message: String
}

#[derive(serde::Deserialize)]
pub struct PaginationQuery {
    path: Option<String>
}

// TODO: randomize this, store in env
const SECRET_KEY: &str = "SUPER_FUCKING_SECURE";

pub fn sign_jwt(user: UserInstance) -> String {
    let key: Hmac<Sha256> = Hmac::new_from_slice(SECRET_KEY.as_bytes()).unwrap();
    let mut claims = BTreeMap::new();

    let id = user.id().to_string();
    let is_admin = user.is_admin().to_string();
    let challenge_solved = user.challenge_solved().to_string();

    claims.insert("id", id.as_str());
    claims.insert("username", user.username());
    claims.insert("email", user.email());
    claims.insert("is_admin", &is_admin);
    claims.insert("challenge_solved", &challenge_solved);

    let token_str = claims.sign_with_key(&key).expect("jwt signing failed");

    return token_str;
}

pub fn get_jwt_claims (token: &str) -> Result<BTreeMap<String, String>, JWT_Error>{
    let key: Hmac<Sha256> = Hmac::new_from_slice(SECRET_KEY.as_bytes())?;
    let claims: BTreeMap<String, String> = token.verify_with_key(&key)?;

    Ok(claims)
}

pub fn get_error(msg: &str) -> HttpResponse {
    let json_resp = JsonResponse {is_error: true, message: msg.to_string()};
    let json_resp = serde_json::to_string(&json_resp).unwrap();
    let resp = HttpResponse::BadRequest()
        .content_type(ContentType::json())
        .body(json_resp);
    
    return resp;
}

pub fn forbiden(msg: &str) -> HttpResponse {
    let json_resp = JsonResponse {is_error: true, message: msg.to_string()};
    let json_resp = serde_json::to_string(&json_resp).unwrap();
    let resp = HttpResponse::Forbidden()
        .content_type(ContentType::json())
        .body(json_resp);
    
    return resp;
}

pub fn unauthorized(msg: &str) -> HttpResponse {
    let json_resp = JsonResponse {is_error: true, message: msg.to_string()};
    let json_resp = serde_json::to_string(&json_resp).unwrap();
    let resp = HttpResponse::Unauthorized()
        .content_type(ContentType::json())
        .body(json_resp);
    
    return resp;
}

pub fn success(msg: &str) -> HttpResponse {
    let json_resp = JsonResponse {is_error: false, message: msg.to_string()};
    let json_resp = serde_json::to_string(&json_resp).unwrap();
    let resp = HttpResponse::Ok()
        .content_type(ContentType::json())
        .body(json_resp);
    
    return resp;
}

pub async fn login() -> ActixResult<Markup> {
    Ok(html! {
        html {
            head {
                link rel="stylesheet" href="/static/css/styles.css" {}
                link rel="stylesheet" href="/static/css/styles_login.css" {}
                meta charset="utf-8" {}
                title {
                    "CoSGang livectf"
                }
            }
            body {
                div class="container" {
                    img src="/static/img/cosgang.jpg" id="cosgang-avt" {}
                    div class="form-login-container" {
                        h1 { "Login" }
                        form action="/api/login" method="POST" {
                            input name="username" type="text" placeholder="Username..." {}
                            input name="password" type="password" placeholder="Password..." {}
                            input name="login" type="submit" value="Login" {}
                        }
                    }
                }
            }
            script src="/static/js/login.js" {}
        }
    })
}

pub async fn register() -> ActixResult<Markup> {
    Ok(html! {
        html {
            head {
                link rel="stylesheet" href="/static/css/styles.css" {}
                link rel="stylesheet" href="/static/css/styles_register.css" {}
                meta charset="utf-8" {}
                title {
                    "CoSGang livectf"
                }
            }
            body {
                div class="container" {
                    img src="/static/img/cosgang.jpg" id="cosgang-avt" {}
                    div class="form-reg-container" {
                        h1 { "Register" }
                        form action="/api/register" method="POST" {
                            input name="email" type="text" placeholder="Email..." {}
                            input name="username" type="text" placeholder="Username..." {}
                            input name="password" type="password" placeholder="Password..." {}
                            input name="register" type="submit" value="Register" {}
                        }
                    }
                }
            }
            script src="/static/js/register.js" {}
        }
    })
}

pub async fn not_found() -> ActixResult<Markup> {
    Ok(html! {
        html {
            head {
                link rel="stylesheet" href="/static/css/styles.css" {}
                link rel="stylesheet" href="/static/css/styles_404.css" {}
                meta charset="utf-8" {}
                title {
                    "CoSGang livectf"
                }
            }
            body {
                div class="container" {
                    img src="/static/img/cosgang.jpg" id="cosgang-avt" {}
                    h1 { "Lost? Let our sheeps take you home" }
                    a href="/" { "< Back" }
                }
            }
        }
    })
}

pub async fn index() -> ActixResult<Markup> {
    Ok(html! {
        html {
            head {
                link rel="stylesheet" href="/static/css/styles.css" {}
                link rel="stylesheet" href="/static/css/styles_index.css" {}
                meta charset="utf-8" {}
                title {
                    "CoSGang livectf"
                }
            }
            body {
                div class="container" {
                    img src="/static/img/cosgang.jpg" id="cosgang-avt" {}
                    h1 { "Under construction! Stay turned hackers." }
                }
            }
        }
    })
}

// TODO: add advanced search and delete feature to solve logs
pub async fn admin_index(page: web::Query<PaginationQuery>, db_conn: web::Data<DbConnection>, req: HttpRequest) -> ActixResult<Markup> {
    let path = page.path.clone().unwrap_or(String::from("/"));
    let mut users: Vec<UserInstance> = vec![];
    let mut solve_logs: Vec<SolveHistoryEntry> = vec![];
    let mut challenges: Vec<(String, DateTime<Utc>, usize, usize, bool)> = vec![];
    
    let cookie: Cookie<'_> = req.cookie("auth").unwrap_or(Cookie::build("auth", "").finish());

    let claims: BTreeMap<String, String> = get_jwt_claims(cookie.value()).unwrap_or(BTreeMap::new());

    if claims.len() == 0 {
        return Ok(html!(
            script {
                "location.href = '/';"
            }
        ));
    }

    let is_admin = claims.get("is_admin").unwrap_or(&"false".to_string()).parse::<bool>().unwrap();
    if is_admin == false {
        return Ok(html!(
            script {
                "location.href = '/';"
            }
        ));
    }

    if path == USER_PATH {
        users = db_conn.get_all_user().await;
    } else if path == SOLVE_LOG_PATH {
        solve_logs = db_conn.fetch_recent_solve_log(20).await;
    } else if path == CHALLENGE_PATH {
        let file_entry = fs::read_dir("./archives/").unwrap();
        for entry in file_entry {
            let dir_entry = entry.unwrap();
            let metadata = fs::metadata(dir_entry.path()).unwrap();
            if metadata.is_dir() {
                let challenge_name = String::from_utf8(dir_entry.file_name().as_encoded_bytes().to_vec()).unwrap();
                let creation_time = DateTime::from_timestamp(metadata.ctime(), 0).unwrap();
                let filter = DbFilter::filter_with(SolveHistoryEntry::new(
                    String::from("test"),
                    challenge_name.clone(),
                    true,
                    String::from("test")
                ), vec![
                    (String::from("challenge_name"), String::from("=")),
                    (String::from("is_success"), String::from("="))
                ]);
               
                let solve_count = db_conn.filter_solve_log(filter, -1).await.len();

                let filter = DbFilter::filter_with(SolveHistoryEntry::new(
                    String::from("test"),
                    challenge_name.clone(),
                    false,
                    String::from("test")
                ), vec![
                    (String::from("challenge_name"), String::from("="))
                ]);

                let submission_count = db_conn.filter_solve_log(filter, -1).await.len();
                let is_up = utils::check_if_challenge_is_up(&challenge_name);
                challenges.push((challenge_name, creation_time, solve_count, submission_count, is_up));
            }
        }
    }

    Ok(html! {
        html {
            head {
                link rel="stylesheet" href="/static/css/styles.css" {}
                link rel="stylesheet" href="/static/css/styles_sheep_center.css" {}
                meta charset="utf-8" {}
                title {
                    "CoSGang livectf - Dashboard"
                }
            }
            body {
                div class="container" {
                    div class="wrapper" {
                        div class="menu-wrapper" {
                            a href=(format!("/sheep_center?path={}", USER_PATH)) { "Users management" }
                            a href="/sheep_center?path=challenges" { "Challenges" }
                            a href=(format!("/sheep_center?path={}", SOLVE_LOG_PATH)) { "Solve logs" }
                        }
    
                        div class="main-section" {
                            @if path == USER_PATH {
                                h1 id="section-title" { "User management" }
                                div class="section-wrapper" {
                                    table class="the-table" {
                                        tr {
                                            th { "ID" }
                                            th { "Username" }
                                            th { "Email" }
                                            th { "Role" }
                                            th { "Solved" }
                                            th { "Locked" }
                                            th { "Action" }
                                        }
                                        @for user in users {
                                            tr {
                                                td { (user.id()) }
                                                td { (user.username()) }
                                                td { (user.email()) }
                                                @if user.is_admin() {
                                                    td { "admin" }
                                                } @else {
                                                    td { "user" }
                                                }
                                                td { (user.challenge_solved()) }
                                                td { (user.is_locked()) }
                                                td { 
                                                    div class="action-btn-wrapper" {
                                                        button data-userid=(user.id()) class="del-btn" {
                                                            "🗑️"
                                                        }

                                                        button data-userid=(user.id()) class="ban-btn" {
                                                            "⛔"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            } @else if path == SOLVE_LOG_PATH {
                                h1 id="section-title" { "Solve log" }
                                div class="section-wrapper" {
                                    table class="the-table" {
                                        tr {
                                            th { "ID" }
                                            th { "Challenge" }
                                            th { "Username" }
                                            th { "Result" }
                                            th { "Time" }
                                            th { "Flag" }
                                        }
                                        @for log in solve_logs {
                                            tr {
                                                td { (log.id()) }
                                                td { (log.challenge_name()) }
                                                td { (log.username()) }
                                                @match log.is_success() {
                                                    true => td class="success-submission" {
                                                        "Success"
                                                    },
                                                    _ => td class="fail-submission" {
                                                        "Failed"
                                                    }
                                                }
                                                td { (log.time()) }
                                                td { (log.submit_content()) }
                                            }
                                        }
                                    }
                                }
                            } @else if path == CHALLENGE_PATH {
                                h1 id="section-title" { "Challenges" }
                                div class="form-wrapper" {
                                    form class="challenge-upload-form" method="post" enctype="multipart/form-data" {
                                        input type="date" name="start-date" id="start-date" {}
                                        input type="time" name="start-time" id="start-time" {}
                                        input type="date" name="end-date" id="end-date" {}
                                        input type="time" name="end-time" id="end-time" {}
                                        input type="file" name="challenge-file" id="fileToUpload" accept=".tar.gz" {}
                                        button id="upload-challenge" { "upload" }
                                    }
    
                                    form class="challenge-schedule-form" method="post" {
                                        select name="challenge-name" id="challenge-name" {
                                            @for chall in challenges.clone() {
                                                option value=(chall.0) { (chall.0) }
                                            }
                                        }
                                        input type="date" name="start-date2" id="start-date2" {}
                                        input type="time" name="start-time" id="start-time2" {}
                                        input type="date" name="end-date2" id="end-date2" {}
                                        input type="time" name="end-time2" id="end-time2" {}
                                        button id="schedule-challenge" { "schedule" }
                                    }
                                }

                                div class="section-wrapper" {
                                    table class="the-table" {
                                        tr {
                                            th { "Challenge ID" }
                                            th { "Upload Time" }
                                            th { "Solved" }
                                            th { "Submission" }
                                            th { "Up" }
                                            th { "Action" }
                                        }
                                        @for chall in challenges {
                                            tr {
                                                td { (chall.0) }
                                                td { (chall.1) }
                                                td { (chall.2) }
                                                td { (chall.3) }
                                                @if chall.4 {
                                                    td { "🟢" }
                                                    div class="action-btn-wrapper" {
                                                        button data-challengeId=(chall.0) class="stop-btn" {
                                                            "Stop"
                                                        }
                                                    }
                                                } @else {
                                                    td { "🔴" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            script src="/static/js/sheep_center.js" {}
        }
    })
}