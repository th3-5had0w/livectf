use sqlx::postgres::PgQueryResult;
use sqlx::{FromRow, Decode};
use chrono::DateTime;
use chrono::offset::Utc;

use crate::database::{DbConnection, DbError, DbFilter, DB_SOLVE_HISTORY_TABLE};

#[derive(FromRow, Decode, serde::Deserialize, serde::Serialize)]
pub struct SolveHistoryEntry {
    id: i32,
    challenge_name: String,
    username: String,
    is_success: bool,
    time: i64,
    submit_content: String
}

impl SolveHistoryEntry {
    pub fn new(username: String, challenge_name: String, is_success: bool, submit_content: String) -> Self {
        SolveHistoryEntry {
            id: -1,
            challenge_name,
            username,
            is_success,
            submit_content,
            time: chrono::offset::Utc::now().timestamp()
        }
    }
    pub fn id(&self) -> i32 {
        self.id
    }

    pub fn username(&self) -> &str {
        self.username.as_str()
    }

    pub fn is_success(&self) -> bool {
        self.is_success
    }

    #[allow(dead_code)]
    pub fn time(&self) -> DateTime<Utc> {
        DateTime::from_timestamp(self.time as i64, 0).unwrap_or(DateTime::from_timestamp(0, 0).unwrap())
    }

    pub fn raw_time(&self) -> i64 {
        self.time
    }

    pub fn submit_content(&self) -> &str {
        self.submit_content.as_str()
    }

    pub fn challenge_name(&self) -> &str {
        &self.challenge_name
    }

    pub fn get_empty_solve_history_entry() -> Self {
        SolveHistoryEntry {
            id: -1,
            username: String::from("nothing"),
            challenge_name: String::from("nothing"),
            is_success: false,
            time: -1,
            submit_content: "".to_string()
        }
    }
}

pub async fn db_save_solve_result(db_connection: &DbConnection, solve_entry: SolveHistoryEntry) -> Result<bool, DbError> {
    if db_connection.is_closed() {
        return Err(DbError::ConnectionAlreadyClosed);
    }

    let query = format!("
    INSERT INTO {table_name} (
        username,
        challenge_name,
        is_success,
        time,
        submit_content
    )
    VALUES
        (
            $1,
            $2,
            $3,
            $4,
            $5
        );", table_name=DB_SOLVE_HISTORY_TABLE);

        let result: PgQueryResult = sqlx::query(&query[..])
        .bind(solve_entry.username())
        .bind(solve_entry.challenge_name())
        .bind(solve_entry.is_success())
        .bind(solve_entry.raw_time())
        .bind(solve_entry.submit_content())
        .execute(&db_connection.pool).await.unwrap();

    if result.rows_affected() > 0 {
        return Ok(true);
    }
    return Ok(false);
}

pub async fn db_delete_solve_result(db_connection: &DbConnection, solve_id: i32) -> Result<bool, DbError> {
    if db_connection.is_closed() {
        return Err(DbError::ConnectionAlreadyClosed);
    } else {
        let query = format!("DELETE FROM {table_name} WHERE id = $1", table_name=DB_SOLVE_HISTORY_TABLE);
            let result: PgQueryResult = sqlx::query(&query[..])
            .bind(solve_id)
            .execute(&db_connection.pool).await.unwrap();

        if result.rows_affected() > 0 {
            return Ok(true);
        }
        return Ok(false);
    }
}

pub async fn db_filter_for_solve_history(
    db_connection: &DbConnection, 
    filter: DbFilter<SolveHistoryEntry>,
    limit: i32
) -> Result<Vec<SolveHistoryEntry>, DbError> {
    if db_connection.is_closed() {
        return Err(DbError::ConnectionAlreadyClosed);
    }

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
                        (format!("id{}", op) + format!("{}", &filter.filter_instance().id()).as_str()).as_str()
                    )
                },
                "challenge_name" => {
                    let challenge_name = &filter.filter_instance().challenge_name();

                    filter_statement.push_str(
                        format!("challenge_name LIKE '{}'", challenge_name.replace("\'", "\\'")).as_str()
                    )
                },
                "username" => { 
                    let username = &filter.filter_instance().username();

                    filter_statement.push_str(
                        format!("username LIKE '{}'", username.replace("\'", "\\'")).as_str()
                    )
                },
                "is_success" => {
                    filter_statement.push_str(
                        format!("is_success={}", &filter.filter_instance().is_success()).as_str()
                    )
                },
                "time" => {
                    filter_statement.push_str(
                        (format!("time{}", op) + &format!("{}", &filter.filter_instance().raw_time()).as_str()).as_str()
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
    
    let mut query = format!("
    SELECT 
        id,
        challenge_name,
        username,
        is_success,
        time,
        submit_content
    FROM 
        {table_name}
    {filter_statement}
    ", table_name=DB_SOLVE_HISTORY_TABLE, filter_statement=filter_statement);

    if limit != -1 {
        query.push_str("LIMIT $1");
    }
    let mut query_as = sqlx::query_as(&query[..]);

    if limit != -1 {
        query_as = query_as.bind(limit);
    }

    let records: Vec<SolveHistoryEntry> = query_as.fetch_all(&db_connection.pool).await.unwrap_or(Vec::<SolveHistoryEntry>::new());
    
    return Ok(records);
}