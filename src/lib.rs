use std::collections::HashMap;

pub struct FastrieBuilderNode<V> {
    built: bool,
    children: HashMap<u8, FastrieBuilderNode<V>>,
    index_width: IndexWidth,
    value: Option<V>,
}

const RESERVED_BYTE: u8 = 0xFF;
const MAX_CLUSTER_GAP_LEN: i16 = 3;

/// How many bytes to store and represent indices in the built data. Must be between 1 and 8 inclusive. Indices will be encoded in little endian format.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct IndexWidth(pub usize);

impl IndexWidth {
  fn reserve_idx(self, vec: &mut Vec<u8>) -> usize {
      let pos = vec.len();
      for _ in 0..self.0 {
          vec.push(RESERVED_BYTE);
      };
      pos
  }

  fn write_idx(self, vec: &mut Vec<u8>, pos: usize, mut idx: usize) -> () {
    for i in 0..self.0 {
      vec[pos + i] = idx as u8;
      idx >>= 8;
    };
  }

  fn push_idx(self, vec: &mut Vec<u8>, idx: usize) -> () {
      let pos = self.reserve_idx(vec);
      self.write_idx(vec, pos, idx);
  }

  fn read_idx(self, data: &[u8], pos: usize) -> usize {
      let mut idx = 0usize;
      for i in 0..self.0 {
        idx <<= 8;
        idx |= data[pos + i] as usize;
      }
      idx
  }
}

pub struct FastrieBuild<V> {
    pub data: Vec<u8>,
    pub index_width: IndexWidth,
    pub values: Vec<V>,
}

impl<V> FastrieBuilderNode<V> {
    pub fn new(index_width: IndexWidth) -> FastrieBuilderNode<V> {
        FastrieBuilderNode {
          built: false,
          children: HashMap::new(),
          index_width,
          value: None,
        }
    }

    pub fn add(&mut self, pattern: &[u8], value: V) -> () {
        let mut current: &mut FastrieBuilderNode<V> = self;
        for c in pattern {
            if !current.children.contains_key(c) {
                current.children.insert(*c, FastrieBuilderNode::new(current.index_width));
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
        self.index_width.push_idx(data, value_idx);

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
                self.index_width.write_idx(data, out_pos, cluster_pos);
            };
            last_cluster_next_cluster_dist_pos = Some(self.index_width.reserve_idx(data));
            let min = cluster.first().unwrap().unwrap();
            let max = cluster.last().unwrap().unwrap();
            data.push(min);
            data.push(max);
            for c in cluster {
                match c {
                    Some(c) => {
                        debug_assert!(!replace_with_child_indices.contains_key(c));
                        replace_with_child_indices.insert(*c, self.index_width.reserve_idx(data));
                    }
                    None => { self.index_width.push_idx(data, 0); }
                };
            };
        };
        if let Some(out_pos) = last_cluster_next_cluster_dist_pos {
            self.index_width.write_idx(data, out_pos, 0);
        };

        for cluster in &child_char_clusters {
            for c in cluster {
                if let Some(c) = c {
                    self.index_width.write_idx(data, *replace_with_child_indices.get(c).unwrap(), data.len());
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
        FastrieBuild {
          data,
          index_width: self.index_width,
          values,
        }
    }
}

pub struct Fastrie<'v, 'd, V> {
    data: &'d [u8],
    index_width: IndexWidth,
    // If None, keys are used as a set.
    values: Option<&'v [V]>,
}

pub struct FastrieMatch<'v, V> {
    /// Inclusive.
    pub end: usize,
    pub value: &'v V,
}


/// # Example
///
/// ```
/// use fastrie::*;
///
/// let mut builder = FastrieBuilderNode::new(IndexWidth(1));
/// builder.add(b"hell", 1);
/// builder.add(b"hello", 2);
/// builder.add(b"world", 4);
/// let build = builder.prebuild();
///
/// // `build.data` can be written as bytes to a file, or embedded directly into code as a literal byte array/slice.
///
/// let trie = from_prebuilt_without_values(build.index_width, &build.data);
/// assert!(trie.contains_key(b"hell"));
/// assert!(trie.contains_key(b"hello"));
/// assert!(trie.contains_key(b"world"));
/// assert!(!trie.contains_key(b"worl"));
/// assert!(!trie.contains_key(b"worlds"));
/// ```
pub const fn from_prebuilt_without_values<'d>(index_width: IndexWidth, data: &'d [u8]) -> Fastrie<'_, 'd, ()> {
  Fastrie {
    data,
    index_width,
    values: None,
  }
}

impl<V> Fastrie<'_, '_, V> {
    /// # Example
    ///
    /// ```
    /// use fastrie::*;
    ///
    /// let mut builder = FastrieBuilderNode::new(IndexWidth(1));
    /// builder.add(b"hell", 1);
    /// builder.add(b"hello", 2);
    /// builder.add(b"world", 4);
    /// let build = builder.prebuild();
    ///
    /// // `build.data` can be written as bytes to a file, or embedded directly into code as a literal byte array/slice.
    ///
    /// let trie = Fastrie::from_prebuilt(build.index_width, &build.values, &build.data);
    /// assert!(trie.contains_key(b"hello"));
    /// let query = b"hello world!";
    /// let mat = trie.longest_matching_prefix(query).unwrap();
    /// assert_eq!(mat.end, 4);
    /// assert_eq!(&query[..=mat.end], b"hello");
    /// assert_eq!(mat.value, &2);
    /// let query = b"hell's kitchen";
    /// let mat = trie.longest_matching_prefix(query).unwrap();
    /// assert_eq!(mat.end, 3);
    /// assert_eq!(&query[..=mat.end], b"hell");
    /// assert_eq!(mat.value, &1);
    /// ```
    pub const fn from_prebuilt<'v, 'd>(index_width: IndexWidth, values: &'v [V], data: &'d [u8]) -> Fastrie<'v, 'd, V> {
        Fastrie {
          data,
          index_width,
          values: Some(values),
        }
    }

    pub fn memory_size(&self) -> usize {
        self.data.len()
    }

    fn _longest_matching_prefix(&self, text: &[u8]) -> Option<(usize, usize)> {
      let mut node_pos: usize = 0;
      let mut match_opt: Option<(usize, usize)> = None;
      'outer: for (i, &c) in text.iter().enumerate() {
          let idx_bytes = self.index_width.0;
          if self.data[node_pos + idx_bytes] == 0 {
              // This node has no children.
              break;
          };

          let mut cluster_pos: usize = node_pos + idx_bytes + 1;
          loop {
              let next_cluster_pos = self.index_width.read_idx(&self.data, cluster_pos);
              let cluster_min: u8 = self.data[cluster_pos + idx_bytes];
              let cluster_max: u8 = self.data[cluster_pos + idx_bytes + 1];
              if c >= cluster_min && c <= cluster_max {
                  // Character is in this cluster, but it might point to a gap.
                  node_pos = self.index_width.read_idx(&self.data, cluster_pos + idx_bytes + 2 + ((c - cluster_min) as usize) * idx_bytes);
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
          let node_value_idx: usize = self.index_width.read_idx(&self.data, node_pos);
          if node_value_idx != 0 {
              match_opt = Some((i, node_value_idx - 1));
          };
      };

      match_opt
    }

    pub fn contains_key(&self, key: &[u8]) -> bool {
      self._longest_matching_prefix(key).filter(|(i, _)| *i == key.len() - 1).is_some()
    }

    pub fn longest_matching_prefix(&self, text: &[u8]) -> Option<FastrieMatch<V>> {
      self._longest_matching_prefix(text).map(|(end, value_idx)| FastrieMatch {
          end,
          value: &self.values.unwrap()[value_idx],
      })
    }
}
