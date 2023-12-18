use std::hash::Hasher;
use twox_hash::{xxh3::Hash128, Xxh3Hash128};
use crate::finalizable::DataSink;

pub struct DataHasher<'a, T: DataSink> {
    write_to: Option<&'a mut T>,
    hasher: Hash128,
    counter: usize,
}

// transparently copies data to `Writer`, calculaing hash in the mean time
impl<'a, T: DataSink> DataHasher<'a, T> {
    pub fn with_writer(to: Option<&'a mut T>, seed: u64) -> DataHasher<'a, T> {
        DataHasher { write_to: to, hasher: Xxh3Hash128::with_seed(seed), counter: 0 }
    }

    pub fn result(&self) -> u64 {
        self.hasher.finish()
    }

    pub fn counter(&self) -> usize {
        self.counter
    }
}

impl<'a, T: DataSink> DataSink for DataHasher<'a, T> {
    fn add(&mut self, data: &[u8]) -> Result<(), String> {
        //eprintln!("DataHasher: writing {} bytes", data.len());
        self.hasher.write(data);
        self.counter += data.len();
        if let Some(write_to) = self.write_to.as_mut() {
            write_to.add(data)
        } else {
            Ok(())
        }
    }

    fn finish(&mut self) -> Result<(), String> {
        //eprintln!("DataHasher: finish");
        if let Some(write_to) = &mut self.write_to {
            write_to.finish()
        } else {
            Ok(())
        }
    }
}
