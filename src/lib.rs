use std::convert::TryInto;
use std::collections::HashMap;

#[derive(Debug)]
pub struct FastrieBuilderNode<V> {
  built: bool,
  value: Option<V>,
  children: HashMap<u8, FastrieBuilderNode<V>>,
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

  fn _serialise(&mut self, out: &mut Vec<u8>, values: &mut Vec<V>) -> () {
    assert!(!self.built);
    self.built = true;
    let value_idx: usize = if self.value.is_some() {
      // Index 0 is reserved.
      let idx = values.len() + 1;
      values.push(self.value.take().unwrap());
      idx
    } else { 0 };
    assert!(value_idx <= 0xFFFF);
    // Push value index data.
    out.push((value_idx >> 8) as u8);
    out.push(value_idx as u8);

    let mut child_chars = self.children.keys().map(|k| *k).collect::<Vec<u8>>();
    child_chars.sort();

    let mut child_char_clusters: Vec<Vec<Option<u8>>> = vec![];
    let mut last_char: Option<u8> = None;
    for c in child_chars {
        // Allow a maximum gap length of 3 between any two children in a cluster.
        // Create a new cluster if it's the first char, or previous char in the current cluster is more than 3 character positions away.
        if last_char.filter(|last| last + 3 >= c).is_none() {
            child_char_clusters.push(Vec::new());
        } else if c > 0 {
            // Fill any gap with None values.
            for _ in last_char.unwrap()..c - 1 {
                child_char_clusters.last_mut().unwrap().push(None);
            };
        };
        child_char_clusters.last_mut().unwrap().push(Some(c));
        last_char = Some(c);
    };
    // Check largest first for faster performance on average.
    child_char_clusters.sort_by(|a, b| b.len().cmp(&a.len()));

    let mut replace_with_child_distances: HashMap<u8, usize> = HashMap::new();

    out.push(!self.children.is_empty() as u8);
    let mut last_cluster_next_cluster_dist_pos: Option<usize> = None;
    for cluster in &child_char_clusters {
      let cluster_pos = out.len();
      if let Some(out_pos) = last_cluster_next_cluster_dist_pos {
        out[out_pos] = (cluster_pos - out_pos).try_into().unwrap();
      };
      last_cluster_next_cluster_dist_pos = Some(out.len());
      out.push(0xFF);
      let min = cluster.first().unwrap().unwrap();
      let max = cluster.last().unwrap().unwrap();
      out.push(min);
      out.push(max);
      for c in cluster {
        if let Some(c) = c {
          assert!(!replace_with_child_distances.contains_key(c));
          replace_with_child_distances.insert(*c, out.len());
        };
        out.push(0xFF);
      };
    };
    if let Some(out_pos) = last_cluster_next_cluster_dist_pos {
      out[out_pos] = 0
    };

    for cluster in &child_char_clusters {
      for c in cluster {
        if let Some(c) = c {
          let out_pos = *replace_with_child_distances.get(c).unwrap();
          out[out_pos] = (out.len() - out_pos).try_into().unwrap();
          let child_node = self.children.get_mut(c).unwrap();
          child_node._serialise(out, values);
        };
      };
    };
  }

  pub fn build(&mut self) -> Fastrie<V> {
    let mut out: Vec<u8> = Vec::new();
    let mut values: Vec<V> = vec![];
    self._serialise(&mut out, &mut values);
    Fastrie { values, data: out }
  }
}

pub struct Fastrie<V> {
  values: Vec<V>,
  data: Vec<u8>,
}

impl<V> Fastrie<V> {
  pub fn greedy(&self, text: &[u8]) -> Option<&V> {
    let mut node_pos: usize = 0;
    let mut value_idx: usize = 0;
    // Use custom iteration variable to allow one final iteration at end of text.
    // It's possible that the last char in text matched a node, so one more iteration is needed to retrieve the value.
    let mut i: usize = 0;
    'outer: loop {
      let node_value_idx: usize = ((self.data[node_pos] as usize) << 8) | self.data[node_pos + 1] as usize;
      if node_value_idx != 0 {
        value_idx = node_value_idx - 1;
      };
      if i == text.len() || self.data[node_pos + 2] == 0 {
        // There is no more text to match or this node has no children.
        break;
      };
      let c = text[i];
      let mut cluster_pos: usize = node_pos + 3;
      loop {
        let next_cluster_pos = cluster_pos + self.data[cluster_pos] as usize;
        let cluster_min: u8 = self.data[cluster_pos + 1];
        let cluster_max: u8 = self.data[cluster_pos + 2];
        if c >= cluster_min && c <= cluster_max {
          // Character is in this cluster.
          node_pos = cluster_pos + 3 + (c - cluster_min) as usize + self.data[cluster_pos + 3 + (c - cluster_min) as usize] as usize;
          break;
        };
        if cluster_pos == next_cluster_pos {
          // Next cluster distance is zero, which means this is last cluster.
          break 'outer;
        };
        cluster_pos = next_cluster_pos;
      };
      i += 1;
    };
    if value_idx != 0 {
      Some(&self.values[value_idx])
    } else {
      None
    }
  }
}
