use crate::file_set::FileSet;
use crate::finalizable::DataSink;
use crate::stats::Stats;

pub trait MultiFilesWriterTarget {
    fn open_next_file(&mut self, full_path: &str) -> Result<(), String>;
    fn close_current_file(&mut self) -> Result<(), String>;
    fn write_to_current_file(&mut self, data: &[u8]) -> Result<(), String>;
    fn write_single_file(&self, path: &str, contents: &str) -> Result<(), String>;
}

pub struct Splitter<'a, T> {
    files_target: &'a mut T,
    chunk_sz: usize,
    file_set: FileSet,
    left_for_chunk: usize,
    next_chunk_no: usize
}

impl<'a, T: MultiFilesWriterTarget> Splitter<'a, T> {
    pub fn from_pattern(tgt: &'a mut T, chunk_size: usize, pattern: &'a str) -> Result<Splitter<'a, T>, String> {
        Ok(Self { 
            files_target: tgt, 
            chunk_sz: chunk_size, 
            file_set: FileSet::from_pattern(pattern)?,
            left_for_chunk: chunk_size, 
            next_chunk_no: 0
        })
    }

    pub fn write_metadata(self, stats: &Stats) -> Result<(), String> {
        self.files_target.write_single_file(
            self.file_set.cfg_path().as_str(),
            format!("\
                in_len={}\n\
                in_hash={:016x}\n\
                hash_seed={:016x}\n\
                xz_len={}\n\
                nr_chunks={}\n\
                chunk_len={}\n\
                auth={}\n\
                auth_len={}\n",
                stats.in_data_len.ok_or("in_data_len is missing")?,
                stats.in_data_hash.ok_or("in_data_hash is missing")?,
                stats.hash_seed.ok_or("hash_seed is missing")?,
                stats.compressed_len.ok_or("compressed_len is missing")?,
                self.next_chunk_no,
                stats.out_chunk_size.ok_or("out_chunk_size is missing")?,
                stats.auth_string, stats.auth_chunk_size,
            ).as_str())
    }
}

impl<'a, T: MultiFilesWriterTarget> DataSink for Splitter<'a, T> {
    fn add(&mut self, data: &[u8]) -> Result<(), String> {
        //eprintln!("Splitter: writing {} bytes", data.len());
        let mut left_for_data: usize = data.len();
        let mut offs_for_data = 0;
        while left_for_data > 0 {
            //eprintln!("  left for chunk before write: {}", self.left_for_chunk);
            if self.left_for_chunk == 0 || self.left_for_chunk == self.chunk_sz {
                if self.next_chunk_no > 0 {
                    self.files_target.close_current_file()?;
                }
                self.files_target
                    .open_next_file(self.file_set.gen_file_path(self.next_chunk_no).as_str())?;
                self.next_chunk_no += 1;
                self.left_for_chunk = self.chunk_sz;
            }
            let to_write = usize::min(left_for_data, self.left_for_chunk);
            //eprintln!("written {} bytes", to_write);
            self.files_target.write_to_current_file(&data[offs_for_data..offs_for_data + to_write])?;
            left_for_data -= to_write;
            offs_for_data += to_write;
            self.left_for_chunk -= to_write;
            //eprintln!("after write: left to chunk = {}, left for data = {}, offs_for_data = {}", self.left_for_chunk, left_for_data, offs_for_data);
        }

        Ok(())
    }

    fn finish(&mut self) -> Result<(), String> {
        //eprintln!("Splitter: finish");
        if self.next_chunk_no > 0 {
            self.files_target.close_current_file()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    //use std::io::Write;

    struct FilesEmulator {
        files: Vec<(String, Vec<u8>, bool)> // filename(0), data(1), is opened(2)
    }

    impl FilesEmulator {
        fn latest_file_ref(&mut self) -> Result<&mut (String, Vec<u8>, bool), String> {
            let last_idx = self.files.len() - 1;
            self.files.get_mut(last_idx).ok_or("no open files".to_owned())
        }
    }

    impl MultiFilesWriterTarget for FilesEmulator {
        fn open_next_file(&mut self, full_path: &str) -> Result<(), String> {
            self.files.push((full_path.to_owned(), Vec::new(), true));
            Ok(())
        }

        fn close_current_file(&mut self) -> Result<(), String> {
            let f = self.latest_file_ref()?;
            if !f.2 {
                return Err(format!("file {} is already closed", f.0));
            }
            f.2 = false;
            Ok(())
        }

        fn write_to_current_file(&mut self, data: &[u8]) -> Result<(), String> {
            let f = self.latest_file_ref()?;
            if !f.2 {
                return Err(format!("file {} is closed", f.0));
            }
            f.1.extend_from_slice(data);
            Ok(())
        }

        fn write_single_file(&self, path: &str, contents: &str) -> Result<(), String> {
            println!("writing single file {}:\n{}", path, contents);
            Ok(())
        }

    }

     fn assert_split(chunk_size: usize, data1: Vec<u8>, data2: Vec<u8>, expected: Vec<(&str, Vec<u8>)>) {
        let mut files = FilesEmulator{ files: Vec::new() };
        let mut spl = Splitter::<FilesEmulator>::from_pattern(&mut files, chunk_size, "out%%%.ext").unwrap();
        spl.add(data1.as_slice()).unwrap();
        spl.add(data2.as_slice()).unwrap();
        spl.finish().unwrap();
        spl.write_metadata(&Stats {
            in_data_len: Some(1), in_data_hash: Some(0x1234567812345678), 
            compressed_len: Some(2), hash_seed: Some(0x8765432187654321),
            out_chunk_size: Some(3), out_nr_chunks: Some(4), auth_chunk_size: 5, auth_string: "auth".to_owned()
        }).unwrap();
        let files = &files.files;
        assert_eq!(files.len(), expected.len());
        let mut it_exp = expected.iter();
        let mut it_act = files.iter();
        while let Some(exp) = it_exp.next() {
            let act = it_act.next().unwrap(); // SAFE because have same size
            assert_eq!(exp.0, act.0.as_str());
            assert_eq!(exp.1, act.1);
            assert!(!act.2);
        }
    }

    #[test]
    fn chunk4data33() {
        // data:    _____ _____
        //          1 2 3 4 5 6 
        // chunks: |       |   |
        assert_split(4, vec![1,2,3], vec![4,5,6], vec![
            ("out000.ext", vec![1,2,3,4]),
            ("out001.ext", vec![5,6]),
        ]);
    }

    #[test]
    fn chunk4data43() {
        // data:    _______ _____
        //          1 2 3 4 5 6 7
        // chunks: |       |     |
        assert_split(4, vec![1,2,3,4], vec![5,6,7], vec![
            ("out000.ext", vec![1,2,3,4]),
            ("out001.ext", vec![5,6,7]),
        ]);
    }

    #[test]
    fn chunk4data53() {
        // data:    _________ _____
        //          1 2 3 4 5 6 7 8
        // chunks: |       |       |
        assert_split(4, vec![1,2,3,4,5], vec![6,7,8], vec![
            ("out000.ext", vec![1,2,3,4]),
            ("out001.ext", vec![5,6,7,8]),
        ]);
    }

    #[test]
    fn chunk4data34() {
        // data:    _____ _______
        //          1 2 3 4 5 6 7
        // chunks: |       |     |
        assert_split(4, vec![1,2,3], vec![4,5,6,7], vec![
            ("out000.ext", vec![1,2,3,4]),
            ("out001.ext", vec![5,6,7]),
        ]);
    }

    #[test]
    fn chunk4data44() {
        // data:    _______ _______
        //          1 2 3 4 5 6 7 8
        // chunks: |       |       |
        assert_split(4, vec![1,2,3,4], vec![5,6,7,8], vec![
            ("out000.ext", vec![1,2,3,4]),
            ("out001.ext", vec![5,6,7,8]),
        ]);
    }

    #[test]
    fn chunk4data54() {
        // data:    _________ _______
        //          1 2 3 4 5 6 7 8 9
        // chunks: |       |       | |
        assert_split(4, vec![1,2,3,4,5], vec![6,7,8,9], vec![
            ("out000.ext", vec![1,2,3,4]),
            ("out001.ext", vec![5,6,7,8]),
            ("out002.ext", vec![9]),
        ]);
    }

    #[test]
    fn chunk4data35() {
        // data:    _____ _________
        //          1 2 3 4 5 6 7 8
        // chunks: |       |       |
        assert_split(4, vec![1,2,3], vec![4,5,6,7,8], vec![
            ("out000.ext", vec![1,2,3,4]),
            ("out001.ext", vec![5,6,7,8]),
        ]);
    }

    #[test]
    fn chunk4data45() {
        // data:    _______ _________
        //          1 2 3 4 5 6 7 8 9
        // chunks: |       |       | |
        assert_split(4, vec![1,2,3,4], vec![5,6,7,8,9], vec![
            ("out000.ext", vec![1,2,3,4]),
            ("out001.ext", vec![5,6,7,8]),
            ("out002.ext", vec![9]),
        ]);
    }

    #[test]
    fn chunk4data55() {
        // data:    _________ _________
        //          1 2 3 4 5 6 7 8 9 0
        // chunks: |       |       |   |
        assert_split(4, vec![1,2,3,4,5], vec![6,7,8,9,0], vec![
            ("out000.ext", vec![1,2,3,4]),
            ("out001.ext", vec![5,6,7,8]),
            ("out002.ext", vec![9,0]),
        ]);
    }
}
