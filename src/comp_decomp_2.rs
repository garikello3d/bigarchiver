use std::io::Write;
use liblzma::write::{XzEncoder, XzDecoder};
use crate::finalizable::DataSink;

pub struct Conv<'a, T: DataSink> {
    t: &'a mut T
}

impl<'a, T: DataSink> Conv<'a, T> {
    #[allow(dead_code)]
    pub fn get_sink(&'a self) -> &'a T {
        self.t
    }
}

impl<'a, T: DataSink> Write for Conv<'a, T> {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        self.t.add(data).map(|_|data.len()).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub struct Compressor2<'a, T: DataSink> {
    enc: XzEncoder<Conv<'a, T>>
}


impl<'a, T: DataSink> Compressor2<'a, T> {
    pub fn new(to: &'a mut T, level: u32) -> Compressor2<'a, T> {
        Compressor2 { enc: XzEncoder::new(Conv{ t: to }, level) }
    }

    #[allow(dead_code)]
    pub fn uncompressed(&self) -> usize {
        self.enc.total_in() as usize
    }

    #[allow(dead_code)]
    pub fn compressed(&self) -> usize {
        self.enc.total_out() as usize
    }
}

impl<'a, T: DataSink> DataSink for Compressor2<'a, T> {
    fn add(&mut self, data: &[u8]) -> Result<(), String> {
        //eprintln!("Compressor: writing {} bytes", data.len());
        self.enc.write_all(data).map_err(|e| format!("write all error: {}", e))
    }
    fn finish(&mut self) -> Result<(), String> {
        self.enc.flush().map_err(|e| format!("compressor flush error: {}", e))?;
        self.enc.try_finish().map_err(|e| format!("compressor finalization error: {}", e))?;
        self.enc.get_mut().t.finish()
    }
}


pub struct Decompressor2<'a, T: DataSink> {
    dec: XzDecoder<Conv<'a, T>>
}

impl<'a, T: DataSink> Decompressor2<'a, T> {
    pub fn new(to: &'a mut T) -> Decompressor2<'a, T> {
        Decompressor2 { dec: XzDecoder::new(Conv{ t: to }) }
    }

    #[allow(dead_code)]
    pub fn get_decoder(&'a self) -> &XzDecoder<Conv<'a, T>> {
        &self.dec
    }
}

impl<'a, T: DataSink> DataSink for Decompressor2<'a, T> {
    fn add(&mut self, data: &[u8]) -> Result<(), String> {
        self.dec.write_all(data).map_err(|e| format!("write all error: {}", e))
    }
    fn finish(&mut self) -> Result<(), String> {
        self.dec.flush().map_err(|e| format!("decompressor flush error: {}", e))?;
        self.dec.get_mut().t.finish()
    }
}

#[cfg(test)]
mod tests {
    use crate::finalizable::DataSink;
    use super::{Compressor2, Decompressor2};
    use rand::{thread_rng, Rng, RngCore};
    use std::{thread, sync::{atomic::{AtomicBool, Ordering}, Arc}};

    struct Sink {
        data: Vec<u8>
    }

    impl DataSink for Sink {
        fn add(&mut self, data: &[u8]) -> Result<(), String> {
            self.data.extend_from_slice(data);
            Ok(())
        }

        fn finish(&mut self) -> Result<(), String> {
            Ok(())
        }
    }

    struct NullSink;

    impl DataSink for NullSink {
        fn add(&mut self, _: &[u8]) -> Result<(), String> { Ok(()) }
        fn finish(&mut self) -> Result<(), String> { Ok(()) }
    }

    #[test]
    fn zip_unzip_small_2() {
        let mut sink_for_zipped = Sink{ data: Vec::new() };
        let mut comp = Compressor2::new(&mut sink_for_zipped, 8);
        comp.add(b"HELLO").unwrap();
        comp.finish().unwrap();
        let data = &comp.enc.get_ref().t.data;
        eprintln!("{} bytes: {:?}", data.len(), data);

        let mut sink_for_unzipped = Sink{ data: Vec::new() };
        let mut decomp = Decompressor2::new(&mut sink_for_unzipped);
        decomp.add(&data.clone()).unwrap();
        decomp.finish().unwrap();
        let orig_data = &decomp.dec.get_ref().t.data;
        eprintln!("{} bytes: {:?}", orig_data.len(), orig_data);

        assert_eq!(orig_data, b"HELLO");
    }

    fn add_by_random_parts<T: DataSink>(t: &mut T, data: &[u8], max_part: usize) {
        let mut left = data.len();
        let mut offs = 0;
        while left != 0 {
            let to_add_max = thread_rng().gen::<usize>() % (max_part - 1) + 1;
            let to_add = usize::min(left, to_add_max);
            t.add(&data[offs..offs+to_add]).unwrap();
            left -= to_add;
            offs += to_add;
        }
    }

    #[test]
    fn zip_unzip_big_2() {
        let mut src: Vec<u8> = Vec::new();
        src.resize(2 * 1024 * 1024, 0);
        thread_rng().fill_bytes(&mut src);

        let mut sink_for_zipped = Sink{ data: Vec::new() };
        let mut comp = Compressor2::new(&mut sink_for_zipped, 9);
        //comp.add(&src).unwrap();
        add_by_random_parts(&mut comp, &src, 512);
        //eprintln!("could write {} bytes to compressor", written);
        comp.finish().unwrap();
        let data = &comp.enc.get_ref().t.data;
        eprintln!("{} bytes -> {} bytes", src.len(), data.len());

        let mut sink_for_unzipped = Sink{ data: Vec::new() };
        let mut decomp = Decompressor2::new(&mut sink_for_unzipped);
        //decomp.add(&data.clone()).unwrap();
        add_by_random_parts(&mut decomp, &data.clone(), 512);
        //eprintln!("could write {} bytes to decompressor", written);

        decomp.finish().unwrap();
        let orig_data = &decomp.dec.get_ref().t.data;
        eprintln!("{} bytes -> {} bytes", data.len(), orig_data.len());

        assert_eq!(orig_data, &src);
    }

    #[test]
    #[ignore]
    fn compress_1_min() {
        const SEND_SIZE: usize = 50 * 1024 * 1024;
        let is_stop = Arc::new(AtomicBool::new(false));
        let is_stop_copy = is_stop.clone();
        let t = std::thread::spawn(move ||{            
            let mut buf = Vec::with_capacity(SEND_SIZE);
            buf.resize(SEND_SIZE, 0);
            let mut null_sink = NullSink{};
            let mut count = 0;
            let mut comp = Compressor2::new(&mut null_sink, 6);
            while !is_stop_copy.load(Ordering::SeqCst) {
                thread_rng().fill_bytes(&mut buf);
                comp.add(&buf).unwrap();
                count += buf.len();
            }
            comp.finish().unwrap();
            (count, comp.compressed())
        });
        thread::sleep(std::time::Duration::from_secs(600));
        is_stop.store(true, Ordering::SeqCst);
        let stat = t.join().unwrap();
        println!("compressed {} bytes into {} bytes", stat.0, stat.1);

        // memory consumption results obtained so far depending on the compression level (50MB portion size):
        // 1 - 60m
        // 2 - 70M
        // 3 - 85M
        // 4 - 100M
        // 5 - 150M
        // 6 - 150M
        // 7 - 245M
        // 8 - 430M
        // 9 - 745M
    }
}
