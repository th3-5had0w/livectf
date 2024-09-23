use actix_web::cookie::time::OffsetDateTime;
use actix_web::http::header;
use regex::Regex;
use std::collections::BTreeMap;

use actix_web::{HttpResponse, web, http::header::ContentType, HttpRequest, cookie::Cookie};
use crate::database::{DbConnection, user::UserInstance, DbFilter};
use crate::web_interface::{JsonResponse, sign_jwt, get_jwt_claims, get_error, success, unauthorized, forbiden};

#[derive(serde::Deserialize)]
pub struct LoginData {
    username: String,
    password: String
}

#[derive(serde::Deserialize)]
pub struct RegisterData {
    username: String,
    password: String,
    email: String
}

pub async fn api_user_login(db_conn: web::Data<DbConnection>, form: web::Form<LoginData>) -> Result<HttpResponse, actix_web::Error> {
    if form.username.len() == 0 || form.password.len() == 0 {
        return Ok(get_error("Missing username/password"));
    } 

    let username = form.username.as_str();
    let password = form.password.as_str();
    
    if password.len() < 6 {
        return Ok(get_error("Password too short"))
    }


    let mut user = db_conn.user_login(
        username,
        password
    ).await;
    
    if user.id == -1 {
        return Ok(forbiden("Login failed"));
    } 

    if user.is_locked {
        let now = chrono::offset::Utc::now().timestamp();
        if user.lock_due_at <= now {
            user.unlock();
        } else {

            return Ok(get_error("Account locked"));
        }
    }

    let jwt: String = sign_jwt(user).unwrap_or(String::new());

    if jwt.len() == 0 {
        return Ok(get_error("Cannot sign JWT"));
    }

    let c = Cookie::build("token", "jfpzihnfiobjgnopka")
        // .domain("localhost")
        .path("/")
        .http_only(true)
        .expires(OffsetDateTime::now_utc())
        .finish();


    let resp = HttpResponse::PermanentRedirect()
        .content_type(ContentType::json())
        .cookie(c)
        .append_header((header::LOCATION, "/"))
        .finish();
    
    return Ok(resp);
}

pub async fn api_user_register(db_conn: web::Data<DbConnection>, form: web::Form<RegisterData>) -> Result<HttpResponse, actix_web::Error> {
    if form.username.len() == 0 || form.password.len() == 0 || form.email.len() == 0 {
        return Ok(get_error("Missing username/password"));
    } 
    let re = Regex::new(r"^[a-zA-Z0-9]+@[a-zA-Z0-9\.]+$").expect("Invalid regex");
    let matches = re.captures(form.email.as_str());

    if matches.is_none() {
        return Ok(get_error("Invalid email"));
    }

    let user = UserInstance::new(form.username.as_str(), form.password.as_str(), form.email.as_str(), false);
    
    let result = db_conn.user_register(user.censor_password(false)).await;
    
    if !result {
        return Ok(get_error("Register failed"));
    } 
    
    let json_resp = JsonResponse {is_error: false, message: "Registered!".to_string()};
    let json_resp = serde_json::to_string(&json_resp).unwrap();
    let resp = HttpResponse::Ok()
        .content_type(ContentType::json())
        .body(json_resp);
    
    return Ok(resp);
}

pub async fn api_user_create(db_conn: web::Data<DbConnection>, req: HttpRequest, form: web::Form<UserInstance>) -> Result<HttpResponse, actix_web::Error> {
    let cookie = req.cookie("auth").unwrap_or(Cookie::build("auth", "").finish());

    let claims: BTreeMap<String, String> = get_jwt_claims(cookie.value()).unwrap_or(BTreeMap::new());

    if claims.len() == 0 {
        return Ok(forbiden("Not authenticated"));
    }

    let is_admin = claims.get("is_admin").unwrap_or(&"false".to_string()).parse::<bool>().unwrap_or(false);
    if !is_admin {
        return Ok(unauthorized("You are not admin"));
    }

    let re = Regex::new(r"^[a-zA-Z0-9]+@[a-zA-Z0-9\.]+$").expect("Invalid regex");
    let matches = re.captures(form.email.as_str());

    if matches.is_none() {
        return Ok(get_error("Invalid email"));
    }

    let result = db_conn.create_user(form.censor_password(false)).await;
    
    if !result {
        return Ok(get_error("Can't create user"));
    } 
    
    return Ok(success("User created!"));
}

pub async fn api_user_edit(db_conn: web::Data<DbConnection>, req: HttpRequest, form: web::Form<UserInstance>) -> Result<HttpResponse, actix_web::Error> {
    let cookie = req.cookie("auth").unwrap_or(Cookie::build("auth", "").finish());

    let claims: BTreeMap<String, String> = get_jwt_claims(cookie.value()).unwrap_or(BTreeMap::new());

    if claims.len() == 0 {
        return Ok(forbiden("Not authenticated"));
    }

    let is_admin = claims.get("is_admin").unwrap_or(&"false".to_string()).parse::<bool>().unwrap_or(false);
    let user_id = claims.get("id").unwrap_or(&"-1".to_string()).parse::<i32>().unwrap_or(-1);

    if !is_admin || form.id != user_id || user_id.is_negative() {
        return Ok(forbiden("You can't edit this user"));
    }

    let re = Regex::new(r"^[a-zA-Z0-9]+@[a-zA-Z0-9\.]+$").expect("Invalid regex");
    let matches = re.captures(form.email.as_str());

    if matches.is_none() {
        return Ok(get_error("Invalid email"));
    }

    let result = db_conn.create_user(form.censor_password(false)).await;
    
    if !result {
        return Ok(get_error("Can't create user"));
    } 
    
    return Ok(success("User created!"));
}

pub async fn api_get_user(db_conn: web::Data<DbConnection>, req: HttpRequest, path: web::Path<(i32,)>) -> Result<HttpResponse, actix_web::Error> {
    let cookie = req.cookie("auth").unwrap_or(Cookie::build("auth", "").finish());

    let claims: BTreeMap<String, String> = get_jwt_claims(cookie.value()).unwrap_or(BTreeMap::new());

    if claims.len() == 0 {
        return Ok(forbiden("Not authenticated"));
    }

    let (user_id_to_get,) = path.into_inner();
    let user_id = claims.get("id").unwrap_or(&"-1".to_string()).parse::<i32>().unwrap_or(-1);
    let mut user = UserInstance::new("", "", "", false);
    let mut should_censor = true;
    if user_id == user_id_to_get || user_id.is_negative() {
        should_censor = false;
    }

    user.id = user_id_to_get;

    let filter_by: Vec<(String, String)> = vec![("id".to_string(), "=".to_string())];
    let filter: DbFilter<UserInstance> = DbFilter::filter_with(user, filter_by);

    let result = db_conn.get_user(filter, should_censor).await;
    
    if result.id == -1 {
        return Ok(get_error("That user does not exist!"));
    } 
    

    let resp = HttpResponse::Ok()
        .content_type(ContentType::json())
        .json(&result.censor_email(should_censor));
    
    return Ok(resp);
}

pub async fn api_delete_user(db_conn: web::Data<DbConnection>, req: HttpRequest, path: web::Path<(i32,)>) -> Result<HttpResponse, actix_web::Error> {
    let cookie = req.cookie("auth").unwrap_or(Cookie::build("auth", "").finish());

    let claims: BTreeMap<String, String> = get_jwt_claims(cookie.value()).unwrap_or(BTreeMap::new());

    if claims.len() == 0 {
        return Ok(forbiden("Not authenticated"));
    }
    
    let is_admin = claims.get("is_admin").unwrap_or(&"false".to_string()).parse::<bool>().unwrap_or(false);
    if !is_admin {
        return Ok(unauthorized("You are not admin"));
    }

    let (user_id_to_del,) = path.into_inner();
    let result = db_conn.delete_user(user_id_to_del).await;
    
    if !result {
        return Ok(get_error("Can't delete user"));
    } 
    
    return Ok(success("User deleted!"));
}

pub async fn api_filter_user(db_conn: web::Data<DbConnection>, req: HttpRequest, query_str: web::Query<DbFilter<UserInstance>>) -> Result<HttpResponse, actix_web::Error> {
    let cookie = req.cookie("auth").unwrap_or(Cookie::build("auth", "").finish());

    let claims: BTreeMap<String, String> = get_jwt_claims(cookie.value()).unwrap_or(BTreeMap::new());

    if claims.len() == 0 {
        return Ok(forbiden("Not authenticated"));
    }

    let user_id = claims.get("id").unwrap_or(&"-1".to_string()).parse::<i32>().unwrap_or(-1);
    let is_admin = claims.get("is_admin").unwrap_or(&"false".to_string()).parse::<bool>().unwrap_or(false);

    if user_id.is_negative() || !is_admin {
        return Ok(unauthorized("unauthorized"));
    }

    let filter: DbFilter<UserInstance> = DbFilter::filter_with(
        query_str.filter_instance().deep_copy(), 
        query_str.filter_by().clone()
    );

    let result = db_conn.filter_user(filter).await;
    let mut final_result: Vec<UserInstance> = vec![];
    for user in &result {
        if user_id != user.id {
            if is_admin {
                final_result.push(user.censor_password(true))
            } else {
                final_result.push(user.censor_password(true).censor_email(true))
            }
        }  else {
            final_result.push(user.deep_copy())
        }
    }

    let resp = HttpResponse::Ok()
        .content_type(ContentType::json())
        .json(&final_result);
    
    return Ok(resp);
}