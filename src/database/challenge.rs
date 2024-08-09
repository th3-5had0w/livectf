use sqlx::postgres::PgQueryResult;
use sqlx::{FromRow, Decode};
use chrono::DateTime;
use chrono::offset::Utc;

use crate::database::{DbConnection, DbError, DbFilter, DB_CHALLENGE_TABLE};

pub struct ChallengeData {
    pub id: i32,
    pub challenge_name: String,
    pub score: i32,
    pub category: String,
    pub solved_by: Vec<String>
}

pub async fn db_store_challenge_metadata(db_connection: DbConnection, challenge: ChallengeData) -> bool {
    let no_one_solved: Vec<String> = vec![];
    let query = format!("
    INSERT INTO {table_name} (
        challenge_name,
        score,
        category,
        solved_by
    )
    VALUES
        (
            $1,
            $2,
            $3,
            $4,
        );", table_name=DB_CHALLENGE_TABLE);
        let result: PgQueryResult = sqlx::query(&query[..])
        .bind(challenge.challenge_name.trim())
        .bind(challenge.score)
        .bind(challenge.category)
        .bind(no_one_solved)
        .execute(&db_connection.pool).await.expect("Error storing challenge metadata");

    if result.rows_affected() > 0 {
        return true;
    }
    return false;
} 

pub async fn db_decay_challenge(db_connection: DbConnection, challenge: ChallengeData) -> bool {
    let no_one_solved: Vec<String> = vec![];
    let query = format!("
    INSERT INTO {table_name} (
        challenge_name,
        score,
        category,
        solved_by
    )
    VALUES
        (
            $1,
            $2,
            $3,
            $4,
        );", table_name=DB_CHALLENGE_TABLE);
        let result: PgQueryResult = sqlx::query(&query[..])
        .bind(challenge.challenge_name.trim())
        .bind(challenge.score)
        .bind(challenge.category)
        .bind(no_one_solved)
        .execute(&db_connection.pool).await.expect("Error storing challenge metadata");

    if result.rows_affected() > 0 {
        return true;
    }
    return false;
} 