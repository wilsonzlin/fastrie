use fastrie::FastrieBuilderNode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::time::Instant;

use crate::direct_trie::DirectTrieNode;
use crate::hashmap_trie::HashMapTrieNode;

mod hashmap_trie;
mod direct_trie;

#[derive(Serialize, Deserialize)]
struct Entity {
    codepoints: Vec<u32>,
    characters: String,
}

fn read_json<T>(name: &str) -> T where for<'de> T: Deserialize<'de> {
    let file = File::open(format!("{}.json", name)).unwrap();
    serde_json::from_reader(file).unwrap()
}

macro_rules! time {
    ($name:literal, $size:expr, $code:block) => {{
        let start = Instant::now();
        $code;
        let duration = start.elapsed().as_secs_f64();
        println!("{:>15}: {:>10.5} seconds {:>10} size", $name, duration, $size);
    }};
}

fn test_large() {
    println!("test_large");
    let entities: HashMap<String, Entity> = read_json("entities");
    let mut fastrie_builder: FastrieBuilderNode<String> = FastrieBuilderNode::new();
    let mut hashmap_trie: HashMapTrieNode<String> = HashMapTrieNode::new();
    let mut direct_trie: DirectTrieNode<String> = DirectTrieNode::new();
    let mut entity_reps: Vec<Vec<u8>> = Vec::new();
    for (rep, Entity { characters, .. }) in entities.iter() {
        entity_reps.push(rep.as_bytes().to_vec());
        fastrie_builder.add(&rep.as_bytes(), characters.clone());
        hashmap_trie.add(&rep.as_bytes(), characters.clone());
        direct_trie.add(&rep.as_bytes(), characters.clone());
    };
    let fastrie = fastrie_builder.build();

    time!("fastrie", fastrie.memory_size(), {
        for _ in 0..100 {
            for rep in entity_reps.iter() {
                let _ = fastrie.longest_matching_prefix(rep.as_slice());
            };
        };
    });

    time!("hashmap_trie", hashmap_trie.memory_size(), {
        for _ in 0..100 {
            for rep in entity_reps.iter() {
                let _ = hashmap_trie.longest_matching_prefix(rep.as_slice());
            };
        };
    });

    time!("direct_trie", direct_trie.memory_size(), {
        for _ in 0..100 {
            for rep in entity_reps.iter() {
                let _ = direct_trie.longest_matching_prefix(rep.as_slice());
            };
        };
    });
}

fn test_small() {
    println!("test_small");
    let values: Vec<&[u8]> = vec![b"anne", b"ane", b"anna", b"ana", b"anene"];
    let mut fastrie_builder: FastrieBuilderNode<bool> = FastrieBuilderNode::new();
    let mut hashmap_trie: HashMapTrieNode<bool> = HashMapTrieNode::new();
    let mut direct_trie: DirectTrieNode<bool> = DirectTrieNode::new();
    for &v in values.iter() {
        fastrie_builder.add(v, true);
        hashmap_trie.add(v, true);
        direct_trie.add(v, true);
    };
    let fastrie = fastrie_builder.build();

    time!("fastrie", fastrie.memory_size(), {
        for _ in 0..100000 {
            for v in values.iter() {
                let _ = fastrie.longest_matching_prefix(v);
            };
        };
    });

    time!("hashmap_trie", hashmap_trie.memory_size(), {
        for _ in 0..100000 {
            for v in values.iter() {
                let _ = hashmap_trie.longest_matching_prefix(v);
            };
        };
    });

    time!("direct_trie", direct_trie.memory_size(), {
        for _ in 0..100000 {
            for v in values.iter() {
                let _ = direct_trie.longest_matching_prefix(v);
            };
        };
    });

    time!("manual", 0, {
        for _ in 0..100000 {
            for v in values.iter() {
                match v.get(0) {
                    Some(b'a') => match v.get(1) {
                        Some(b'n') => match v.get(2) {
                            Some(b'a') => true,
                            Some(b'e') => match v.get(3) {
                                Some(b'n') => match v.get(4) {
                                    Some(b'e') => true,
                                    _ => false,
                                },
                                _ => true,
                            },
                            Some(b'n') => match v.get(3) {
                                Some(b'a') => true,
                                Some(b'e') => true,
                                _ => false,
                            },
                            _ => false,
                        },
                        _ => false,
                    },
                    _ => false,
                };
            };
        };
    });
}

fn main() {
    test_large();
    test_small();
}
