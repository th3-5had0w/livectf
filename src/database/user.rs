use sqlx::{FromRow, Decode};
use sqlx::postgres::PgQueryResult;
use chrono::{DateTime, offset::Utc};
use bcrypt::{verify as bcrypt_verify, hash as bcrypt_hash};
use serde;
    
use crate::database::{DbConnection, DbFilter, DB_USER_TABLE};

#[derive(FromRow, Decode, serde::Deserialize, serde::Serialize, Debug)]
pub struct UserInstance {
    pub id: i32,
    pub username: String,
    pub password: String,
    pub email: String,
    pub challenge_solved: Vec<String>,
    pub bio: String,
    pub is_locked: bool,
    pub lock_due_at: i64,
    pub is_admin: bool,
    pub last_submission: i64
}

impl UserInstance {
    pub fn new(
        username: &str,
        password: &str,
        email: &str,
        is_admin: bool
    ) -> Self {
        UserInstance {
            id: -1,
            username: username.to_string(),
            password: password.to_string(),
            email: email.to_string(),
            challenge_solved: vec![],
            bio: "write something...".to_string(),
            is_locked: false,
            lock_due_at: 0,
            is_admin,
            last_submission: 0
        }
    } 

    #[allow(dead_code)]
    pub fn lock_due_at(&self) -> DateTime<Utc> {
        DateTime::from_timestamp(self.lock_due_at as i64, 0).unwrap_or(DateTime::from_timestamp(0, 0).unwrap())
    }

    pub fn deep_copy(&self) -> Self {
        UserInstance {
            id: self.id,
            username: self.username.to_string(),
            password: self.password.to_string(),
            email: self.email.to_string(),
            bio: self.bio.to_string(),
            challenge_solved: self.challenge_solved.to_vec(),
            is_locked: self.is_locked,
            lock_due_at: self.lock_due_at,
            is_admin: self.is_admin,
            last_submission: 0
        }
    }

    pub fn unlock(&mut self) {
        self.is_locked = false;
    }

    #[allow(dead_code)]
    pub fn lock(&mut self) {
        self.is_locked = true;
    }

    // a dead guy cannot login, return when a authentication failure occure
    pub fn get_dead_guy_user() -> Self {
        UserInstance {
            id: -1,
            username: "dead guy".to_string(),
            password: "dead guy".to_string(),
            email: "dead guy".to_string(),
            challenge_solved: vec![],
            bio: "no account matched that username".to_string(),
            is_locked: false,
            lock_due_at: 0,
            is_admin: false,
            last_submission: 0
        }
    }

    pub fn censor_password(&self, should_censor: bool) -> Self {
        let mut new_user = self.deep_copy();
        if should_censor {
            new_user.password = String::from("<REDACTED>");
        }
        return new_user
    }

    pub fn censor_email(&self, should_censor: bool) -> Self {
        let mut new_user = self.deep_copy();
        if should_censor {
            new_user.email = String::from("<REDACTED>");
        }
        return new_user
    }
}

pub async fn db_filter_for_user(db_connection: &DbConnection, filter: DbFilter<UserInstance>, limit: i64) -> Result<Vec<UserInstance>, sqlx::Error> {
    let mut filter_statement = String::new();
    let filter_by = filter.filter_by();

    if filter_by.len() > 0 {
        filter_statement.push_str("WHERE ");
        let mut count = 1;
        for field in filter_by {
            let name = field.0.as_str();
            let op = field.1.clone();

            match name {
                "id" => {
                    filter_statement.push_str(
                        (format!("id{}", op) + format!("{}", &filter.filter_instance().id).as_str()).as_str()
                    )
                },
                "username" => { 
                    let username = &filter.filter_instance().username;

                    filter_statement.push_str(
                        format!("username LIKE '{}'", username.replace("\'", "\\'")).as_str()
                    )
                },
                "bio" => {
                    let bio = &filter.filter_instance().bio;

                    filter_statement.push_str(
                        format!("bio LIKE '{}'", bio.replace("\'", "\\'")).as_str()
                    )
                },
                "is_locked" => {
                    filter_statement.push_str(
                        format!("is_locked={}", &filter.filter_instance().is_locked).as_str()
                    )
                },
                "is_admin" => {
                    filter_statement.push_str(
                        format!("is_admin={}", &filter.filter_instance().is_admin).as_str()
                    )
                },
                _ => ()
            } 

            if count != filter_by.len() {
                filter_statement.push_str(" AND ");
            }
            count += 1;
        }
    }
    
    let mut query = format!("SELECT * FROM {table_name} {filter_statement}", table_name=DB_USER_TABLE, filter_statement=filter_statement);

    if limit != -1 {
        query.push_str("LIMIT $1");
    }
    let mut query_as = sqlx::query_as(&query[..]);

    if limit != -1 {
        query_as = query_as.bind(limit);
    }

    let records: Vec<UserInstance> = query_as.fetch_all(&db_connection.pool).await.unwrap_or(Vec::<UserInstance>::new());
    
    return Ok(records);
}

pub async fn db_edit_user(db_connection: &DbConnection, user: UserInstance) -> Result<bool, sqlx::Error> {

    let query = format!("
    UPDATE {table_name} 
    SET 
        (
            username,
            password,
            email,
            challenge_solved,
            bio,
            is_locked,
            lock_due_at,
            is_admin,
            last_submission
        )
    =
        (
            $1,
            $2,
            $3,
            $4,
            $5,
            $6,
            $7,
            $8,
            $9
        )
    WHERE id = $10
    ", table_name=DB_USER_TABLE);
    let result: PgQueryResult = sqlx::query(&query[..])
        .bind(user.username)
        .bind(bcrypt_hash(user.password, 6).unwrap())
        .bind(user.email)
        .bind(user.challenge_solved)
        .bind(user.bio)
        .bind(user.is_locked)
        .bind(user.lock_due_at)
        .bind(user.is_admin)
        .bind(user.last_submission)
        .bind(user.id)
        .execute(&db_connection.pool).await.unwrap();

    if result.rows_affected() > 0 {
        return Ok(true);
    }
    return Ok(false);
}

pub async fn db_delete_user(db_connection: &DbConnection, user_id: i32) -> Result<bool, sqlx::Error> {
    let query = format!("DELETE FROM {table_name} WHERE id = $1", table_name=DB_USER_TABLE);
    let result: PgQueryResult = sqlx::query(&query[..])
        .bind(user_id)
        .execute(&db_connection.pool).await.unwrap();

    if result.rows_affected() > 0 {
        return Ok(true);
    }
    return Ok(false);
}

pub async fn db_user_login(db_connection: &DbConnection, username: &str, password: &str) -> Result<UserInstance, sqlx::Error> {
    let query = format!("SELECT * FROM {table_name} WHERE username=$1;", table_name=DB_USER_TABLE);


    let user = sqlx::query_as(&query[..])
        .bind(username)
        .fetch_one(&db_connection.pool).await.unwrap_or_else(|_| {
            UserInstance::get_dead_guy_user()
        });
    
    let result = bcrypt_verify(password, &user.password[..]).unwrap_or(false);
    
    if result {
        return Ok(user);
    } else {
        return Ok(UserInstance::get_dead_guy_user());
    }
}

pub async fn db_user_register(db_connection: &DbConnection, user: UserInstance) -> Result<bool, sqlx::Error> {
    let query = format!("
    INSERT INTO {table_name} (
        username,
        password,
        email,
        challenge_solved,
        bio,
        is_locked,
        lock_due_at,
        is_admin,
        last_submission
    )
    VALUES
        (
            $1,
            $2,
            $3,
            $4,
            $5,
            $6,
            $7,
            $8,
            $9
        );", table_name=DB_USER_TABLE);
        let result: PgQueryResult = sqlx::query(&query[..])
        .bind(user.username.trim())
        .bind(bcrypt_hash(user.password, 6).unwrap())
        .bind(user.email.trim())
        .bind(user.challenge_solved)
        .bind(user.bio)
        .bind(user.is_locked)
        .bind(user.lock_due_at)
        .bind(user.is_admin)
        .bind(user.last_submission)
        .execute(&db_connection.pool).await?;

    if result.rows_affected() > 0 {
        return Ok(true);
    }
    return Ok(false);

}

pub async fn db_user_create(db_connection: &DbConnection, user_to_create: UserInstance) -> Result<bool, sqlx::Error> {
    let result: bool = db_user_register(db_connection, user_to_create).await?;

    return Ok(result);

}

pub async fn db_get_all_user(db_connection: &DbConnection) -> Vec<UserInstance> {
    let result: Vec<UserInstance> = sqlx::query_as("SELECT * FROM users;")  
        .fetch_all(&db_connection.pool).await
        .unwrap_or(Vec::<UserInstance>::new());

    return result;
}

pub async fn db_get_user_by_id(db_connection: &DbConnection, user_id: i32) -> Result<UserInstance, sqlx::Error> {
    let query = format!("SELECT * FROM {table_name} WHERE id=$1;", table_name=DB_USER_TABLE);

    let user = sqlx::query_as(&query[..])
        .bind(user_id)
        .fetch_one(&db_connection.pool).await.unwrap_or(UserInstance::get_dead_guy_user());
    
    return Ok(user);
}

pub async fn db_get_user_by_name(db_connection: &DbConnection, name: String) -> Result<UserInstance, sqlx::Error> {
    let query = format!("SELECT * FROM {table_name} WHERE username=$1;", table_name=DB_USER_TABLE);


    let user = sqlx::query_as(&query[..])
        .bind(name)
        .fetch_one(&db_connection.pool).await.unwrap_or(UserInstance::get_dead_guy_user());
    
    return Ok(user);
}

// pub async fn db_get_all_user_solved_challenge(db_connection: &DbConnection, challenge_name: String) -> Vec<UserInstance> {
//     let query = format!("
//     SELECT 
//         id,
//         username,
//         password,
//         email,
//         challenge_solved,
//         bio,
//         is_locked,
//         lock_due_at,
//         is_admin
//     FROM 
//         {table_name}
//     WHERE 
//         $1 = ANY (challenge_solved);",
//         table_name=DB_USER_TABLE
//     );


//     let users = sqlx::query_as(&query[..])
//         .bind(challenge_name)
//         .fetch_all(&db_connection.pool).await.unwrap_or(vec![]);
    
//     return users;
// }