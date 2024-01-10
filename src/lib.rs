mod hasher;
use hasher::DataHasher;

pub mod finalizable;
use finalizable::DataSink;

mod enc_dec;
use enc_dec::{Encryptor, Decryptor};

mod comp_decomp_2;
use comp_decomp_2::{Compressor2, Decompressor2};

mod fixed_size_writer;
use fixed_size_writer::FixedSizeWriter;

mod joiner;
use joiner::Joiner;

mod multi_files_reader;
use multi_files_reader::MultiFilesReader;

mod buffered_reader;
use buffered_reader::BufferedReader;

mod stats;
use stats::Stats;

mod multi_files_writer;
use multi_files_writer::MultiFilesWriter;

mod splitter;
use splitter::Splitter;

pub mod arg_opts;
pub mod file_set;

mod free_space;
use free_space::get_free_space;

use std::time::{SystemTime, UNIX_EPOCH};
use std::io::Read;
use time::OffsetDateTime;
use std::sync::{Arc, atomic::AtomicBool};
use std::fs::File;

pub fn timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap() // SAFE: rely on fact that now() cannot return anything earlier than EPOCH
        .as_secs()
}

fn time_str() -> String {
    let now = OffsetDateTime::now_local().unwrap_or(OffsetDateTime::now_utc());
    let dt = now.date();
    let tm = now.time();
    format!("{}-{:02}-{:02} {:02}:{:02}:{:02} Z{:02}", dt.year(), dt.month() as u8, dt.day(), tm.hour(), tm.minute(), tm.second(), now.offset().whole_hours())
}

pub fn backup<R: Read>(
    mut read_from: R,
    auth: &str, auth_every_bytes: usize, split_size_bytes: usize, out_template: &str, 
    pass: &str, compress_level: u8, nr_threads: usize, buf_size_bytes: usize, exit_flag: Option<Arc<AtomicBool>>) -> Result<usize, String>
{
    let hash_seed = timestamp();
    let start_time_str = time_str();

    let mut stats = Stats::new();
    stats.auth_string = String::from(auth);
    stats.auth_chunk_size = auth_every_bytes;
    stats.out_chunk_size = split_size_bytes;
    stats.hash_seed = hash_seed;

    let mut fmgr = MultiFilesWriter::new();
    let mut spl: Splitter<'_, MultiFilesWriter> = Splitter::from_pattern(&mut fmgr, split_size_bytes, out_template)?;
    {
        let enc = Encryptor::new(&mut spl, pass, auth);
        let mut fbuf = FixedSizeWriter::new(enc, auth_every_bytes);
        let mut comp = Compressor2::new(&mut fbuf, compress_level as u32, nr_threads as u32)?;
        {
            let mut hash_copier = DataHasher::with_writer(Some(&mut comp), hash_seed);

            let mut stdinbuf = BufferedReader::new(
                &mut read_from, &mut hash_copier, buf_size_bytes / 8, buf_size_bytes, exit_flag);

            stdinbuf.read_and_write_all()?;

            stats.in_data_len = hash_copier.counter();
            stats.in_data_hash = hash_copier.result();
        }
        stats.compressed_len = comp.compressed();
    }

    let end_timestamp = timestamp();
    let end_time_str = time_str();
    let throughput_mbps = if end_timestamp - hash_seed != 0 { stats.in_data_len as u64 / 1024 / 1024 / (end_timestamp - hash_seed) } else { 0 };
    stats.misc_info = Some(format!("version={}, started={}, ended={}, took={}s, througput={}MB/s", 
        option_env!("VERSION").unwrap_or("?"),
        start_time_str, end_time_str, end_timestamp - hash_seed, throughput_mbps));

    spl.write_metadata(&stats)?;
    Ok(stats.in_data_len)
}

    
pub fn check<W: DataSink>(mut write_to: Option<W>, cfg_path: &str, pass: &str, nr_threads: usize, buf_size_bytes: usize, check_free_space: &Option<&str>, show_info: bool) -> Result<(), String> {
    let stats = Stats::from_readable(File::open(cfg_path)
        .map_err(|e| format!("could not open metadata file '{}': {}", cfg_path, e))?)?;

    if show_info {
        eprintln!("authentication string: {}", stats.auth_string);
        eprintln!("misc info: {}", stats.misc_info.as_ref().unwrap_or(&"none".to_owned()));
    }

    if let Some(mount_point) = check_free_space {
        if get_free_space(mount_point)? < stats.in_data_len {
            return Err(format!("filesystem of '{}' won't fit {} bytes of data to restore", mount_point, stats.in_data_len));
        }
    }

    let ref_write_to = write_to.as_mut();

    let mut hash_copier = DataHasher::with_writer(ref_write_to, stats.hash_seed);
    {
        let mut decomp = Decompressor2::new(&mut hash_copier, nr_threads as u32)?;
        let dec = Decryptor::new(&mut decomp, pass, &stats.auth_string);
        let mut fbuf = FixedSizeWriter::new(dec, stats.auth_chunk_size + 16);
        let fmgr = MultiFilesReader::new();

        let mut joiner = Joiner::from_metadata(
            fmgr, &mut fbuf, cfg_path, buf_size_bytes)?;

        joiner.read_and_write_all()?;
    }

    if hash_copier.result() != stats.in_data_hash {
        Err("hash verification error".to_owned())
    } else {
        Ok(())
    }
}
