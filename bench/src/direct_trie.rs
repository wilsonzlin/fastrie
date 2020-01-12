pub struct DirectTrieNode<V> {
    value: Option<V>,
    children: Vec<Option<DirectTrieNode<V>>>,
}

pub struct DirectTrieNodeMatch<'v, V> {
    pub end: usize,
    pub value: &'v V,
}

impl<V> DirectTrieNode<V> {
    pub fn new() -> DirectTrieNode<V> {
        let mut node = DirectTrieNode { value: None, children: Vec::with_capacity(256) };
        for _ in 0..256 {
            node.children.push(None);
        };
        node
    }

    pub fn memory_size(&self) -> usize {
        1 + self.children
            .iter()
            // Increase by one for size of Option.
            .map(|c| 1 + match c {
                Some(n) => n.memory_size(),
                None => 0,
            })
            .sum::<usize>()
    }

    pub fn add(&mut self, pattern: &[u8], value: V) -> () {
        if pattern.len() == 0 {
            self.value = Some(value);
        } else {
            let c = pattern[0];
            if let None = self.children[c as usize] {
                self.children[c as usize] = Some(DirectTrieNode::new());
            };
            self.children[c as usize].as_mut().unwrap().add(&pattern[1..], value);
        };
    }

    pub fn longest_matching_prefix(&self, text: &[u8]) -> Option<DirectTrieNodeMatch<V>> {
        let mut node: &DirectTrieNode<V> = self;
        let mut value: Option<DirectTrieNodeMatch<V>> = None;
        for (i, &c) in text.iter().enumerate() {
            match &node.children[c as usize] {
                Some(child) => node = &child,
                None => break,
            };
            match &node.value {
                Some(v) => value = Some(DirectTrieNodeMatch { end: i, value: &v }),
                None => {}
            };
        };
        value
    }
}
