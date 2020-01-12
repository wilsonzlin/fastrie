use fastrie::FastrieBuilderNode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
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

fn main() {
    let mut test_code = Vec::<u8>::new();
    let mut test_file = File::open("entities.html").unwrap();
    test_file.read_to_end(&mut test_code).unwrap();

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

    let start = Instant::now();
    for _ in 0..100 {
        for rep in entity_reps.iter() {
            let _ = fastrie.longest_matching_prefix(rep.as_slice()).unwrap();
        };
    };
    let duration = start.elapsed().as_secs_f64();
    println!("fastrie: {:?}, {} size", duration, fastrie.memory_size());

    let start = Instant::now();
    for _ in 0..100 {
        for rep in entity_reps.iter() {
            let _ = hashmap_trie.longest_matching_prefix(rep.as_slice()).unwrap();
        };
    };
    let duration = start.elapsed().as_secs_f64();
    println!("hashmap_trie: {:?} seconds, {} size", duration, hashmap_trie.memory_size());

    let start = Instant::now();
    for _ in 0..100 {
        for rep in entity_reps.iter() {
            let _ = direct_trie.longest_matching_prefix(rep.as_slice()).unwrap();
        };
    };
    let duration = start.elapsed().as_secs_f64();
    println!("direct_trie: {:?}, {} size", duration, direct_trie.memory_size());
}
