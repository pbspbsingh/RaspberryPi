create table if not exists filters (
    f_id INTEGER PRIMARY KEY NOT NULL,
    ct DATETIME DEFAULT (datetime('now','localtime')) NOT NULL,
    expr TEXT NOT NULL,
    is_regex BOOLEAN NOT NULL,
    enabled BOOLEAN NOT NULL,
    is_allow BOOLEAN NOT NULL
);
insert into filters(expr, is_regex, enabled, is_allow) values('facebook.com', false, true, true);
insert into filters(expr, is_regex, enabled, is_allow) values('instagram.com', false, true, true);
insert into filters(expr, is_regex, enabled, is_allow) values('workplace.com', false, true, true);
insert into filters(expr, is_regex, enabled, is_allow) values('hn.algolia.com', false, true, true);

create table if not exists dns_requests (
    req_id INTEGER PRIMARY KEY NOT NULL,
    req_time DATETIME DEFAULT (datetime('now','localtime')) NOT NULL,
    req_type TEXT,
    request TEXT,
    response TEXT,
    filtered BOOLEAN,
    reason TEXT,
    responded BOOLEAN NOT NULL,
    resp_ms INTEGER NOT NULL
);

create table if not exists sys_info (
    s_id INTEGER PRIMARY KEY NOT NULL,
    s_time DATETIME DEFAULT (datetime('now','localtime')) NOT NULL,
    cpu_avg REAL,
    cpu_temp REAL,
    memory REAL,
    extras JSON NOT NULL
);

