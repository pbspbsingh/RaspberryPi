create table if not exists filters (
    f_id INTEGER PRIMARY KEY NOT NULL,
    ct DATETIME DEFAULT (datetime('now','localtime')) NOT NULL,
    expr TEXT NOT NULL,
    is_regex BOOLEAN NOT NULL,
    enabled BOOLEAN NOT NULL,
    is_allow BOOLEAN NOT NULL
);
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
    temperature REAL,
    humidity REAL
);

create table if not exists block_list (
    b_id INTEGER PRIMARY KEY NOT NULL,
    b_src TEXT NOT NULL,
    b_count INTEGER,
    b_last_updated DATETIME NOT NULL
);
insert into block_list(b_src, b_count, b_last_updated) values('https://raw.githubusercontent.com/PolishFiltersTeam/KADhosts/master/KADhosts_without_controversies.txt', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://raw.githubusercontent.com/FadeMind/hosts.extras/master/add.Spam/hosts', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://v.firebog.net/hosts/static/w3kbl.txt', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://www.dshield.org/feeds/suspiciousdomains_Low.txt', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://www.dshield.org/feeds/suspiciousdomains_Medium.txt', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://www.dshield.org/feeds/suspiciousdomains_High.txt', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://raw.githubusercontent.com/matomo-org/referrer-spam-blacklist/master/spammers.txt', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://someonewhocares.org/hosts/zero/hosts', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://raw.githubusercontent.com/vokins/yhosts/master/hosts', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://winhelp2002.mvps.org/hosts.txt', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://hosts.nfz.moe/basic/hosts', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://raw.githubusercontent.com/RooneyMcNibNug/pihole-stuff/master/SNAFU.txt', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://ssl.bblck.me/blacklists/hosts-file.txt', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://adaway.org/hosts.txt', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://v.firebog.net/hosts/AdguardDNS.txt', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://v.firebog.net/hosts/Admiral.txt', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://raw.githubusercontent.com/anudeepND/blacklist/master/adservers.txt', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://s3.amazonaws.com/lists.disconnect.me/simple_ad.txt', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://v.firebog.net/hosts/Easylist.txt', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://pgl.yoyo.org/adservers/serverlist.php?hostformat=hosts&showintro=0&mimetype=plaintext', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://raw.githubusercontent.com/FadeMind/hosts.extras/master/UncheckyAds/hosts', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://raw.githubusercontent.com/bigdargon/hostsVN/master/hosts', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://raw.githubusercontent.com/jdlingyu/ad-wars/master/hosts', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://v.firebog.net/hosts/Easyprivacy.txt', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://v.firebog.net/hosts/Prigent-Ads.txt', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://gitlab.com/quidsup/notrack-blocklists/raw/master/notrack-blocklist.txt', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://raw.githubusercontent.com/FadeMind/hosts.extras/master/add.2o7Net/hosts', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://raw.githubusercontent.com/crazy-max/WindowsSpyBlocker/master/data/hosts/spy.txt', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://hostfiles.frogeye.fr/firstparty-trackers-hosts.txt', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://hostfiles.frogeye.fr/multiparty-trackers-hosts.txt', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://www.github.developerdan.com/hosts/lists/ads-and-tracking-extended.txt', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://raw.githubusercontent.com/Perflyst/PiHoleBlocklist/master/android-tracking.txt', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://raw.githubusercontent.com/Perflyst/PiHoleBlocklist/master/SmartTV.txt', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://raw.githubusercontent.com/Perflyst/PiHoleBlocklist/master/AmazonFireTV.txt', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://raw.githubusercontent.com/DandelionSprout/adfilt/master/Alternate%20versions%20Anti-Malware%20List/AntiMalwareHosts.txt', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://osint.digitalside.it/Threat-Intel/lists/latestdomains.txt', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://s3.amazonaws.com/lists.disconnect.me/simple_malvertising.txt', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://mirror1.malwaredomains.com/files/justdomains', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://v.firebog.net/hosts/Prigent-Crypto.txt', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://v.firebog.net/hosts/Prigent-Malware.txt', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://mirror.cedia.org.ec/malwaredomains/immortal_domains.txt', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://www.malwaredomainlist.com/hostslist/hosts.txt', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://bitbucket.org/ethanr/dns-blacklists/raw/8575c9f96e5b4a1308f2f12394abd86d0927a4a0/bad_lists/Mandiant_APT1_Report_Appendix_D.txt', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://phishing.army/download/phishing_army_blocklist_extended.txt', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://gitlab.com/quidsup/notrack-blocklists/raw/master/notrack-malware.txt', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://v.firebog.net/hosts/Shalla-mal.txt', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://raw.githubusercontent.com/Spam404/lists/master/main-blacklist.txt', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://raw.githubusercontent.com/FadeMind/hosts.extras/master/add.Risk/hosts', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://urlhaus.abuse.ch/downloads/hostfile/', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://raw.githubusercontent.com/HorusTeknoloji/TR-PhishingList/master/url-lists.txt', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://zerodot1.gitlab.io/CoinBlockerLists/hosts_browser', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://raw.githubusercontent.com/chadmayfield/my-pihole-blocklists/master/lists/pi_blocklist_porn_all.list', -1, datetime('now','localtime'));
insert into block_list(b_src, b_count, b_last_updated) values('https://raw.githubusercontent.com/chadmayfield/my-pihole-blocklists/master/lists/pi_blocklist_porn_top1m.list', -1, datetime('now','localtime'));
