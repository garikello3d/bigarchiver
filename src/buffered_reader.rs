use std::io::Read;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use crate::finalizable::DataSink;

pub struct BufferedReader<'a, R: Read, T: DataSink> {
    read_from: &'a mut R,
    write_to: &'a mut T,
    read_buf_size: usize,
    store_buf_size: usize,
    exit_flag: Option<Arc<AtomicBool>>,
}

impl<'a, R: Read, T: DataSink> BufferedReader<'a, R, T> {
    pub fn new(read_from: &'a mut R, write_to: &'a mut T, read_buf_size: usize, store_buf_size: usize, exit_flag: Option<Arc<AtomicBool>>) -> Self {
        assert!(read_buf_size < store_buf_size);
        Self { read_from, write_to, read_buf_size, store_buf_size, exit_flag: exit_flag }
    }

    pub fn read_and_write_all(&mut self) -> Result<(), String> {
        let mut buf: Vec<u8> = Vec::with_capacity(self.store_buf_size);
        buf.resize(self.store_buf_size, 0);

        let mut eof = false;

        while !eof {
            let mut offs = 0;
            let mut left = self.store_buf_size;
    
            while left > self.read_buf_size {
                if let Some(ex_flag) = &self.exit_flag {
                    if ex_flag.load(Ordering::SeqCst) {
                        eof = true;
                        break;
                    }
                }

                if let Ok(bytes_read) = self.read_from.read(&mut buf[offs..offs+self.read_buf_size]) {
                    if bytes_read > 0 {
                        //eprintln!("BufferedReader: read and buffered {} bytes from source", bytes_read);
                        offs += bytes_read;
                        left -= bytes_read;
                    } else {
                        //eprintln!("BufferedReader: eof");
                        eof = true;
                        break;
                    }
                }
            }

            self.write_to.add(&buf[..offs])?;
        }

        self.write_to.finish()?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io::Read;
    use std::sync::{Arc, atomic::AtomicBool};
    use std::thread;
    use rand::{thread_rng, Rng, RngCore};
    use crate::finalizable::DataSink;
    use crate::buffered_reader::BufferedReader;

    struct DummyReader {
        all_data: Vec<u8>,
        offset: usize,
        read_delay_ms: u32
    }
    impl DummyReader {
        fn new(data_size: usize, read_delay_ms: u32) -> Self {
            let mut data = Vec::with_capacity(data_size);
            data.resize(data_size, 0);
            thread_rng().fill_bytes(&mut data);
            Self { all_data: data, offset: 0, read_delay_ms }
        }
    }
    impl Read for DummyReader {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            if self.offset == self.all_data.len() {
                return Ok(0);
            }
            let to_return_max = usize::min(buf.len(), self.all_data.len() - self.offset);
            let to_return = thread_rng().gen::<usize>() % to_return_max + 1;
            buf[..to_return].copy_from_slice(&self.all_data[self.offset..self.offset+to_return]);
            self.offset += to_return;
            if self.read_delay_ms > 0 {
                thread::sleep(std::time::Duration::from_millis(self.read_delay_ms as u64));
            }
            Ok(to_return)
        }
    }

    struct TestSink {
        data: Vec<u8>,
    }
    impl DataSink for TestSink {
        fn add(&mut self, data: &[u8]) -> Result<(), String> {
            //eprintln!("sink: received {} bytes", data.len());
            self.data.extend_from_slice(data);
            Ok(())
        }
        fn finish(&mut self) -> Result<(), String> {
            Ok(())
        }
    }

    #[test]
    fn dummy_reader() {
        let mut dr = DummyReader::new(1000, 0);
        let mut received_data = Vec::new();
        let mut buf = [0u8; 100];
        loop {
            let bytes_rcv = dr.read(&mut buf).unwrap();
            if bytes_rcv == 0 {
                break;
            }
            received_data.extend_from_slice(&buf[..bytes_rcv]);
        }
        assert_eq!(received_data, dr.all_data);
    }

    #[test]
    fn buffered_reader() {
        let mut dr = DummyReader::new(1000, 0);
        {
            let mut sink = TestSink{ data: Vec::new() };
            {
                let mut buf_reader = BufferedReader::new(
                    &mut dr, &mut sink, 10, 100, None);
                buf_reader.read_and_write_all().unwrap();
            }
            //assert_eq!(buf_reader.read_from.all_data, buf_reader.write_to.data);
            assert_eq!(dr.all_data, sink.data);
        }
    }

    #[test]
    fn buffered_reader_stoppable() {
        let exit_flag = Arc::new(AtomicBool::new(false));
        let exit_flag_clone = exit_flag.clone();

        let thread = thread::spawn(move|| {
            let mut dr = DummyReader::new(1000, 100);
            {
                let mut sink = TestSink{ data: Vec::new() };
                {
                    let mut buf_reader = BufferedReader::new(
                        &mut dr, &mut sink, 10, 100, Some(exit_flag_clone));
                    buf_reader.read_and_write_all().unwrap();
                }
                let nr_written = sink.data.len();
                assert!(nr_written > 0);
                assert!(nr_written < 30);
                assert_eq!(dr.all_data[..nr_written], sink.data);
            }

        });

        thread::sleep(std::time::Duration::from_millis(200));
        exit_flag.store(true, std::sync::atomic::Ordering::SeqCst);
        thread.join().unwrap();
    }
}
