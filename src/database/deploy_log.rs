use sqlx::postgres::PgQueryResult;
use sqlx::{FromRow, Decode};
use chrono::DateTime;
use chrono::offset::Utc;
use serde;

use crate::database::{DbConnection, DbError, DbFilter, DB_DEPLOY_LOG_TABLE, DB_DATABASE_NAME};

#[allow(dead_code)]
pub enum DeployState {
    Success,
    Failed,
    Pending
}

#[derive(FromRow, Decode, serde::Serialize, serde::Deserialize)]
pub struct DeployLogInstance {
    pub id: i32,
    pub challenge_id: i32,
    pub state: i32,
    pub start_time: i64,
    pub end_time: i64
}

impl DeployLogInstance {
    pub fn id(&self) -> i32 {
        self.id
    }

    pub fn challenge_id(&self) -> i32 {
        self.challenge_id
    }

    #[allow(dead_code)]
    pub fn state(&self) -> DeployState {
        match self.state {
            0 => DeployState::Success,
            1 => DeployState::Pending,
            _ => DeployState::Failed
        }
    }

    pub fn raw_state(&self) -> i32 {
        self.state
    }

    #[allow(dead_code)]
    pub fn start_time(&self) -> DateTime<Utc> {
        DateTime::from_timestamp(self.start_time as i64, 0).unwrap_or(DateTime::from_timestamp(0, 0).unwrap())
    }

    pub fn raw_start_time(&self) -> i64 {
        self.start_time
    }

    #[allow(dead_code)]
    pub fn end_time(&self) -> DateTime<Utc> {
        DateTime::from_timestamp(self.end_time as i64, 0).unwrap_or(DateTime::from_timestamp(0, 0).unwrap())
    }

    pub fn raw_end_time(&self) -> i64 {
        self.end_time
    }
}

pub async fn db_filter_for_deploy_log(
    db_connection: &DbConnection, 
    filter: DbFilter<DeployLogInstance>,
    limit: i32
) -> Result<Vec<DeployLogInstance>, DbError> {
    if db_connection.is_closed() {
        return Err(DbError::ConnectionAlreadyClosed);
    } else {
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
                    "challenge_id" => { 
                        filter_statement.push_str(
                            (format!("challenge_id{}", op) + format!("{}", &filter.filter_instance().challenge_id()).as_str()).as_str()
                        )
                    },
                    "state" => {
                        filter_statement.push_str(
                            (format!("state{}", op) + format!("{}", &filter.filter_instance().raw_state()).as_str()).as_str()
                        )
                    },
                    "start_time" => {
                        filter_statement.push_str(
                            (format!("start_time{}", op) + format!("{}", &filter.filter_instance().raw_state()).as_str()).as_str()
                        )
                    },
                    "end_time" => {
                        filter_statement.push_str(
                            (format!("end_time{}", op) + format!("{}", &filter.filter_instance().raw_state()).as_str()).as_str()
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
            challenge_id,
            state,
            start_time,
            end_time
        FROM 
            {db}.{table_name}
        WHERE 
            {filter_statement}
        ", db=DB_DATABASE_NAME, table_name=DB_DEPLOY_LOG_TABLE, filter_statement=filter_statement);

        if limit != -1 {
            query.push_str("LIMIT $1")
        }
        
        let mut query_as = sqlx::query_as(&query[..]);
        
        if limit != -1 {
            query_as = query_as.bind(limit); 
        }

        let records: Vec<DeployLogInstance> = query_as.fetch_all(&db_connection.pool).await.unwrap_or(Vec::<DeployLogInstance>::new());
        
        return Ok(records);
    }
}

pub async fn db_insert_deploy_log(db_connection: &DbConnection, deploy_log: &DeployLogInstance) -> Result<bool, DbError> {
    if db_connection.is_closed() {
        return Err(DbError::ConnectionAlreadyClosed);
    } else {
        let query = format!("
        INSERT INTO {db}.{table_name} (
            challenge_id,
            state,
            start_time,
            end_time
        )
        VALUES
            (
                $1,
                $2,
                $3,
                $4,
            );", db=DB_DATABASE_NAME, table_name=DB_DEPLOY_LOG_TABLE);
            let result: PgQueryResult = sqlx::query(&query[..])
            .bind(deploy_log.challenge_id())
            .bind(deploy_log.raw_state())
            .bind(deploy_log.raw_start_time())
            .bind(deploy_log.raw_end_time())
            .execute(&db_connection.pool).await.unwrap();

        if result.rows_affected() > 0 {
            return Ok(true);
        }
        return Ok(false);
    }
}

pub async fn db_delete_deploy_log(db_connection: &DbConnection, deploy_log_id: i32) -> Result<bool, DbError> {
    if db_connection.is_closed() {
        return Err(DbError::ConnectionAlreadyClosed);
    } else {
        let query = format!("DELETE FROM {db}.{table_name} WHERE id = $1", db=DB_DATABASE_NAME, table_name=DB_DEPLOY_LOG_TABLE);
            let result: PgQueryResult = sqlx::query(&query[..])
            .bind(deploy_log_id)
            .execute(&db_connection.pool).await.unwrap();

        if result.rows_affected() > 0 {
            return Ok(true);
        }
        return Ok(false);
    }
}