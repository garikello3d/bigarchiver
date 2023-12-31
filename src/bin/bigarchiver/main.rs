use bigarchiver::arg_opts::{ArgOpts, Commands};
use bigarchiver::{backup, check, timestamp};
use bigarchiver::file_set::cfg_from_pattern;
use bigarchiver::finalizable::DataSink;
use clap::Parser;
use std::io::{stdout, Write};
use std::process::ExitCode;
use std::{thread, fs};
use std::sync::{Arc, atomic::AtomicBool};

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

fn process_args(args: &ArgOpts) -> Result<(), String> {
    match &args.command {
        Commands::Backup { 
            out_template, pass, auth, auth_every, split_size, compress_level, buf_size, no_check
        } => {
            eprintln!("backing up...");
            let buf_size = *buf_size * 1_048_576;
            let split_size = *split_size * 1_048_576;
            let auth_every = *auth_every * 1_048_576;
            backup(&mut std::io::stdin(),
                &auth, auth_every, 
                split_size, &out_template, 
                pass, *compress_level, buf_size, None)?;
            if !no_check {
                let cfg_path = cfg_from_pattern(&out_template);
                eprintln!("verifying...");
                check(None::<StdoutWriter>, &cfg_path, pass, buf_size, &None::<&str>, true)
            } else {
                Ok(())
            }
        },

        Commands::Restore { config, pass, buf_size, check_free_space, no_check } => {
            let buf_size = *buf_size * 1_048_576;
            if !no_check {
                eprintln!("verifying before restore...");
                check(None::<StdoutWriter>, &config, pass, buf_size, &None, true)
                    .map_err(|e| format!("will not restore data, integrity check error: {}", e))?;
            }
            eprintln!("restoring...");
            let may_be_check = check_free_space.as_ref().map(|s| s.as_str());
            check(Some(StdoutWriter{}), &config, pass, 
                buf_size, &may_be_check, true)
                    .map_err(|e| format!("error restoring data: {}", e))
        },

        Commands::Check { config, pass, buf_size } => {
            eprintln!("verifying...");
            let buf_size = *buf_size * 1_048_576;
            check(None::<StdoutWriter>, &config, pass, 
                buf_size, &None, true)
        },

        Commands::Bench { out_dir, duration, compress_levels, buf_sizes } => {
            struct Throughput {
                level: u8,
                buf_size: usize,
                time_spent_s: u64,
                bytes: usize,
                bps: usize
            }

            let mut thrpts: Vec<Throughput> = Vec::new();

            for compress_level in compress_levels {
                for buf_size in buf_sizes {
                    let exit_flag = Arc::new(AtomicBool::new(false));
                    let exit_flag_clone = exit_flag.clone();
                    let level = *compress_level;
                    let buf_size_bytes = *buf_size * 1_048_576;

                    let base_dir = format!("{}/{}-{}", out_dir, compress_level, buf_size);
                    let _ = fs::remove_dir_all(&base_dir); // we don't care if it does not exist
                    fs::create_dir(&base_dir).map_err(|e| format!("could not create directory {}: {}", &base_dir, e))?;
                    
                    let out_template = format!("{}/%", &base_dir);
                    let out_cfg = format!("{}/0.cfg", &base_dir);

                    let ts_start = timestamp();

                    let thread: thread::JoinHandle<Result<usize, String>> = thread::spawn(move|| {
                        let bytes = backup(&mut std::io::stdin(),
                            "auth", 1_048_576, 
                            usize::MAX, &out_template, 
                            "pass", level, buf_size_bytes, Some(exit_flag_clone))?;

                        check(None::<StdoutWriter>, &out_cfg, "pass", buf_size_bytes, &None::<&str>, false)?;

                        Ok(bytes)
                    });

                    thread::sleep(std::time::Duration::from_millis(*duration as u64 * 1000));
                    //eprintln!("waking up");
                    exit_flag.store(true, std::sync::atomic::Ordering::SeqCst);
                    let bytes = thread.join().unwrap()?;
                    let ts_end = timestamp();
                    let ts_delta = ts_end - ts_start;

                    thrpts.push(Throughput{ 
                        level: *compress_level, 
                        buf_size: *buf_size, 
                        time_spent_s: ts_delta,
                        bytes: bytes,
                        bps: if ts_delta > 0 { bytes / ts_delta as usize } else { 0 }
                    });

                    fs::remove_dir_all(&base_dir).map_err(|e| format!("could not cleanup base directory {}: {}", &base_dir, e))?;
                }
            }

            thrpts.sort_by(|a,b| b.bps.cmp(&a.bps));
            println!("statistics gathered:");
            thrpts.into_iter().for_each(|t| {
                println!("speed = {} b/s\tbytes = {}\tseconds = {}\tlevel = {}\tbuffer = {} MB\t", 
                    t.bps, t.bytes, t.time_spent_s, t.level, t.buf_size);
            });

            Ok(())
        }
    }
}

fn main() -> ExitCode {
    let args = ArgOpts::parse();

    if let Err(e) = process_args(&args) {
        eprintln!("\nerror: {}\n", e);
        return ExitCode::from(1);
    } else {
        eprintln!("\ndone\n");
    }
    ExitCode::SUCCESS
}
