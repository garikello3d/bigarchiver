//use std::io::Write;
use crate::finalizable::DataSink;

pub struct FixedSizeWriter<T: DataSink> {
    out: T,
    size: usize,
    buf: Vec<u8>
}

impl<T: DataSink> FixedSizeWriter<T> {
    pub fn new(out: T, size: usize) -> FixedSizeWriter<T> {
        FixedSizeWriter { out: out, size: size, buf: Vec::new() }
    }
}

impl<T: DataSink> DataSink for FixedSizeWriter<T> {
    fn add(&mut self, data: &[u8]) -> Result<(), String> {
        //eprintln!("FixedSizeWriter: writing {} bytes", data.len());
        let mut data_offs = 0;
        if !self.buf.is_empty() { // process internal buffer first, if we can write something
            let left_to_fill_buf = self.size - self.buf.len();
            if data.len() >= left_to_fill_buf {
                self.buf.extend_from_slice(&data[..left_to_fill_buf]);
                self.out.add(self.buf.as_slice())?;
                self.buf.clear();
                data_offs += left_to_fill_buf;
            } else {
                self.buf.extend_from_slice(data);
                return Ok(());
            }
        }
        assert!(self.buf.is_empty());
        let mut left_for_data = data.len() - data_offs;
        while left_for_data >= self.size {
            self.out.add(&data[data_offs..data_offs+self.size])?;
            left_for_data -= self.size;
            data_offs += self.size;
        }
        if left_for_data > 0 {
            self.buf = data[data_offs..].to_vec();
        }
        Ok(())
    }

    fn finish(&mut self) -> Result<(), String> {
        //eprintln!("FixedSizeWriter: finish");
        if !self.buf.is_empty() {
            self.out.add(self.buf.as_slice())?;
            self.buf.clear();
        }
        self.out.finish()
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    impl<T: DataSink> FixedSizeWriter<T> {
        fn internal_buf(&self) -> &[u8] {
            &self.buf
        }
    }
    
    struct TestOut {
        actual_writes: Vec<Vec<u8>>,
        expected_writes: Vec<Vec<u8>>,
    }

    impl DataSink for TestOut {
        fn add(&mut self, data: &[u8]) -> Result<(), String> {
            self.actual_writes.push(data.to_vec());
            Ok(())
        }
        fn finish(&mut self) -> Result<(), String> {
            assert_eq!(self.expected_writes, self.actual_writes);
            Ok(())
        }
    }

    fn conv(slices: &[&[u8]]) -> Vec<Vec<u8>> {
        let mut ret = Vec::new();
        for slice in slices {
            ret.push(slice.to_vec());
        }
        ret
    }

    fn write_xx(buf_size: usize, in_writes: &[&[u8]], out_writes: &[&[u8]]) {
        let out = TestOut{ actual_writes: Vec::new(), expected_writes: conv(out_writes) };
        let mut fsw = FixedSizeWriter::<TestOut>::new(out, buf_size);
        for iw in in_writes {
            fsw.add(&iw).unwrap();
        }
        fsw.finish().unwrap();
        assert!(fsw.internal_buf().is_empty());
    }

    #[test]
    fn write_22() {
        write_xx(3, &[&[1,2], &[3,4]], &[&[1,2,3], &[4]]); // 22
        write_xx(3, &[&[1,2,3], &[4,5]], &[&[1,2,3], &[4,5]]); // 32
        write_xx(3, &[&[1,2,3,4,5], &[6,7]], &[&[1,2,3], &[4,5,6], &[7]]); // 52

        write_xx(3, &[&[1,2], &[3,4,5]], &[&[1,2,3], &[4,5]]); // 23
        write_xx(3, &[&[1,2,3], &[4,5,6]], &[&[1,2,3], &[4,5,6]]); // 33
        write_xx(3, &[&[1,2,3,4,5], &[6,7,8]], &[&[1,2,3], &[4,5,6], &[7,8]]); // 53

        write_xx(3, &[&[1,2], &[3,4,5,6,7]], &[&[1,2,3], &[4,5,6], &[7]]); // 25
        write_xx(3, &[&[1,2,3], &[4,5,6,7,8]], &[&[1,2,3], &[4,5,6], &[7,8]]); // 35
        write_xx(3, &[&[1,2,3,4,5], &[6,7,8,9,0]], &[&[1,2,3], &[4,5,6], &[7,8,9], &[0]]); // 55
    }
}
