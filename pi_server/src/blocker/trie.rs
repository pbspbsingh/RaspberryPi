use std::collections::HashMap;

use trust_dns_proto::rr::Name;

#[derive(Clone, Debug)]
pub struct Trie {
    children: HashMap<Vec<u8>, Trie>,
    valid: bool,
}

impl Trie {
    pub fn new() -> Self {
        Trie {
            children: HashMap::new(),
            valid: false,
        }
    }

    pub fn push(&mut self, name: &Name) {
        if name.num_labels() == 0 {
            return;
        }

        let mut curr = self;
        for label in &reverse_labels(name) {
            if !curr.children.contains_key(label) {
                curr.children.insert(
                    label.clone(),
                    Trie {
                        children: HashMap::with_capacity(0),
                        valid: false,
                    },
                );
            }

            curr = curr.children.get_mut(label).unwrap();
            if curr.valid {
                break;
            }
        }
        curr.valid = true;
        curr.children.clear();
    }

    pub fn contains(&self, name: &Name) -> bool {
        if name.num_labels() == 0 {
            return false;
        }

        let mut curr = self;
        for label in &reverse_labels(name) {
            if curr.valid || !curr.children.contains_key(label) {
                break;
            }
            curr = &curr.children[label];
        }
        curr.valid
    }

    pub fn shrink(&mut self) {
        self.children.shrink_to_fit();
        self.children.values_mut().for_each(|c| c.shrink());
    }

    pub fn len(&self) -> u32 {
        let self_len: u32 = if self.valid { 1 } else { 0 };
        let children_len: u32 = self.children.values().map(Trie::len).sum();
        return self_len + children_len;
    }

    pub fn dump(&self) -> String {
        fn _dump(trie: &Trie, temp: &mut Vec<String>, output: &mut Vec<String>) {
            if trie.valid {
                output.push(temp.join("."));
            } else {
                for (label, child) in &trie.children {
                    temp.push(String::from_utf8_lossy(label).to_string());
                    _dump(child, temp, output);
                    temp.pop();
                }
            }
        }
        let mut output = Vec::new();
        _dump(&self, &mut Vec::with_capacity(5), &mut output);
        output.sort();
        output.join("\n")
    }
}

fn reverse_labels(name: &Name) -> Vec<Vec<u8>> {
    name.to_lowercase()
        .iter()
        .map(|v| v.to_vec())
        .rev()
        .collect()
}

#[cfg(test)]
mod test_trie {
    use std::path::Path;
    use std::str::FromStr;
    use std::time::Instant;

    use tokio::fs::File;
    use tokio::io::{AsyncBufReadExt, BufReader};
    use trust_dns_proto::rr::Name;

    use crate::blocker::{is_valid_domain, Trie};

    #[test]
    fn test() {
        let mut trie = Trie::new();
        trie.push(&name("star.c10r.facebook.com."));
        trie.push(&name("star.c10r.facebook.com."));
        trie.push(&name("www.amazon.com"));
        trie.push(&name("amazon.com"));
        trie.push(&name("amazon.co.uk"));
        trie.push(&name("www.amazon.co.uk"));

        println!("Count: {}", trie.len());

        assert_eq!(trie.contains(&name("com")), false);
        assert_eq!(trie.contains(&name("amazon.com")), true);
        assert_eq!(trie.contains(&name("www5.amazon.com")), true);
        assert_eq!(trie.contains(&name("portal.amazon.com")), true);
        assert_eq!(trie.contains(&name("portal.aws.amazon.com")), true);
        assert_eq!(trie.contains(&name("facebook.com")), false);
    }

    fn name(url: &str) -> Name {
        Name::from_utf8(url).unwrap()
    }

    #[tokio::test]
    async fn test2() -> anyhow::Result<()> {
        if !Path::new("block_list.txt").exists() {
            return Ok(());
        }

        let start = Instant::now();
        let mut trie = Trie::new();
        let mut buff = String::with_capacity(100);
        let mut reader = BufReader::new(File::open("block_list.txt").await?);
        loop {
            buff.clear();
            if reader.read_line(&mut buff).await? <= 0 {
                break;
            }

            let mut line = buff.trim();
            if line.is_empty() || line.starts_with("#") {
                continue;
            }
            if let Some(idx) = line.find("#") {
                line = &line[..idx];
            }
            line = line.trim();
            if line.is_empty() {
                continue;
            }
            line.split_ascii_whitespace()
                .map(str::trim)
                .filter(|&token| "localhost" != token)
                .filter(|&token| is_valid_domain(token))
                .filter_map(|token| Name::from_str(token).ok())
                .for_each(|name| trie.push(&name));
        }
        println!("Trie size: {}, {}s", trie.len(), start.elapsed().as_secs());
        let start = Instant::now();
        std::fs::write("trie_dump.txt", trie.dump())?;
        println!("Dumped trie in {}ms", start.elapsed().as_millis());
        Ok(())
    }
}
