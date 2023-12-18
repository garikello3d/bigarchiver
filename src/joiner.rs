use crate::patterns::{gen_chunk_path, analyze_pattern, pattern_from_cfg};
use crate::finalizable::DataSink;
use crate::stats::Stats;

pub trait MultiFilesReaderSource {
    fn open_next_file(&mut self, full_path: &str) -> Result<bool, String>;
    fn read_from_current_file(&mut self, buf: &mut [u8]) -> Result<usize, String>;
    fn close_current_file(&mut self, ) -> Result<(), String>;
    fn read_single_file(full_path: &str) -> Result<Vec<u8>, String>;
}

pub struct Joiner<'a, T: DataSink, R: MultiFilesReaderSource> {
    from: R,
    to: &'a mut T,
    file_patt: String,
    max_read_buf_size: usize,
    next_chunk_no: usize,
    patt_offset: usize,
    patt_length: usize
}

impl <'a, T: DataSink, R: MultiFilesReaderSource> Joiner<'a, T, R> {
    pub fn from_pattern(read_from: R, write_to: &'a mut T, pattern: &'a str, max_read_buf_size: usize) -> Result<Self, String> {
        let (offset, length) = analyze_pattern(pattern)?;
        Ok(Self { 
            from: read_from, 
            to: write_to,
            file_patt: pattern.to_owned(),
            max_read_buf_size,
            next_chunk_no: 0, 
            patt_offset: offset, 
            patt_length: length
        })
    }

    pub fn from_metadata(read_from: R, write_to: &'a mut T, metadata_path: &'a str, max_read_buf_size: usize) -> Result<Self, String> {
        let pattern = pattern_from_cfg(metadata_path)?;
        let (offset, length) = analyze_pattern(&pattern)?;
        Ok(Self { 
            from: read_from, 
            to: write_to,
            file_patt: pattern,
            max_read_buf_size,
            next_chunk_no: 0, 
            patt_offset: offset, 
            patt_length: length
        })
    }

    pub fn read_and_write_all(&mut self) -> Result<(), String> {
        let mut read_buf: Vec<u8> = Vec::with_capacity(self.max_read_buf_size);
        read_buf.resize(self.max_read_buf_size, 0);

        loop {
            if self.next_chunk_no != 0 {
                self.from.close_current_file()?;
            }

            let opened_or_not_found = self.from.open_next_file(
                gen_chunk_path(
                    &self.file_patt, self.next_chunk_no, self.patt_offset, self.patt_length)?
                    .as_str()
                ).map_err(|e| format!("could not read chunk #{} for pattern {}: {}", self.next_chunk_no, self.file_patt, e))?;

            if !opened_or_not_found {
                if self.next_chunk_no == 0 { // first chunk must exist - otherwise it's a fatal error
                    return Err(format!("could find first chunk for pattern {}", self.file_patt));
                } else { // further chunk not found -> treat it as end of everything
                    break;
                }
            }
            else {
                self.next_chunk_no += 1;
            }

            let mut eof = false;

            while !eof {
                let mut left_for_buf = self.max_read_buf_size;
                let mut buf_offs = 0;

                while left_for_buf > 0 {
                    let mut buf = &mut read_buf[buf_offs..];
                    let bytes_read: usize = self.from.read_from_current_file(&mut buf)?;
                    if bytes_read == 0 {
                        eof = true;
                        break; // exhausted current chunk, will move to the next (if any)
                    }
                    left_for_buf -= bytes_read;
                    buf_offs += bytes_read;
                }

                if buf_offs > 0 {
                    let _ = self.to.add(&read_buf[..buf_offs]).map_err(|e| format!("target write error of {} bytes: {}", buf_offs, e))?;
                }
            }
        }

        self.to.finish().map_err(|e| format!("finalization error: {}", e))?;

        Ok(())
    }
}

pub fn read_metadata<R: MultiFilesReaderSource>(metadata_path: &str) -> Result<Stats, String> {
    let mut stats = Stats::new();
    let s = String::from_utf8(
        R::read_single_file(metadata_path)?)
        .map_err(|e|format!("metadata file doesn't contain valid utf8 data: {}", e))?;
    for line in s.split("\n").map(|ln| ln.trim()).filter(|ln| ln.len() > 0) {
        let param_val = line.split("=").collect::<Vec<&str>>();
        if param_val.len() != 2 {
            return Err(format!("invalid metadata line: '{}'", line));
        }
        let param = param_val[0].trim();
        let val = param_val[1].trim();
        match param {
            "in_len" => stats.in_data_len = Some(val.parse::<usize>().map_err(|_| "error reading 'in_len'")?),
            "in_hash" => stats.in_data_hash = Some(u64::from_str_radix(val, 16).map_err(|_| "error reading 'in_hash'")?),
            "hash_seed" => stats.hash_seed = Some(u64::from_str_radix(val, 16).map_err(|_| "error reading 'hash_seed'")?),
            "xz_len" => stats.compressed_len = Some(val.parse::<usize>().map_err(|_| "error reading 'xz_len'")?),
            "nr_chunks" => stats.out_nr_chunks = Some(val.parse::<usize>().map_err(|_| "error reading 'nr_chunks'")?),
            "chunk_len" => stats.out_chunk_size = Some(val.parse::<usize>().map_err(|_| "error reading 'chunk_len'")?),
            "auth_len" => stats.auth_chunk_size = val.parse::<usize>().map_err(|_| "error reading 'auth_len'")?,
            "auth" => stats.auth_string = val.to_string(),
            _ => return Err(format!("unknown field '{}'", param))
        }    
    }
    if stats.all_set() {
        Ok(stats)
    } else {
        Err("not all metadata fields are provided".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, HashSet};
    use crate::{joiner::{Joiner, MultiFilesReaderSource, read_metadata}, finalizable::DataSink};
    use crate::stats::Stats;
    use rand::{thread_rng, Rng, RngCore};

    #[derive(Debug)]
    struct TestReaderSource {
        data: BTreeMap<String, (Vec<u8>, Option<usize>)>,
        failed_files: HashSet<String>
    }

    impl MultiFilesReaderSource for TestReaderSource {
        fn open_next_file(&mut self, full_path: &str) -> Result<bool, String> {
            //eprintln!("open_next_file({full_path})");
            if self.failed_files.contains(full_path) {
                return Err("error opening file".to_owned());
            }
            if let Some((file_name, _ )) = self.data.iter().find(|d| d.1.1.is_some()) {
                return Err(format!("file {} already opened", file_name));
            }
            if let Some((_, offs)) = self.data.get_mut(full_path) {
                assert!(offs.is_none());
                *offs = Some(0);
                Ok(true)
            }
            else {
                return Ok(false);
            }
        }

        fn read_from_current_file(&mut self, buf: &mut [u8]) -> Result<usize, String> {
            //eprintln!("read_from_current_file([ {} bytes buf ])", buf.len());
            let (_file_name, (data, opt_offs)) = self.data.iter_mut()
                .find(|d| d.1.1.is_some())
                .ok_or("no opened files to read from".to_owned())?;
            let offs = opt_offs.unwrap(); // SAFE because otherwise would've returned from line above
            if offs == data.len() {
                Ok(0) // reached end-of-file
            } else {
                let to_read = usize::min(buf.len(), data.len() - offs);
                buf[..to_read].copy_from_slice(&data[offs..offs+to_read]);
                *opt_offs = Some(offs + to_read);
                Ok(to_read)
            }
        }

        fn close_current_file(&mut self) -> Result<(), String> {
            //eprintln!("close_current_file()");
            let (_, (_, offs)) = self.data.iter_mut()
                .find(|d| d.1.1.is_some())
                .ok_or("no opened files to close".to_owned())?;
            *offs = None;
            Ok(())
        }

        fn read_single_file(_: &str) -> Result<Vec<u8>, String> {
            Ok(b"\
            in_len=12345\n\
            in_hash=abcde\n\
            hash_seed=edcba\n\
            xz_len=54321\n\
            nr_chunks=1\n\
            chunk_len=2\n\
            auth=Author Name\n\
            auth_len=3".to_vec())    
        }
    }

    struct TestReaderTarget {
        data: Vec<u8>
    }

    impl TestReaderTarget {
        fn new() -> Self { Self { data: Vec::new() } }
    }

    impl DataSink for TestReaderTarget {
        fn add(&mut self, data: &[u8]) -> Result<(), String> {
            self.data.extend_from_slice(data);
            Ok(())
        }
        fn finish(&mut self) -> Result<(), String> {
            Ok(())
        }
    }

    #[test]
    fn test_test_reader() {
        let mut tr = TestReaderSource{ 
            data: BTreeMap::from([
                ("f1".to_owned(), (vec![1,2,3], None)),
                ("f2".to_owned(), (vec![1,2,3], None)),
                ("f3".to_owned(), (vec![1,2,3], None)),
                ("f4".to_owned(), (vec![1,2,3], None)),
                ("f5".to_owned(), (vec![1,2,3], None)),
                ("f6".to_owned(), (vec![1,2,3], None)),
                ]), 
            failed_files: HashSet::new() };

        for (fname, file_exp_data) in [
            ("f1", vec![(1, vec![1]), (1, vec![2]), (1, vec![3])]),
            ("f2", vec![(2, vec![1,2]), (1, vec![3])]),
            ("f3", vec![(1, vec![1]), (2, vec![2,3])]),
            ("f4", vec![(2, vec![1,2]), (2, vec![3])]),
            ("f5", vec![(3, vec![1,2,3])]),
            ("f6", vec![(4, vec![1,2,3])]),
        ]
        {
            assert!(tr.open_next_file(fname).unwrap());
            for (requested_len, exp_data) in file_exp_data {
                //eprintln!("requested len = {}, exp_data = {:?}", requested_len, exp_data);
                let mut act_data: Vec<u8> = Vec::with_capacity(requested_len);
                act_data.resize(requested_len, 0);
                let bytes_act_read = tr.read_from_current_file(&mut act_data).unwrap();
                assert_eq!(bytes_act_read, exp_data.len());
                assert_eq!(act_data[..bytes_act_read], exp_data);
            }
            tr.close_current_file().unwrap();
        }
    }

    #[test]
    fn first_open_err() {
        { // error opening
            let src = TestReaderSource{ data: BTreeMap::new(), failed_files: HashSet::from(["failed_file".to_owned()]) };
            let mut dst = TestReaderTarget::new();
            let mut j = Joiner::from_pattern(src, &mut dst, "file%%%", 3).unwrap();
            let r = j.read_and_write_all();
            assert!(r.is_err());
            assert!(dst.data.is_empty());
        }
        { // not found
            let src = TestReaderSource{ data: BTreeMap::new(), failed_files: HashSet::new() };
            let mut dst = TestReaderTarget::new();
            let mut j = Joiner::from_pattern(src, &mut dst, "file%%%", 3).unwrap();
            let r = j.read_and_write_all();
            assert!(r.is_err());
            assert!(dst.data.is_empty());
        }
    }

    #[test]
    fn two_small_chunks_ok() {
        let src = TestReaderSource{ data: BTreeMap::from([
            ("f00".to_owned(), (vec![1,2], None)),
            ("f01".to_owned(), (vec![3], None)),
            ]), failed_files: HashSet::new() };
        let mut dst = TestReaderTarget::new();
        let mut j = Joiner::from_pattern(src, &mut dst, "f%%", 3).unwrap();
        j.read_and_write_all().unwrap();
        assert_eq!(dst.data, vec![1,2,3]);
    }

    #[test]
    fn two_big_chunks_ok() {
        let src = TestReaderSource{ data: BTreeMap::from([
            ("f00".to_owned(), (vec![1,2,3,4,5], None)),
            ("f01".to_owned(), (vec![6,7,8,9], None)),
            ]), failed_files: HashSet::new() };
        let mut dst = TestReaderTarget::new();
        let mut j = Joiner::from_pattern(src, &mut dst, "f%%", 3).unwrap();
        j.read_and_write_all().unwrap();
        assert_eq!(dst.data, vec![1,2,3,4,5,6,7,8,9]);
    }

    #[test]
    fn two_small_chunks_ok_last_bad() {
        let src = TestReaderSource{ data: BTreeMap::from([
            ("f00".to_owned(), (vec![1,2], None)),
            ("f01".to_owned(), (vec![3], None)),
            ]), failed_files: HashSet::from(["f02".to_owned()]) };
        let mut dst = TestReaderTarget::new();
        let mut j = Joiner::from_pattern(src, &mut dst, "f%%", 3).unwrap();
        j.read_and_write_all().unwrap_err();
    }

    fn random_chunks(src_len: usize, chunk_max_len: usize, max_read: usize) {
        let mut src_stream: Vec<u8> = Vec::with_capacity(src_len);
        src_stream.resize(src_len, 0);
        thread_rng().fill_bytes(&mut src_stream);

        let mut left_from_src = src_stream.len();
        let mut src_offs = 0;
        let mut chunk_cnt = 0;
        let mut src = TestReaderSource{ data: BTreeMap::new(), failed_files: HashSet::new() };
        while left_from_src != 0 {
            let chunk_len = thread_rng().gen::<usize>() % (chunk_max_len - 1) + 1;
            let chunk_len = usize::min(chunk_len, left_from_src);
            let portion = &src_stream[src_offs..src_offs + chunk_len];
            src.data.insert(format!("f{:09}", chunk_cnt), (Vec::from(portion), None));
            chunk_cnt += 1;
            left_from_src -= chunk_len;
            src_offs += chunk_len;
        }

        let chunks_concated = src.data
            .iter()
            .fold(Vec::new(), |acc, x| {
                let mut prev = acc.clone();
                prev.extend_from_slice(&x.1.0);
                prev
        } );
        assert_eq!(src_stream, chunks_concated);

        let mut target = TestReaderTarget::new();
        let mut j = Joiner::from_pattern(src, &mut target, "f%%%%%%%%%", max_read).unwrap();
        j.read_and_write_all().unwrap();
        assert_eq!(target.data, src_stream);
    }

    #[test]
    fn various_random_chunks() {
        for c in [10, 50, 100] {
            for m in [10, 50, 100] {
                random_chunks(1000, c, m);
            }
        }
    }

    #[test]
    fn read_meta() {
        assert_eq!(
            read_metadata::<TestReaderSource>("").unwrap(),
            Stats {
                in_data_len: Some(12345),
                in_data_hash: Some(0xabcde),
                hash_seed: Some(0xedcba),
                compressed_len: Some(54321),
                out_nr_chunks: Some(1),
                out_chunk_size: Some(2),
                auth_string: "Author Name".to_owned(),
                auth_chunk_size: 3
            }
        );
    }
}
