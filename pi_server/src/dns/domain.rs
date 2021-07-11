use std::cmp::Ordering;
use std::fmt::Display;
use std::fmt::Formatter;
use std::iter::FromIterator;
use std::time::Instant;

use trust_dns_proto::rr::Name;

use crate::Timer;

pub fn sort_domains<C: FromIterator<Domain>>(mut domains: Vec<Domain>) -> C {
    let start = Instant::now();
    let old_len = domains.len();

    let mut new_domains = Vec::with_capacity(old_len);
    domains.sort();
    for curr in domains {
        if let Some(prev) = new_domains.last() {
            if !curr.subdomain_of(prev) {
                new_domains.push(curr);
            }
        } else {
            new_domains.push(curr);
        }
    }
    log::info!(
        "Domain before sorting {}, after sorting {} in {}",
        old_len,
        new_domains.len(),
        start.t()
    );
    new_domains.into_iter().collect::<C>()
}

#[derive(Clone, Debug, Hash)]
pub struct Domain {
    labels: String,
}

impl Domain {
    pub fn from_name(name: &Name) -> Option<Self> {
        if name.num_labels() == 0 {
            return None;
        }
        Some(Domain {
            labels: name
                .to_lowercase()
                .iter()
                .rev()
                .map(|l| String::from_utf8_lossy(l))
                .collect::<Vec<_>>()
                .join("."),
        })
    }

    pub fn parse(name: impl AsRef<str>) -> Option<Self> {
        let name = name.as_ref().trim();
        if name.is_empty()
            || name
                .split('.')
                .map(|part| part.parse::<u32>())
                .any(|res| res.is_ok())
        {
            log::trace!("Invalid domain name: {}", name);
            return None;
        }
        Name::from_utf8(name)
            .ok()
            .map(|name| Self::from_name(&name))
            .flatten()
    }

    pub fn parent(&self) -> Option<Self> {
        let len = self.len();
        if len == 1 {
            return None;
        }
        Some(Domain {
            labels: self.labels().take(len - 1).collect::<Vec<_>>().join("."),
        })
    }

    pub fn name(&self) -> String {
        self.labels().rev().collect::<Vec<_>>().join(".")
    }

    pub fn labels(&self) -> impl DoubleEndedIterator<Item = &str> {
        self.labels.split('.')
    }

    pub fn len(&self) -> usize {
        self.labels().count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn subdomain_of(&self, other: &Self) -> bool {
        if self.len() < other.len() {
            return false;
        }
        other.labels().zip(self.labels()).all(|(o, s)| o == s)
    }
}

impl PartialEq for Domain {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        self.labels().zip(other.labels()).all(|(s, o)| s == o)
    }
}

impl Eq for Domain {}

impl PartialOrd for Domain {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let mut s_itr = self.labels();
        let mut o_itr = other.labels();
        for _ in 0..self.len().max(other.len()) {
            let s = s_itr.next();
            let o = o_itr.next();
            if let (Some(s), Some(o)) = (s, o) {
                match s.cmp(o) {
                    Ordering::Less => return Some(Ordering::Less),
                    Ordering::Greater => return Some(Ordering::Greater),
                    Ordering::Equal => continue,
                };
            } else {
                return if s.is_some() && o.is_none() {
                    Some(Ordering::Greater)
                } else {
                    Some(Ordering::Less)
                };
            }
        }
        None
    }
}

impl Ord for Domain {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

impl Display for Domain {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.labels)
    }
}

#[cfg(test)]
mod test {
    use std::net::Ipv6Addr;
    use std::path::Path;
    use std::time::Instant;

    use tokio::sync::Mutex;

    use crate::blocker::load_block_list;
    use crate::dns::domain::Domain;
    use crate::Timer;

    #[test]
    fn test1() {
        let mut doms = vec![
            Domain::parse("www.amazon.com").unwrap(),
            Domain::parse("amazon.com").unwrap(),
            Domain::parse("awsamazon.com").unwrap(),
            Domain::parse("uk.awsamazon.com").unwrap(),
            Domain::parse("awsamazon.co.uk").unwrap(),
        ];

        for dom1 in doms.iter() {
            for dom2 in doms.iter() {
                println!(
                    "{}[{}] < {}[{}]: {}",
                    dom1,
                    dom1.len(),
                    dom2,
                    dom2.len(),
                    dom1.subdomain_of(dom2)
                );
            }
        }
        println!("Before: {:?}", doms);
        doms.sort();
        println!("Sorted: {:?}", doms);
        let mut i = 1;
        while i < doms.len() {
            if doms[i].subdomain_of(&doms[i - 1]) {
                doms.remove(i);
            } else {
                i += 1;
            }
        }
        println!("Cleaned: {:?}", doms);
    }

    #[tokio::test]
    async fn test2() -> anyhow::Result<()> {
        crate::blocker::UPDATE_LOCK.set(Mutex::new(())).ok();
        let file = "block_list.txt";
        if !Path::new(file).exists() {
            return Ok(());
        }

        let start = Instant::now();
        let mut domains = load_block_list(file).await.unwrap();
        domains.sort();
        std::fs::write(
            "trie_dump.txt",
            domains
                .iter()
                .map(|d| d.to_string())
                .collect::<Vec<_>>()
                .join("\n"),
        )?;
        println!("Dumped set in {}", start.t());
        Ok(())
    }

    #[test]
    fn test3() {
        let mut doms = vec![
            Domain::parse("www.amazon.com").unwrap(),
            Domain::parse("facebook.com").unwrap(),
            Domain::parse("portal.facebook.com").unwrap(),
            Domain::parse("uk.awsamazon.com").unwrap(),
            Domain::parse("www.portal.facebook.com").unwrap(),
        ];
        doms.sort();
        println!("{:?}", doms);
        dbg!(doms[1].cmp(&doms[2]));
        dbg!(doms[2].cmp(&doms[1]));
    }

    #[test]
    fn test4() {
        let ip: Ipv6Addr = "::".parse().unwrap();
        println!("{:?}", ip);
    }
}
