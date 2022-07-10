use itertools::{Either, Itertools};
use log::info;
use once_cell::sync::Lazy;
use regex::RegexSet;
use tokio::sync::RwLock;

use crate::db::filters::load_filters;
use crate::filters::trie::NameTrie;

static DOMAIN_FILTER: Lazy<RwLock<NameTrie>> = Lazy::new(|| RwLock::new(NameTrie::default()));

static ALLOWED: Lazy<RwLock<Vec<String>>> = Lazy::new(|| RwLock::new(Vec::new()));
static REGEX_ALLOWED: Lazy<RwLock<RegexSet>> =
    Lazy::new(|| RwLock::new(RegexSet::new(Vec::<String>::new()).unwrap()));

static BLOCKED: Lazy<RwLock<Vec<String>>> = Lazy::new(|| RwLock::new(Vec::new()));
static REGEX_BLOCKED: Lazy<RwLock<RegexSet>> =
    Lazy::new(|| RwLock::new(RegexSet::new(Vec::<String>::new()).unwrap()));

pub async fn reload_filters() -> anyhow::Result<()> {
    let filters = load_filters().await?;
    let trie = NameTrie::create(
        filters
            .iter()
            .filter(|df| !df.is_regex)
            .map(|df| (&df.expr, df.is_allow)),
    );
    info!("Creating a name trie of size: {}", trie.count());
    *DOMAIN_FILTER.write().await = trie;

    let (allowed, blocked): (Vec<_>, Vec<_>) = filters
        .into_iter()
        .filter(|df| df.is_regex)
        .partition_map(|df| {
            if df.is_allow {
                Either::Left(df.expr)
            } else {
                Either::Right(df.expr)
            }
        });
    info!("Allowed regex filters: {allowed:?}, Blocked regex filters: {blocked:?}");
    if let Ok(regex) = RegexSet::new(&allowed) {
        *ALLOWED.write().await = allowed;
        *REGEX_ALLOWED.write().await = regex;
    }
    if let Ok(regex) = RegexSet::new(&blocked) {
        *BLOCKED.write().await = blocked;
        *REGEX_BLOCKED.write().await = regex;
    }
    Ok(())
}

pub async fn check_filters(domain: impl AsRef<str>) -> Option<(bool, String)> {
    let domain = domain.as_ref();
    if let Some((allowed, reason)) = DOMAIN_FILTER.read().await.check(domain) {
        return Some((allowed, format!("Domain match: {reason}")));
    }

    let allow_match = REGEX_ALLOWED.read().await.matches(domain);
    if allow_match.len() > 0 {
        let guard = ALLOWED.read().await;
        let reason = allow_match.into_iter().map(|i| &guard[i]).join(", ");
        return Some((true, format!("Allowed regex: {reason}")));
    }

    let block_match = REGEX_BLOCKED.read().await.matches(domain);
    if block_match.len() > 0 {
        let guard = BLOCKED.read().await;
        let reason = block_match.into_iter().map(|i| &guard[i]).join(", ");
        return Some((false, format!("Blocked regex: {reason}")));
    }

    None
}

mod trie {
    use std::collections::HashMap;

    use itertools::Itertools;
    use log::debug;

    #[derive(Clone, Debug, Default)]
    pub struct NameTrie {
        names: HashMap<String, Name>,
    }

    #[derive(Clone, Debug)]
    struct Name {
        children: NameTrie,
        is_allow: Option<bool>,
    }

    impl NameTrie {
        pub fn create(list: impl IntoIterator<Item = (impl AsRef<str>, bool)>) -> Self {
            let mut trie = NameTrie::default();
            for (domain, is_allow) in list {
                let domain = domain.as_ref();
                let sub_names = Self::sub_names(domain);
                debug!("Inserting into trie {domain}/{is_allow} => {sub_names:?}");
                trie.insert(0, &sub_names, is_allow);
            }
            trie
        }

        fn insert(&mut self, idx: usize, sub_names: &[&str], is_allow: bool) {
            if idx == sub_names.len() {
                return;
            }

            if self.names.get(sub_names[idx]).is_none() {
                self.names.insert(
                    sub_names[idx].to_owned(),
                    Name {
                        children: NameTrie::default(),
                        is_allow: None,
                    },
                );
            }
            let name = self.names.get_mut(sub_names[idx]).unwrap();
            if idx == sub_names.len() - 1 {
                name.is_allow = Some(is_allow);
            }
            if name.is_allow.is_none() {
                name.children.insert(idx + 1, sub_names, is_allow);
            } else {
                // No, need to proceed, just drop the children which has lower priority anyways
                name.children.names.clear();
            }
        }

        pub fn count(&self) -> usize {
            let mut count = 0;
            for name in self.names.values() {
                if name.is_allow.is_some() {
                    count += 1;
                }
                count += name.children.count();
            }
            count
        }

        pub fn check(&self, name: impl AsRef<str>) -> Option<(bool, String)> {
            let sub_names = Self::sub_names(name.as_ref());
            let mut path = Vec::with_capacity(sub_names.len());
            let mut names = &self.names;
            for (idx, sub_name) in sub_names.into_iter().enumerate() {
                if !names.contains_key(sub_name) {
                    break;
                }

                if path.len() > idx {
                    path.pop().unwrap();
                }
                path.push(sub_name);

                let name = &names[sub_name];
                if let Some(is_allow) = name.is_allow {
                    return Some((is_allow, path.into_iter().rev().join(".")));
                }
                names = &name.children.names;
            }
            None
        }

        fn sub_names(name: &str) -> Vec<&str> {
            let mut split = name
                .split('.')
                .map(str::trim)
                .filter(|x| !x.is_empty())
                .collect::<Vec<_>>();
            split.reverse();
            split
        }
    }
}

#[cfg(test)]
mod test {
    use super::trie::NameTrie;

    #[test]
    fn test_create() {
        let trie = NameTrie::create([
            ("www.amazon.com", false),
            ("amazon.com", true),
            ("www.facebook.com", false),
            ("my-cdn.google.com", false),
        ]);
        println!("{trie:#?}");
        println!("Count {}", trie.count());
    }

    #[test]
    fn test_check() {
        let trie = NameTrie::create([
            ("www.amazon.com", false),
            ("amazon.com", true),
            ("www.facebook.com", false),
            ("my-cdn.google.com", false),
        ]);
        println!("{:?}", trie.check("my-cdn.google.com"));
        println!("{:?}", trie.check("my-cdn.amazon.com"));
        println!("{:?}", trie.check("www.amazon.com"));
        println!("{:?}", trie.check("facebook.com"));
        println!("{:?}", trie.check("loda.lahsun.www.facebook.com"));
    }
}
