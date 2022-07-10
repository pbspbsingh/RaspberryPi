create table dns_requests (
    req_id INTEGER PRIMARY KEY NOT NULL,
    req_time DATETIME DEFAULT (datetime('now','localtime')) NOT NULL,
    requester TEXT NOT NULL,
    req_type TEXT,
    request TEXT,
    response TEXT,
    filtered BOOLEAN,
    reason TEXT,
    responded BOOLEAN NOT NULL,
    resp_ms INTEGER NOT NULL
);
create INDEX dns_req_time_idx on dns_requests(req_time);

create table sys_info (
    s_id INTEGER PRIMARY KEY NOT NULL,
    s_time DATETIME DEFAULT (datetime('now','localtime')) NOT NULL,
    cpu_avg REAL,
    cpu_temp REAL,
    memory REAL,
    temperature REAL,
    humidity REAL
);