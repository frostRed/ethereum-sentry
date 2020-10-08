use ethereum_types::H256;
use plain_hasher::PlainHasher;
use std::collections::{HashMap, HashSet};

pub type H256Map<T> = HashMap<H256, T, PlainHasher>;
pub type H256Set = HashSet<H256, PlainHasher>;
