pub trait DataSink {
    fn add(&mut self, data: &[u8]) -> Result<(), String>;
    fn finish(&mut self) -> Result<(), String>;
}
