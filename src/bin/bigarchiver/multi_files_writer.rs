use crate::splitter::MultiFilesWriterTarget;
use std::fs::File;
use std::io::prelude::*;

pub struct MultiFilesWriter {
    current_file: Option<(File, String)>
}

impl MultiFilesWriter {
    pub fn new() -> Self {
        Self { current_file: None }
    }
}

impl MultiFilesWriterTarget for MultiFilesWriter {
    fn open_next_file(&mut self, full_path: &str) -> Result<(), String> {
        if let Some((_, name)) = &self.current_file {
            return Err(format!("previous file {} was not closed before opening a new file {}", name, full_path));
        }
        self.current_file = Some((
            File::create(full_path).map_err(|e| format!("could not create file {}: {}", full_path, e))?, 
            full_path.to_owned()
        ));
        eprintln!("writing to {}", full_path);
        Ok(())
    }

    fn close_current_file(&mut self) -> Result<(), String> {
        if self.current_file.is_none() {
            return Err("no current file opened to close".to_owned());
        }
        self.current_file = None;
        Ok(())
    }

    fn write_to_current_file(&mut self, data: &[u8]) -> Result<(), String> {
        let file_and_name = self.current_file
            .as_mut()
            .ok_or("no current file opened to write".to_owned())?;
        file_and_name
            .0
            .write_all(data)
            .map_err(|e| format!("could not write {} bytes to file {}: {}", data.len(), file_and_name.1, e))
            .map(|_|())
    }

    fn write_single_file(&self, path: &str, contents: &str) -> Result<(), String> {
        File::create(path)
            .map_err(|e| format!("could not create single file {}: {}", path, e))?
            .write_all(contents.as_bytes())
            .map_err(|e| format!("could not write to single file {}: {}", path, e))
    }

}

#[cfg(test)]
mod tests {
    use crate::{multi_files_writer::MultiFilesWriter, splitter::MultiFilesWriterTarget};
    use std::fs;
    use std::fs::File;
    use std::io::Read;

    fn clear_file(fname: &str) {
        let _ = fs::remove_file(fname);
    }

    fn check_and_clear_file(fname: &str, expected_data: &[u8]) {
        let mut buf = Vec::new();
        File::open(fname).unwrap().read_to_end(&mut buf).unwrap();
        assert_eq!(expected_data, buf);
        clear_file(fname);
    }

    fn write_to_file(f: &mut MultiFilesWriter, fname: &str, data: &[u8]) {
        f.open_next_file(fname).unwrap();
        f.write_to_current_file(data).unwrap();
        f.close_current_file().unwrap();
    }

    #[test]
    fn open_write_close_all_ok() {
        const FN1: &str = "/tmp/file1";
        const FN2: &str = "/tmp/file2";
        const FN3: &str = "/tmp/file3";

        clear_file(FN1);
        clear_file(FN2);
        clear_file(FN3);

        let mut f = MultiFilesWriter::new();

        write_to_file(&mut f, FN1, &[1,2,3]);
        write_to_file(&mut f, FN2, &[4,5,6]);
        f.write_single_file(FN3, "single").unwrap();

        check_and_clear_file(FN1, &[1,2,3]);
        check_and_clear_file(FN2, &[4,5,6]);
        check_and_clear_file(FN3, b"single");
    }

    #[test]
    fn double_open_without_close() {
        const FN1: &str = "/tmp/file11";
        const FN2: &str = "/tmp/file12";
        clear_file(FN1);
        clear_file(FN2);
        let mut f = MultiFilesWriter::new();
        f.open_next_file(FN1).unwrap();
        f.open_next_file(FN2).unwrap_err();
        clear_file(FN1);
        clear_file(FN2);
    }


        #[test]
    fn double_close_without_open() {
        const FN1: &str = "/tmp/file111";
        clear_file(FN1);
        let mut f = MultiFilesWriter::new();
        f.open_next_file(FN1).unwrap();
        f.close_current_file().unwrap();
        f.close_current_file().unwrap_err();
        clear_file(FN1);
    }
}
