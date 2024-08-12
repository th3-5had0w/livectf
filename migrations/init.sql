CREATE DATABASE livectf;

\c livectf;

CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    username TEXT UNIQUE,
    password TEXT,
    email TEXT UNIQUE,
    challenge_solved TEXT[],
    bio TEXT,
    is_locked BOOLEAN,
    lock_due_at BIGINT,
    is_admin BOOLEAN,
    last_submission BIGINT
);

CREATE TABLE solve_history (
    id SERIAL PRIMARY KEY,
    challenge_name TEXT,
    username TEXT,
    is_success BOOLEAN,
    time BIGINT,
    submit_content TEXT
);

CREATE TABLE challenges (
    id SERIAL PRIMARY KEY,
    challenge_name TEXT UNIQUE,
    score INTEGER,
    category TEXT,
    solved_by TEXT[],
    running BOOLEAN,
    connection_string TEXT
);
