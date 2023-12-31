#[cfg(test)]

use bigarchiver::{backup,check};
use bigarchiver::finalizable::DataSink;

mod common;

use rand::RngCore;
use test_case::test_matrix;
use std::io::Write;
use std::sync::atomic::AtomicI32;
use std::fs::File;

static CNT: AtomicI32 = AtomicI32::new(0);

struct SinkToVector<'a> {
    incoming: Vec<u8>,
    etalon: &'a [u8]
}

impl DataSink for SinkToVector<'_> {
    fn add(&mut self, data: &[u8]) -> Result<(), String> {
        self.incoming.extend_from_slice(data);
        Ok(())
    }

    fn finish(&mut self) -> Result<(), String> {
        assert_eq!(&self.incoming, self.etalon);
        Ok(())
    }
}

#[test_matrix(
    [10, 100, 1000], // input_size
    [10, 100, 1000], // auth_size
    [10, 100, 1000], // split_size
    [10, 100, 1000]  // buf_size
)]
fn backup_restore_all_ok(input_size: usize, auth_size: usize, split_size: usize, buf_size: usize) {
    let cnt = CNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let parent_dir = format!("/tmp/all_ok_{}", cnt);
    let _ = std::fs::remove_dir_all(&parent_dir);
    let _ = std::fs::create_dir(&parent_dir);
    let out_tpl = format!("{}/%%%%%%", &parent_dir);
    let out_cfg = format!("{}/000000.cfg", &parent_dir);

    let mut src: Vec<u8> = Vec::with_capacity(input_size);
    src.resize(input_size, 0);
    rand::thread_rng().fill_bytes(&mut src);

    backup(
        &src[..],
        "The Author",
        auth_size,
        split_size,
        &out_tpl,
        "secret",
        9,
        buf_size, None).unwrap();

    let src_unpacked = SinkToVector{ incoming: Vec::new(), etalon: &src };

    check(
        Some(src_unpacked),
        &out_cfg,
        "secret",
        buf_size, &None::<&str>, true).unwrap();

}

#[test]
fn restore_no_free_space() {
    let cfg_path = "/tmp/no_free_space0.cfg";
    let cfg_contents = format!("\
        in_len={}\n\
        in_hash=abcde\n\
        hash_seed=edcba\n\
        xz_len=54321\n\
        nr_chunks=1\n\
        chunk_len=2\n\
        auth=Author Name\n\
        auth_len=3", usize::MAX);
    File::create(cfg_path).unwrap().write_all(cfg_contents.as_bytes()).unwrap();
    let err = check(Some(SinkToVector{ incoming: Vec::new(), etalon: b"" }), cfg_path, "", 100, &Some("/tmp"), true).unwrap_err();
    println!("err = {}", err);
}
