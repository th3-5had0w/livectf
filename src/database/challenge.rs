use std::vec;

use sqlx::postgres::PgQueryResult;
use sqlx::FromRow;

use crate::database::{DbConnection, DB_CHALLENGE_TABLE};

const MINIMUM_SCORE: i32 = 50;
const INITIAL_SCORE: i32 = 500;
// lower this makes score decay faster
const DECAY_VALUE: i32 = 300;
const SOLVES_BEFORE_DECAY: i32 = 1;

#[derive(FromRow)]
pub struct ChallengeData {
    #[allow(dead_code)]
    pub id: i32,
    pub challenge_name: String,
    pub score: i32,
    pub category: String,
    pub solved_by: Vec<String>,
    pub running: bool,
    pub connection_string: String
}

#[derive(FromRow)]
struct ScoreStruct {
    score: i32
}

pub async fn db_store_challenge_metadata(db_connection: &DbConnection, challenge: ChallengeData) -> bool {
    let no_one_solved: Vec<String> = vec![];
    let query = format!("
    INSERT INTO {table_name} (
        challenge_name,
        score,
        category,
        solved_by,
        running,
        connection_string
    )
    VALUES
        (
            $1,
            $2,
            $3,
            $4,
            $5,
            $6
        );", table_name=DB_CHALLENGE_TABLE);
        let result: PgQueryResult = sqlx::query(&query[..])
        .bind(challenge.challenge_name.trim())
        .bind(challenge.score)
        .bind(challenge.category)
        .bind(no_one_solved)
        .bind(false)
        .bind(challenge.connection_string)
        .execute(&db_connection.pool).await.expect("Error storing challenge metadata");

    if result.rows_affected() > 0 {
        return true;
    }
    return false;
} 

pub async fn db_get_challenge_score(db_connection: &DbConnection, name: String) -> i32 {
    let query = format!("SELECT score FROM {table_name} WHERE challenge_name=$1;", table_name=DB_CHALLENGE_TABLE);


    let res: ScoreStruct = sqlx::query_as(&query[..])
        .bind(name.trim())
        .fetch_one(&db_connection.pool).await.unwrap_or(ScoreStruct { score: 0});
    
    return res.score;
}

pub async fn db_challenge_solve(db_connection: &DbConnection, chall_name: String, username: String) -> bool {
    let query = format!("UPDATE {table_name} SET solved_by = array_append(solved_by, $2) WHERE challenge_name=$1;", table_name=DB_CHALLENGE_TABLE);

    let res= sqlx::query(&query[..])
        .bind(chall_name.trim())
        .bind(username.trim())
        .execute(&db_connection.pool).await.expect("cannot update challenge");
    
    if res.rows_affected() == 0 {
        return false;
    }

    // (((minimum - initial)/(decay**2)) * (solve_count**2)) + initial
    let chall = db_get_challenge_by_name(db_connection, chall_name).await;
    let score = (MINIMUM_SCORE - INITIAL_SCORE)/(DECAY_VALUE);
    let score = score * (i32::try_from(chall.solved_by.len().pow(2)).unwrap() - SOLVES_BEFORE_DECAY);
    let score = score + INITIAL_SCORE;

    if db_update_challenge_score(db_connection, chall.challenge_name, score).await {
        return true;
    }

    return false;
}

pub async fn db_get_challenge_by_name(db_connection: &DbConnection, name: String) -> ChallengeData {
    let query = format!("SELECT * FROM {table_name} WHERE challenge_name=$1;", table_name=DB_CHALLENGE_TABLE);


    let chall = sqlx::query_as(&query[..])
        .bind(name)
        .fetch_one(&db_connection.pool).await.unwrap_or(ChallengeData {
            id: -1,
            challenge_name: "none".to_string(),
            score: 0,
            category: "Nope".to_string(),
            solved_by: vec![],
            running: false,
            connection_string: "".to_string()
        });
    
    return chall;
}

pub async fn db_get_all_running_challenges(db_connection: &DbConnection) -> Vec<ChallengeData> {
    let query = format!("SELECT * FROM {table_name} WHERE running=true", table_name=DB_CHALLENGE_TABLE);


    let challs = sqlx::query_as(&query[..])
        .fetch_all(&db_connection.pool).await.unwrap_or(vec![]);
    
    return challs;
}

pub async fn db_set_challenge_running(db_connection: &DbConnection, name: String, is_running: bool) -> bool {
    let query = format!("UPDATE {table_name} SET running = $2 WHERE challenge_name=$1;", table_name=DB_CHALLENGE_TABLE);
    
    let res= sqlx::query(&query[..])
        .bind(name.trim())
        .bind(is_running)
        .execute(&db_connection.pool).await.expect("cannot update challenge");
    
    if res.rows_affected() > 0 {
        return true;
    }
    return false;
}

pub async fn db_set_challenge_connection_string(db_connection: &DbConnection, name: String, connection_string: String) -> bool {
    let query = format!("UPDATE {table_name} SET connection_string = $2 WHERE challenge_name=$1;", table_name=DB_CHALLENGE_TABLE);

    let res= sqlx::query(&query[..])
        .bind(name.trim())
        .bind(connection_string)
        .execute(&db_connection.pool).await.expect("cannot update challenge");
    
    if res.rows_affected() > 0 {
        return true;
    }
    return false;
}

pub async fn db_update_challenge_score(db_connection: &DbConnection, name: String, score: i32) -> bool {
    let query = format!("UPDATE {table_name} SET score = $2 WHERE challenge_name=$1;", table_name=DB_CHALLENGE_TABLE);

    let res= sqlx::query(&query[..])
        .bind(name.trim())
        .bind(score)
        .execute(&db_connection.pool).await.expect("cannot update challenge");
    
    if res.rows_affected() > 0 {
        return true;
    }
    return false;
}