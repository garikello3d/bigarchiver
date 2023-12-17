use std::io::{stdin, stdout};
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

use bigarchiver::finalizable::DataSink;
use bigarchiver::enc_dec::{Encryptor, Decryptor};
use bigarchiver::comp_decomp_2::{Compressor2, Decompressor2};
use bigarchiver::hasher::DataHasher;
use bigarchiver::fixed_size_writer::FixedSizeWriter;
use bigarchiver::multi_files_reader::MultiFilesReader;
use bigarchiver::joiner::{Joiner,read_metadata};
use bigarchiver::buffered_reader::BufferedReader;
use bigarchiver::stats::Stats;
use bigarchiver::arg_opts::{ArgOpts, ArgModeSpecificOpts};
use bigarchiver::patterns::cfg_from_pattern;

mod splitter;
use splitter::Splitter;

mod multi_files_writer;
use multi_files_writer::MultiFilesWriter;

fn backup(
    auth: &str, auth_every_bytes: usize, split_size_bytes: usize, out_template: &str, 
    pass: &str, compress_level: u8, buf_size_bytes: usize) -> Result<(), String>
{
    let hash_seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap() // SAFE: rely on fact that now() cannot return anything earlier than EPOCH
        .as_secs();

    let mut stats = Stats::new();
    stats.auth_string = String::from(auth);
    stats.auth_chunk_size = auth_every_bytes;
    stats.out_chunk_size = Some(split_size_bytes);
    stats.hash_seed = Some(hash_seed);

    let mut fmgr = MultiFilesWriter::new();
    let mut spl: Splitter<'_, MultiFilesWriter> = Splitter::new(&mut fmgr, split_size_bytes, out_template)?;
    {
        let enc = Encryptor::new(&mut spl, pass, auth);
        let mut fbuf = FixedSizeWriter::new(enc, auth_every_bytes);
        let mut comp = Compressor2::new(&mut fbuf, compress_level as u32);
        {
            let mut hash_copier = DataHasher::with_writer(&mut comp, hash_seed);

            let sin = &mut stdin();
            let mut stdinbuf = BufferedReader::new(
                sin, &mut hash_copier, buf_size_bytes / 8, buf_size_bytes);

            stdinbuf.read_and_write_all()?;

            stats.in_data_len = Some(hash_copier.counter());
            stats.in_data_hash = Some(hash_copier.result());
        }
        stats.compressed_len = Some(comp.compressed());
    }

    spl.write_metadata(&stats)
}

    
fn check(restore: bool, cfg_path: &str, pass: &str, buf_size_bytes: usize, check_free_space: bool) -> Result<(), String> {
    struct StdoutWriter;

    impl DataSink for StdoutWriter {
        fn add(&mut self, data: &[u8]) -> Result<(), String> {
            //eprintln!("writing {} bytes to stdout", data.len());
            stdout().write_all(data).map_err(|e| format!("could not write {} bytes to stdout: {}", data.len(), e))
        }

        fn finish(&mut self) -> Result<(), String> {
            stdout().flush().map_err(|e| format!("could not flush to stdout: {}", e))
        }
    }

    let stats = read_metadata::<MultiFilesReader>(cfg_path)?;

    let mut out = StdoutWriter{};

    let mut hash_copier = 
        if restore { DataHasher::with_writer(&mut out, stats.hash_seed.unwrap() ) } 
        else { DataHasher::with_null(stats.hash_seed.unwrap()) }; // SAFE: read_metadata checked that all is set
    
    {
        let mut decomp = Decompressor2::new(&mut hash_copier);
        let dec = Decryptor::new(&mut decomp, pass, &stats.auth_string);
        let mut fbuf = FixedSizeWriter::new(dec, stats.auth_chunk_size + 16);
        let fmgr = MultiFilesReader::new();

        let mut joiner = Joiner::from_metadata(
            fmgr, &mut fbuf, cfg_path, buf_size_bytes)?;

        joiner.read_and_write_all()?;
    }

    if hash_copier.result() != stats.in_data_hash.unwrap() { // SAFE: read_metadata checked that all is set
        Err("hash verification error".to_owned())
    } else {
        Ok(())
    }
}

fn process_args(args: &ArgOpts) -> Result<(), String> {
    match &args.mode_specific_opts {
        ArgModeSpecificOpts::Backup { 
            out_template, no_check, auth, auth_every, split_size, compress_level
        } => {
            eprintln!("backing up...");
            backup(
                &auth, *auth_every, 
                *split_size, &out_template, 
                &args.pass, *compress_level, args.buf_size)?;
            if !no_check {
                let cfg_path = cfg_from_pattern(&out_template)?;
                eprintln!("verifying...");
                check(false, &cfg_path, &args.pass, args.buf_size, false)
            } else {
                Ok(())
            }
        },
        ArgModeSpecificOpts::Restore { config_path, no_check, no_check_free_space } => {
            if !no_check {
                eprintln!("verifying before restore...");
                check(false, &config_path, &args.pass, args.buf_size, false)
                    .map_err(|e| format!("will not restore data, integrity check error: {}", e))?;
            }
            eprintln!("restoring...");
            check(true, &config_path, &args.pass, args.buf_size, !no_check_free_space)
                .map_err(|e| format!("error restoring data: {}", e))
        },
        ArgModeSpecificOpts::Check { config_path } => {
            eprintln!("verifying...");
            check(false, &config_path, &args.pass, 
                args.buf_size, false)
        }
    }
}

fn main() {
    let args = {
        let args = ArgOpts::from_os_args(&std::env::args_os().skip(1).collect());
        if let Err(e) = &args {
            eprintln!("
error parsing command line: {}\n\n\
example to pack from stdout:\n
tar cf - /my/files | bigarchiver --backup --out-template /path/to/dir/file%%%%xxx --pass Secret --buf-size 256 --auth AuthData --auth-every 32 --split-size 10 --compress-level 6 [--no-check]\n
example to unpack into stdout:\n
./bigarchiver --restore --config /path/to/dir/file%%%%xxx.cfg --pass Secret --buf-size 256 [--no-check] [--no-check-free-space] | tar xf -\n
example to check existing backup without restoring:\n
./bigarchiver --check --config /path/to/dir/file%%%%xxx.cfg --pass Secret --buf-size 256
" ,e);
            std::process::exit(1);
        };
        args.unwrap()
    };

    if let Err(e) = process_args(&args) {
        eprintln!("\nerror: {}\n", e);
        // TODO set proper exit code
    } else {
        eprintln!("\ndone\n");
    }
    
}
