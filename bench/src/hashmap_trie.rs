use std::collections::HashMap;

pub struct HashMapTrieNode<V> {
    value: Option<V>,
    children: HashMap<u8, HashMapTrieNode<V>>,
}

pub struct HashMapTrieNodeMatch<'v, V> {
    pub end: usize,
    pub value: &'v V,
}

impl<V> HashMapTrieNode<V> {
    pub fn new() -> HashMapTrieNode<V> {
        HashMapTrieNode { value: None, children: HashMap::new() }
    }

    pub fn memory_size(&self) -> usize {
        self.children.capacity() + self.children
            .iter()
            .map(|(_, c)| c.memory_size())
            .sum::<usize>()
    }

    pub fn add(&mut self, pattern: &[u8], value: V) -> () {
        let mut current: &mut HashMapTrieNode<V> = self;
        for c in pattern {
            if !current.children.contains_key(c) {
                current.children.insert(*c, HashMapTrieNode::new());
            };
            current = current.children.get_mut(c).unwrap();
        };
        current.value = Some(value);
    }

    pub fn longest_matching_prefix(&self, text: &[u8]) -> Option<HashMapTrieNodeMatch<V>> {
        let mut node: &HashMapTrieNode<V> = self;
        let mut value: Option<HashMapTrieNodeMatch<V>> = None;
        for (i, c) in text.iter().enumerate() {
            match node.children.get(&c) {
                Some(child) => node = child,
                None => break,
            };
            match &node.value {
                Some(v) => value = Some(HashMapTrieNodeMatch { end: i, value: &v }),
                None => {}
            };
        };
        value
    }
}
