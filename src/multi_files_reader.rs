use crate::joiner::MultiFilesReaderSource;
use std::fs::File;
use std::io::Read;

pub struct MultiFilesReader {
    file: Option<(File, String)>
}

impl MultiFilesReader {
    pub fn new() -> Self {
        Self { file : None }
    }
}

impl MultiFilesReaderSource for MultiFilesReader {
    fn open_next_file(&mut self, full_path: &str) -> Result<bool, String> {
        if let Some((_, s)) = &self.file {
            return Err(format!("previous file {} was not closed before opening a new file {}", s, full_path));
        }
        let (opt_f, ret) = 
            match File::open(full_path) {
                Ok(f) => (Some((f, full_path.to_owned())), true),
                Err(e) => {
                    match e.kind() {
                        std::io::ErrorKind::NotFound => (None, false),
                        _ => { return Err(format!("could not open file {}: {}", full_path, e)); }
                    }
                }
            };
        self.file = opt_f;
        eprintln!("reading from {}", full_path);
        Ok(ret)
    }

    fn read_from_current_file(&mut self, buf: &mut [u8]) -> Result<usize, String> {
        let (file, name) = self.file
            .as_mut()
            .ok_or("no current file opened to read from".to_owned())?;
        file
            .read(buf)
            .map_err(|e| format!("could not read max {} bytes from file {}: {}", buf.len(), name, e))
    }

    fn close_current_file(&mut self, ) -> Result<(), String> {
        if self.file.is_none() {
            return Err("no current file opened to close".to_owned());
        }
        self.file = None;
        Ok(())
    }

    fn read_single_file(full_path: &str) -> Result<Vec<u8>, String> {
        let mut contents = Vec::new();
        File::open(full_path)
            .map_err(|e| format!("could not open single file {} for reading: {}", full_path, e))?
            .read_to_end(&mut contents)
            .map_err(|e| format!("could not read single file {}: {}", full_path, e))?;
        Ok(contents)
    }
}

#[cfg(test)]
mod tests {
    use std::env::temp_dir;
    use std::io::Write;
    use std::path::MAIN_SEPARATOR_STR;
    use std::fs::File;

    use crate::joiner::MultiFilesReaderSource;

    use super::MultiFilesReader;

    fn full_file_name(s: &str) -> String {
        format!("{}{}{}", temp_dir().display(), MAIN_SEPARATOR_STR, s)
    }

    #[test]
    fn read_from_two_files() {
        for (fname, data) in vec![
            ("f1", &vec![1,2,3]),
            ("f2", &vec![4,5])
        ]
        {
            let mut f = File::create(full_file_name(fname)).unwrap();
            f.write_all(data).unwrap();
        }

        let mut mfr = MultiFilesReader{ file: None };
        let mut all_data = Vec::new();

        for fname in vec!["f1", "f2"] {
            mfr.open_next_file(full_file_name(fname).as_str()).unwrap();
            let mut buf = [0u8; 8];
            let b_read = mfr.read_from_current_file(buf.as_mut_slice()).unwrap();
            all_data.extend_from_slice(&buf[..b_read]);
            mfr.close_current_file().unwrap();
        }

        assert_eq!(&[1,2,3,4,5], all_data.as_slice());
    }
}