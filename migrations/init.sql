CREATE DATABASE livectf;

\c livectf;

CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    username TEXT UNIQUE,
    password TEXT,
    email TEXT UNIQUE,
    challenge_solved INTEGER,
    bio TEXT,
    is_locked BOOLEAN,
    lock_due_at BIGINT,
    is_admin BOOLEAN,
    score INTEGER
);

CREATE TABLE solve_history (
    id SERIAL PRIMARY KEY,
    challenge_name TEXT,
    username TEXT,
    is_success BOOLEAN,
    time BIGINT,
    submit_content TEXT
);

CREATE TABLE challenge_metadata {
    id SERIAL PRIMARY KEY,
    challenge_name TEXT,
    score: INTEGER,
    category: TEXT,
    solved_by: TEXT[]
}

-- CREATE DUMMY DATA FOR TESTING
INSERT INTO solve_history (username, challenge_name, is_success, time, submit_content) VALUES ('shin24', '051130ad-acc4-404b-8331-ff1e9714a10c', true, 1723193798, 'cosgang{so_cool_for_first_challenge}');

INSERT INTO solve_history (username, challenge_name, is_success, time, submit_content) VALUES ('shin24', '051130ad-acc4-404b-8331-ff1e9714a10c', true, 1723193798, 'cosgang{so_cool_for_first_challenge}');
INSERT INTO solve_history (username, challenge_name, is_success, time, submit_content) VALUES ('shin24', '051130ad-acc4-404b-8331-ff1e9714a10c', true, 1723193798, 'cosgang{so_cool_for_first_challenge}');
INSERT INTO solve_history (username, challenge_name, is_success, time, submit_content) VALUES ('shin24', '051130ad-acc4-404b-8331-ff1e9714a10c', true, 1723193798, 'cosgang{so_cool_for_first_challenge}');
INSERT INTO solve_history (username, challenge_name, is_success, time, submit_content) VALUES ('shin24', '051130ad-acc4-404b-8331-ff1e9714a10c', true, 1723193798, 'cosgang{so_cool_for_first_challenge}');
INSERT INTO solve_history (username, challenge_name, is_success, time, submit_content) VALUES ('shin24', '051130ad-acc4-404b-8331-ff1e9714a10c', true, 1723193798, 'cosgang{so_cool_for_first_challenge}');
INSERT INTO solve_history (username, challenge_name, is_success, time, submit_content) VALUES ('shin24', '051130ad-acc4-404b-8331-ff1e9714a10c', false, 1723193798, 'cosgang{so_cool_for_first_challengaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaae}');