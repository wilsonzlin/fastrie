use std::collections::HashMap;

#[cfg(test)]
mod tests;

pub struct FastrieBuilderNode<V> {
    built: bool,
    value: Option<V>,
    children: HashMap<u8, FastrieBuilderNode<V>>,
}

const IDX_BYTES: usize = 3;
const RESERVED_BYTE: u8 = 0xFF;
const MAX_CLUSTER_GAP_LEN: i16 = 3;

fn reserve_idx(vec: &mut Vec<u8>) -> usize {
    let pos = vec.len();
    for _ in 0..IDX_BYTES {
        vec.push(RESERVED_BYTE);
    };
    pos
}

fn encode_idx(idx: usize) -> [u8; IDX_BYTES] {
    assert!(idx <= 2usize.pow((8 * IDX_BYTES) as u32) - 1);
    [(idx >> 16) as u8, (idx >> 8) as u8, idx as u8]
}

fn decode_idx(encoded: &[u8]) -> usize {
    assert_eq!(encoded.len(), IDX_BYTES);
    ((encoded[0] as usize) << 16) | ((encoded[1] as usize) << 8) | (encoded[2] as usize)
}

fn write_idx(vec: &mut Vec<u8>, pos: usize, idx: usize) -> () {
    vec[pos..pos + IDX_BYTES].copy_from_slice(&encode_idx(idx))
}

fn push_idx(vec: &mut Vec<u8>, idx: usize) -> () {
    vec.extend_from_slice(&encode_idx(idx))
}

fn read_idx(data: &Vec<u8>, pos: usize) -> usize {
    decode_idx(&data[pos..pos + IDX_BYTES])
}

pub struct FastrieBuild<V> {
    pub values: Vec<V>,
    pub data: Vec<u8>,
}

impl<V> FastrieBuilderNode<V> {
    pub fn new() -> FastrieBuilderNode<V> {
        FastrieBuilderNode { built: false, value: None, children: HashMap::new() }
    }

    pub fn add(&mut self, pattern: &[u8], value: V) -> () {
        let mut current: &mut FastrieBuilderNode<V> = self;
        for c in pattern {
            if !current.children.contains_key(c) {
                current.children.insert(*c, FastrieBuilderNode::new());
            };
            current = current.children.get_mut(c).unwrap();
        };
        current.value = Some(value);
    }

    fn _build(&mut self, data: &mut Vec<u8>, values: &mut Vec<V>) -> () {
        assert!(!self.built);
        self.built = true;

        let value_idx: usize = if self.value.is_some() {
            // Index 0 is reserved.
            let idx = values.len() + 1;
            values.push(self.value.take().unwrap());
            idx
        } else { 0 };
        push_idx(data, value_idx);

        let mut child_chars = self.children.keys().map(|k| *k).collect::<Vec<u8>>();
        child_chars.sort();

        let mut child_char_clusters: Vec<Vec<Option<u8>>> = vec![];
        // Use i16 for:
        // - safe initial value that's guaranteed to cause new cluster creation;
        // - safe adding of `last_char + MAX_CLUSTER_GAP_LEN` without overflow; and
        // - safe calculation of `p - 1`.
        let mut last_char: i16 = std::i16::MIN;
        for c in child_chars {
            let p = c as i16;
            // Allow a maximum gap length of MAX_CLUSTER_GAP_LEN between any two children in a cluster.
            // Create a new cluster if it's the first char, or previous char in the current cluster is more than 3 character positions away.
            if last_char + MAX_CLUSTER_GAP_LEN < p {
                child_char_clusters.push(Vec::new());
            } else {
                // Fill any gaps with None values.
                for _ in last_char..p - 1 {
                    child_char_clusters.last_mut().unwrap().push(None);
                };
            };
            child_char_clusters.last_mut().unwrap().push(Some(c));
            last_char = p;
        };
        // Check largest first for faster performance on average.
        child_char_clusters.sort_by(|a, b| b.len().cmp(&a.len()));

        let mut replace_with_child_indices: HashMap<u8, usize> = HashMap::new();

        data.push(!self.children.is_empty() as u8);
        let mut last_cluster_next_cluster_dist_pos: Option<usize> = None;
        for cluster in &child_char_clusters {
            let cluster_pos = data.len();
            if let Some(out_pos) = last_cluster_next_cluster_dist_pos {
                write_idx(data, out_pos, cluster_pos);
            };
            last_cluster_next_cluster_dist_pos = Some(reserve_idx(data));
            let min = cluster.first().unwrap().unwrap();
            let max = cluster.last().unwrap().unwrap();
            data.push(min);
            data.push(max);
            for c in cluster {
                match c {
                    Some(c) => {
                        debug_assert!(!replace_with_child_indices.contains_key(c));
                        replace_with_child_indices.insert(*c, reserve_idx(data));
                    }
                    None => { push_idx(data, 0); }
                };
            };
        };
        if let Some(out_pos) = last_cluster_next_cluster_dist_pos {
            write_idx(data, out_pos, 0);
        };

        for cluster in &child_char_clusters {
            for c in cluster {
                if let Some(c) = c {
                    write_idx(data, *replace_with_child_indices.get(c).unwrap(), data.len());
                    let child_node = self.children.get_mut(c).unwrap();
                    child_node._build(data, values);
                };
            };
        };
    }

    pub fn prebuild(&mut self) -> FastrieBuild<V> {
        let mut data: Vec<u8> = Vec::new();
        let mut values: Vec<V> = Vec::new();
        self._build(&mut data, &mut values);
        FastrieBuild { values, data }
    }

    pub fn build(&mut self) -> Fastrie<V> {
        let FastrieBuild { values, data } = self.prebuild();
        Fastrie { values, data }
    }
}

pub struct Fastrie<V> {
    values: Vec<V>,
    data: Vec<u8>,
}

pub struct FastrieMatch<'v, V> {
    pub end: usize,
    pub value: &'v V,
}

impl<V> Fastrie<V> {
    pub fn precomputed(values: Vec<V>, data: Vec<u8>) -> Fastrie<V> {
        Fastrie { values, data }
    }

    pub fn memory_size(&self) -> usize {
        self.data.len()
    }

    pub fn longest_matching_prefix(&self, text: &[u8]) -> Option<FastrieMatch<V>> {
        let mut node_pos: usize = 0;
        let mut match_opt: Option<FastrieMatch<V>> = None;
        'outer: for (i, &c) in text.iter().enumerate() {
            if self.data[node_pos + IDX_BYTES] == 0 {
                // This node has no children.
                break;
            };

            let mut cluster_pos: usize = node_pos + IDX_BYTES + 1;
            loop {
                let next_cluster_pos = read_idx(&self.data, cluster_pos);
                let cluster_min: u8 = self.data[cluster_pos + IDX_BYTES];
                let cluster_max: u8 = self.data[cluster_pos + IDX_BYTES + 1];
                if c >= cluster_min && c <= cluster_max {
                    // Character is in this cluster, but it might point to a gap.
                    node_pos = read_idx(&self.data, cluster_pos + IDX_BYTES + 2 + ((c - cluster_min) as usize) * IDX_BYTES);
                    if node_pos == 0 {
                        // Character is not a child, as child node index is zero which means it's a gap.
                        break 'outer;
                    } else {
                        break;
                    };
                };
                if next_cluster_pos == 0 {
                    // Next cluster index is zero, which means this is last cluster.
                    break 'outer;
                };
                cluster_pos = next_cluster_pos;
            };

            // Get value of child node.
            let node_value_idx: usize = read_idx(&self.data, node_pos);
            if node_value_idx != 0 {
                match_opt = Some(FastrieMatch {
                    end: i,
                    value: &self.values[node_value_idx - 1],
                });
            };
        };

        match_opt
    }
}
