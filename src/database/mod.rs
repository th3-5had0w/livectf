use sqlx::postgres::{PgPoolOptions, Postgres};
use std::thread;
use sqlx::pool::Pool;
use uuid::Uuid;
use std::future::Future;
use std::sync::mpsc::{self, Receiver, Sender};
use std::collections::HashMap;
use std::clone::Clone;


use crate::{Notifier, NotifierCommInfo, notifier};

pub mod user;
pub mod deploy_log;
pub mod solve_history;

// TODO: change TEXT to VARCHAR as TEXT is slow
// remember to change this to a .env file, the credentials should be stored in environment variable rather than hard-coded
const DB_HOST: &str = "localhost";
const DB_USERNAME: &str = "test";
const DB_PASSWORD: &str = "WisHBrAdhOtalMaNOste";
const DB_DATABASE_NAME: &str = "livectf";
const DB_POOL_MAX_CONNECTION: u32 = 5;

const DB_DEPLOY_LOG_TABLE: &str = "depoy_log";
const DB_USER_TABLE: &str = "users";
const DB_SOLVE_HISTORY_TABLE: &str = "solve_history";

// async fn database_loop(ctx: DatabaseReceiverCtx) {
//     loop {
//         let serialized_data = ctx.listener.recv().expect("deployer channel communication exited");
//         println!("db received smth");
//         let data = deserialize_data(&serialized_data);
//         let orig_sender = data.get("sender").unwrap().to_string();
//         let response = "database_response".to_string();
//         let error = "database_error".to_string();

//         match data.get("cmd").unwrap().as_str() {
//             "fetch_recent_deploy_log" => {
//                 let result = ctx.database_connection.fetch_recent_deploy_log(
//                     data.get("data").expect("`id` not present").trim().parse::<u32>().expect("`id` must be of type `i32`")
//                 ).await;
                
//                 ctx.sender.send((orig_sender, notifier::craft_type_notify_message(&response, &[serialize_data(&result)]))).expect("database cannot send");
//             },
//             "filter_deploy_log" => {
//                 let data_data = data.get("data").expect("no data found").as_bytes().to_vec();
//                 let data = deserialize_data(&data_data);
//                 let filter = data.get("filter").expect("`filter` is not present");
//                 let filter: DbFilter<deploy_log::DeployLogInstance> = serde_json::from_str(filter.as_str())
//                     .expect("cannot deserialize `filter`");

//                 let result = ctx.database_connection.filter_deploy_log(
//                     filter,
//                     data.get("limit").expect("no data found").trim().parse::<u32>().expect("`limit` must be of type `i32`")
//                 ).await;
                
//                 ctx.sender.send((orig_sender, notifier::craft_type_notify_message(&response, &[serialize_data(&result)]))).expect("database cannot send");
//             },
//             "save_log_deploy" => {
//                 let challenge_id = data.get("challenge_id").expect("`challenge_id` is not present");
//                 let state = data.get("state").expect("`state` is not present");
//                 let start_time = data.get("start_time").expect("`start_time` is not present");
//                 let end_time = data.get("end_time").expect("`end_time` is not present");

//                 ctx.database_connection.save_log_deploy(
//                     challenge_id.trim().parse::<i32>().expect("`challenge_id` must be of type `i32`"),
//                     state.trim().parse::<i32>().expect("`state` must be of type `i32`"),
//                     start_time.trim().parse::<i64>().expect("`start_time` must be of type `i32`"),
//                     end_time.trim().parse::<i64>().expect("`end_time` must be of type `i32`")
//                 ).await;
                
//                 // ctx.sender.send((orig_sender, notifier::craft_type_notify_message(&response, &[result]))).expect("database cannot send");
//             },
//             "delete_deploy_log" => {
//                 let data_data = data.get("data").expect("no `data`.`data` found").as_bytes().to_vec();
//                 let data = deserialize_data(&data_data);
//                 let id = data.get("id").expect("`id` is not present");

//                 let result = ctx.database_connection.delete_deploy_log(
//                     id.trim().parse::<i32>().expect("`id` must be of type `i32`"),
//                 ).await;
                
//                 ctx.sender.send((orig_sender, notifier::craft_type_notify_message(&response, &[result]))).expect("database cannot send");
//             },
//             "get_user" => {
//                 let data_data = data.get("data").expect("no data found").as_bytes().to_vec();
//                 let data = deserialize_data(&data_data);
//                 let filter = data.get("filter").expect("`filter` is not present");
//                 let filter: DbFilter<user::UserInstance> = serde_json::from_str(filter.as_str())
//                     .expect("cannot deserialize `filter`");

//                 let user = ctx.database_connection.get_user(
//                     filter,
//                     data.get("should_censor_password").unwrap_or(&"true".to_string()).parse::<bool>()
//                         .expect("`should_censor_password` must be of type `bool`")
//                 ).await;
                
//                 if user.id() == -1 {
//                     let exception = DbException {
//                         error: DbError::FetchFailed
//                     };

//                     ctx.sender.send((orig_sender, notifier::craft_type_notify_message(
//                         &error, &[serialize_data(&exception)]
//                     ))).expect("database cannot send");
//                 } else {
//                     ctx.sender.send((orig_sender, notifier::craft_type_notify_message(
//                         &response, &[serialize_data(&user)]
//                     ))).expect("database cannot send");
//                 }
//             },
//             "user_login" => {
//                 let data_data = data.get("data").expect("no data found").as_bytes().to_vec();
//                 let data = deserialize_data(&data_data);
//                 let username = data.get("username").expect("`username` is not present");
//                 let password = data.get("password").expect("`password` is not present");

//                 let mut user = ctx.database_connection.user_login(
//                     username,
//                     password
//                 ).await;
                
//                 if user.id() == -1 {
//                     let exception = DbException {
//                         error: DbError::AuthenticationFailed
//                     };
                    
//                     ctx.sender.send((orig_sender, notifier::craft_type_notify_message(
//                         &error, &[serialize_data(&exception)]
//                     ))).expect("database cannot send");
//                 } else {
//                     if user.is_locked() {
//                         let now = chrono::offset::Utc::now().timestamp();
//                         if user.raw_lock_due_at() <= now {
//                             user.unlock();
//                         }

//                         else {
//                             let exception = DbException {
//                                 error: DbError::AuthenticationFailed
//                             };

//                             ctx.sender.send((orig_sender, notifier::craft_type_notify_message(
//                                 &error, &[serialize_data(&exception)]
//                             ))).expect("database cannot send");
//                             return ();
//                         }
//                     }

//                     ctx.sender.send((orig_sender, notifier::craft_type_notify_message(&response, &[
//                         serialize_data(&user)]
//                     ))).expect("database cannot send");
//                 }
//             },
//             "user_register" => {
//                 let data_data = data.get("data").expect("no `data`.`data` found").as_bytes().to_vec();
//                 let data = deserialize_data(&data_data);
//                 let user: user::UserInstance = serde_json::from_str(
//                     data.get("user").expect("`id` is not present")
//                 ).expect("cannot deserialize `user`");

//                 let result = ctx.database_connection.user_register(
//                     user,
//                 ).await;
                
//                 ctx.sender.send((orig_sender, notifier::craft_type_notify_message(&response, &[result]))).expect("database cannot send");
//             },
//             "user_edit" => {
//                 let data_data = data.get("data").expect("no `data`.`data` found").as_bytes().to_vec();
//                 let data = deserialize_data(&data_data);
//                 let user: user::UserInstance = serde_json::from_str(
//                     data.get("user").expect("`user` is not present")
//                 ).expect("cannot deserialize `user`");

//                 let result = ctx.database_connection.edit_user(
//                     user
//                 ).await;
                
//                 ctx.sender.send((orig_sender, notifier::craft_type_notify_message(&response, &[result]))).expect("database cannot send");
//             },
//             "user_create" => {
//                 let data_data = data.get("data").expect("no `data`.`data` found").as_bytes().to_vec();
//                 let data = deserialize_data(&data_data);
//                 let user: user::UserInstance = serde_json::from_str(
//                     data.get("user").expect("`user` is not present")
//                 ).expect("cannot deserialize `user`");

//                 let result = ctx.database_connection.create_user(
//                     user
//                 ).await;
                
//                 ctx.sender.send((orig_sender, notifier::craft_type_notify_message(&response, &[result]))).expect("database cannot send");
//             },
//             "user_delete" => {

//                 let result = ctx.database_connection.delete_user(
//                     data.get("data").expect("`id` not present").trim().parse::<i32>().expect("`id` must be of type `i32`")
//                 ).await;
                
//                 ctx.sender.send((orig_sender, notifier::craft_type_notify_message(&response, &[result]))).expect("database cannot send");
//             },
//             "fetch_recent_solve_log" => {
//                 let result = ctx.database_connection.fetch_recent_solve_log(
//                     data.get("data").expect("`id` not present").trim().parse::<u32>().expect("`id` must be of type `i32`")
//                 ).await;
                
//                 ctx.sender.send((orig_sender, notifier::craft_type_notify_message(&response, &[serialize_data(&result)]))).expect("database cannot send");
//             },
//             "log_solve_result" => {
//                 let data_data = data.get("data").expect("no `data`.`data` found").as_bytes().to_vec();
//                 let history_entry: solve_history::SolveHistoryEntry = deserialize_solve_history(&data_data);

//                 ctx.database_connection.log_solve_result(
//                     history_entry
//                 ).await;
                
//                 // ctx.sender.send((orig_sender, notifier::craft_type_notify_message(&response, &[serialize_data(&result)]))).expect("database cannot send");
//             },
//             "filter_solve_log" => {
//                 let data_data = data.get("data").expect("no data found").as_bytes().to_vec();
//                 let data = deserialize_data(&data_data);
//                 let filter = data.get("filter").expect("`filter` is not present");
//                 let filter: DbFilter<solve_history::SolveHistoryEntry> = serde_json::from_str(filter.as_str())
//                     .expect("cannot deserialize `filter`");

//                 let result = ctx.database_connection.filter_solve_log(
//                     filter,
//                     data.get("limit").expect("no data found").trim().parse::<u32>().expect("`limit` must be of type `i32`")
//                 ).await;
                
//                 ctx.sender.send((orig_sender, notifier::craft_type_notify_message(&response, &[serialize_data(&result)]))).expect("database cannot send");
//             },
//             "delete_solve_log" => {
//                 let data_data = data.get("data").expect("no `data`.`data` found").as_bytes().to_vec();
//                 let data = deserialize_data(&data_data);
//                 let id = data.get("id").expect("`id` is not present");

//                 let result = ctx.database_connection.delete_solve_log(
//                     id.trim().parse::<i32>().expect("`id` must be of type `i32`"),
//                 ).await;
                
//                 ctx.sender.send((orig_sender, notifier::craft_type_notify_message(&response, &[result]))).expect("database cannot send");
//             },
//             _ => panic!("this shit is unknow xd")
//         }
//     }
// }

// fn deserialize_data(serialized_data: &Vec<u8>) -> HashMap<&str, String> {
//     let data: HashMap<&str, String> = serde_json::from_slice(serialized_data.as_slice()).expect("deserialize failed!");
//     return data;
// }

// fn deserialize_solve_history(serialized_data: &Vec<u8>) -> solve_history::SolveHistoryEntry {
//     let data: solve_history::SolveHistoryEntry = serde_json::from_slice(serialized_data.as_slice()).expect("deserialize failed!");
//     return data;
// }

// fn deserialize_user(serialized_data: &Vec<u8>) -> user::UserInstance {
//     let data: user::UserInstance = serde_json::from_slice(serialized_data.as_slice()).expect("deserialize failed!");
//     return data;
// }

// fn deserialize_deploy_log(serialized_data: &Vec<u8>) -> deploy_log::DeployLogInstance {
//     let data: deploy_log::DeployLogInstance = serde_json::from_slice(serialized_data.as_slice()).expect("deserialize failed!");
//     return data;
// }

// fn serialize_data<T: serde::Serialize>(data: &T) -> String {
//     serde_json::to_string(data).expect("cannot serialize data")
// }

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub enum DbError {
    ConnectionAlreadyClosed,
    FetchFailed,
    AuthenticationFailed
}

#[derive(Clone)]
pub struct DbConnection {
    pool: Pool<Postgres>
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct DbException {
    error: DbError
}

#[derive(serde::Deserialize, Debug)]
pub struct DbFilter<T> {
    filter_instance: T,
    pub filter_by: Vec<(String, String)>
}

impl<T> DbFilter<T> {
    pub fn filter_with(instance: T, filter: Vec<(String, String)>) -> Self {
        DbFilter::<T> {
            filter_instance: instance,
            filter_by: filter
        }
    }
    pub fn filter_by(&self) -> &Vec<(String, String)> {
        &self.filter_by
    }

    pub fn filter_instance(&self) -> &T {
        &self.filter_instance
    }
}
impl DbConnection {
    pub fn do_clone(&self) -> Self {
        DbConnection {
            pool: self.pool.clone()
        }
    }

    #[allow(dead_code)]
    async fn close(&self) -> bool {
        self.pool.close().await;
        if self.pool.is_closed() {
            return true;
        }

        return false;
    }

    pub fn is_closed(&self) -> bool {
        self.pool.is_closed()
    }

    pub async fn fetch_recent_deploy_log(&self, limit: u32) -> Vec<deploy_log::DeployLogInstance>  {
        let filter_none: DbFilter<deploy_log::DeployLogInstance> = DbFilter::<deploy_log::DeployLogInstance> {
            filter_instance: deploy_log::DeployLogInstance {
                id: -1,
                challenge_id: -1,
                state: -1,
                start_time: -1,
                end_time: -1
            },
            filter_by: Vec::new()
        };

        deploy_log::db_filter_for_deploy_log(&self, filter_none, limit as i32).await.expect(
            "Attemp to query on a closed DB connection"
        )
    }

    pub async fn filter_deploy_log(&self, filter: DbFilter<deploy_log::DeployLogInstance>, limit: u32) -> Vec<deploy_log::DeployLogInstance> {
        deploy_log::db_filter_for_deploy_log(&self, filter, limit as i32).await.expect("Attemp to query on a closed DB connection")
    }

    pub async fn save_log_deploy(&self, challenge_id: i32, state: i32, start_time: i64, end_time: i64) -> bool {
        let result: bool = deploy_log::db_insert_deploy_log(&self, &deploy_log::DeployLogInstance {
            id: -1, // id is auto and serial, assign to shut the rust compiler's mouth
            challenge_id,
            state,
            start_time,
            end_time
        }).await.unwrap_or(false);

        return result;
    }

    pub async fn delete_deploy_log(&self, deploy_id: i32) -> bool {
        deploy_log::db_delete_deploy_log(&self, deploy_id).await.expect("Attemp to query on a closed DB connection")
    }

    pub async fn get_user(&self, filter: DbFilter<user::UserInstance>, password_censor: bool) -> user::UserInstance {
        let users: Vec<user::UserInstance> = user::db_filter_for_user(&self, filter, 1).await.unwrap_or(
            vec![user::UserInstance::get_dead_guy_user()]
        );

        if users.len() == 0 {
            return user::UserInstance::get_dead_guy_user();
        }

        let user = users.get(0).unwrap();

        user.censor_password(password_censor)
    }

    pub async fn filter_user(&self, filter: DbFilter<user::UserInstance>) -> Vec<user::UserInstance> {
        let users: Vec<user::UserInstance> = user::db_filter_for_user(&self, filter, 1).await.unwrap_or(
            Vec::new()
        );

        users
    }

    pub async fn user_login(&self, username: &str, password: &str) -> user::UserInstance {
        let user: user::UserInstance = user::db_user_login(&self, username, password).await.unwrap_or(
            user::UserInstance::get_dead_guy_user()
        );

        user.censor_password(true)
    }

    pub async fn user_register(&self, user: user::UserInstance) -> bool {
        let result: bool = user::db_user_register(self, user).await.unwrap_or(false);

        return result;
    }

    pub async fn edit_user(&self, user: user::UserInstance) -> bool {
        let result: bool = user::db_edit_user(self, user).await.unwrap_or(false);

        return result;
    }

    pub async fn create_user(&self, user_to_create: user::UserInstance) -> bool {
        let result: bool = user::db_user_create(self, user_to_create).await.unwrap_or(false);

        return result;
    }

    pub async fn delete_user(&self, user_id: i32) -> bool {
        user::db_delete_user(&self, user_id).await.unwrap_or(false)
    }

    pub async fn fetch_recent_solve_log(&self, limit: u32) -> Vec<solve_history::SolveHistoryEntry> {
        let filter_none: DbFilter<solve_history::SolveHistoryEntry> = DbFilter::<solve_history::SolveHistoryEntry> {
            filter_instance: solve_history::SolveHistoryEntry::get_empty_solve_history_entry(),
            filter_by: Vec::new()
        };

        solve_history::db_filter_for_solve_history(&self, filter_none, limit as i32).await.expect(
            "Attemp to query on a closed DB connection"
        )
    }

    pub async fn log_solve_result(&self, solve_entry: solve_history::SolveHistoryEntry) -> bool {
        let result: bool = solve_history::db_save_solve_result(self, solve_entry).await.unwrap_or(false);

        return result;
    }

    pub async fn filter_solve_log(&self, filter: DbFilter<solve_history::SolveHistoryEntry>, limit: u32) -> Vec<solve_history::SolveHistoryEntry> {
        solve_history::db_filter_for_solve_history(&self, filter, limit as i32).await.unwrap_or(
            vec![]
        )
    }

    pub async fn delete_solve_log(&self, solve_id: i32) -> bool {
        solve_history::db_delete_solve_result(&self, solve_id).await.expect(
            "Can't delete log"
        )
    }
}

pub async fn new_db_connection() -> Result<DbConnection, sqlx::Error> {

    return match db_connect().await {
        Ok(pool) => {
            println!("Db Connected");
            Ok(DbConnection {
                pool
            })
        },
        Err(err) => Err(err)
    };
} 

#[allow(dead_code)]
pub async fn initialize_database() -> Result<bool, sqlx::Error> {
    let pool = db_connect().await.expect("Can't initialize db");

    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(true)
}

async fn db_connect() -> Result<Pool<Postgres>, sqlx::Error> {
    let connection_str = format!(
        "postgres://{username}:{password}@{host}/{db_name}", 
        username=DB_USERNAME, 
        password=DB_PASSWORD,
        host=DB_HOST,
        db_name=DB_DATABASE_NAME
    );

    println!("connecting to database");
    let pool: Pool<Postgres> = PgPoolOptions::new()
        .max_connections(DB_POOL_MAX_CONNECTION)
        .connect(&connection_str[..]).await?;
    
    Ok(pool)
}