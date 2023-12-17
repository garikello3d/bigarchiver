pub trait Finalizable {
    fn finalize(&mut self) -> Result<(), ()>;
}

pub trait DataSink {
    fn add(&mut self, data: &[u8]) -> Result<(), String>;
    fn finish(&mut self) -> Result<(), String>;
}
