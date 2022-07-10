-- Script to initialize database domain.db

create table filters (
    f_id INTEGER PRIMARY KEY NOT NULL,
    create_time DATETIME DEFAULT (datetime('now','localtime')) NOT NULL,
    expr TEXT NOT NULL,
    is_regex BOOLEAN NOT NULL,
    enabled BOOLEAN NOT NULL,
    is_allow BOOLEAN NOT NULL
);
create unique index unique_filter_expr on filters(expr, is_regex);
insert into filters(expr, is_regex, enabled, is_allow) values('hn.algolia.com', false, true, true);

create table block_list (
    bl_id INTEGER PRIMARY KEY NOT NULL,
    src TEXT NOT NULL,
    retry_count INTEGER DEFAULT 0 NOT NULL,
    domain_count INTEGER DEFAULT -1 NOT NULL,
    last_updated DATETIME DEFAULT (datetime('now','localtime')) NOT NULL
);
insert into block_list(src) values('https://v.firebog.net/hosts/Prigent-Malware.txt');

create table blocked_domains(
    bd_id INTEGER PRIMARY KEY NOT NULL,
    domain_name TEXT NOT NULL,
    source TEXT,
    updated DATETIME DEFAULT (datetime('now', 'localtime')) NOT NULL
);
create unique index unique_domain_name on blocked_domains(domain_name);