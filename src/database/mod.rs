use sqlx::postgres::{PgPoolOptions, Postgres};
use sqlx::pool::Pool;
use std::clone::Clone;
use std::vec;

pub mod user;
pub mod solve_history;
pub mod challenge;

// TODO: change TEXT to VARCHAR as TEXT is slow
// TODO: remove all the getters they are so useless 
// remember to change this to a .env file, the credentials should be stored in environment variable rather than hard-coded
const DB_HOST: &str = "localhost";
const DB_USERNAME: &str = "test";
const DB_PASSWORD: &str = "WisHBrAdhOtalMaNOste";
const DB_DATABASE_NAME: &str = "livectf";
const DB_POOL_MAX_CONNECTION: u32 = 5;

const DB_CHALLENGE_TABLE: &str = "challenges";
const DB_USER_TABLE: &str = "users";
const DB_SOLVE_HISTORY_TABLE: &str = "solve_history";

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

    pub async fn get_user_by_id(&self, user_id: i32, password_censor: bool) -> user::UserInstance {
        let found_user: user::UserInstance = user::db_get_user_by_id(&self, user_id).await.unwrap();

        found_user.censor_password(password_censor)
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

    pub async fn user_add_score(&self, user_id: i32, how_many: i32) -> bool {
        let mut found_user = self.get_user_by_id(user_id, true).await;

        found_user.score += how_many;
        self.edit_user(found_user).await
    }

    pub async fn create_user(&self, user_to_create: user::UserInstance) -> bool {
        let result: bool = user::db_user_create(self, user_to_create).await.unwrap_or(false);

        return result;
    }

    pub async fn delete_user(&self, user_id: i32) -> bool {
        user::db_delete_user(&self, user_id).await.unwrap_or(false)
    }

    pub async fn get_all_user(&self) -> Vec<user::UserInstance> {
        user::db_get_all_user(&self).await
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

    pub async fn filter_solve_log(&self, filter: DbFilter<solve_history::SolveHistoryEntry>, limit: i32) -> Vec<solve_history::SolveHistoryEntry> {
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