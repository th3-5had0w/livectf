CREATE DATABASE livectf;

\c livectf;

CREATE TABLE deploy_log (
    id SERIAL PRIMARY KEY,
    challenge_id INTEGER,
    state INTEGER,
    start_time BIGINT,
    end_time BIGINT
);

CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    username TEXT UNIQUE,
    password TEXT,
    email TEXT UNIQUE,
    challenge_solved INTEGER,
    bio TEXT,
    is_locked BOOLEAN,
    lock_due_at BIGINT,
    is_admin BOOLEAN
);

CREATE TABLE solve_history (
    id SERIAL PRIMARY KEY,
    user_id INTEGER,
    is_success BOOLEAN,
    time BIGINT,
    submit_content TEXT
);
