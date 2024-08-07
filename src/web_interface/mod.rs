// use std::{sync::{mpsc::{self, Receiver, Sender}, Arc}, thread::spawn};
use actix_web::{Result as ActixResult, HttpResponse, http::header::ContentType, web};
use maud::{html, Markup};
use jwt::{SignWithKey, VerifyWithKey, Error as JWT_Error};
use hmac::{Hmac, Mac};
use crate::database::user::UserInstance;
use std::collections::BTreeMap;
use sha2::Sha256;
// use futures_util::lock::Mutex;
// use uuid::Uuid;

use crate::database::{DbConnection};

pub mod user;

#[derive(serde::Serialize)]
pub struct JsonResponse {
    is_error: bool,
    message: String
}

#[derive(serde::Deserialize)]
pub struct PaginationQuery {
    path: Option<String>
}

// pub async fn not_found() -> Result<HttpResponse, actix_web::Error> {
//     let resp = HttpResponse::Ok()
//         .content_type(ContentType::plaintext())
//         .body("404 - not found Xd");
    
//     Ok(resp)
// }

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

pub async fn admin_index(page: web::Query<PaginationQuery>, db_conn: web::Data<DbConnection>) -> ActixResult<Markup> {
    let path = page.path.clone().unwrap_or(String::from("/"));
    let mut users: Vec<UserInstance> = vec![];
    if path == "users" {
        users = db_conn.get_all_user().await;
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
                            a href="/sheep_center?path=users" { "Users management" }
                            a href="/sheep_center?path=challenges" { "Challenges" }
                            a href="/sheep_center?path=logs" { "View logs" }
                        }
    
                        div class="main-section" {
                            @if path == "users" {
                                h1 id="section-title" { "User management" }
                                div class="section-wrapper" {
                                    table class="user-table" {
                                        tr {
                                            th { "ID" }
                                            th { "Username" }
                                            th { "Email" }
                                            th { "Role" }
                                            th { "Solved" }
                                            th { "Locked" }
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