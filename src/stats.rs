#[derive(Default, PartialEq, Eq, Debug)]
pub struct Stats {
    pub in_data_len: Option<usize>,
    pub in_data_hash: Option<u64>,
    pub hash_seed: Option<u64>,
    pub compressed_len: Option<usize>,
    pub out_nr_chunks: Option<usize>,
    pub out_chunk_size: Option<usize>,
    pub auth_string: String,
    pub auth_chunk_size: usize,
}

impl Stats {
    pub fn new() -> Self {
        Self { ..Default::default() }
    }

    pub fn all_set(&self) -> bool {
        self.in_data_hash.is_some() && self.in_data_len.is_some() && 
        self.hash_seed.is_some() && self.compressed_len.is_some() && 
        self.out_nr_chunks.is_some() && self.out_chunk_size.is_some() && 
        !self.auth_string.is_empty() && self.auth_chunk_size > 0
    }
}
