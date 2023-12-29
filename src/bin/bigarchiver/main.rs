use bigarchiver::arg_opts::{ArgOpts, Commands};
use bigarchiver::{backup, check};
use bigarchiver::file_set::cfg_from_pattern;
use bigarchiver::finalizable::DataSink;
use clap::Parser;
use std::io::{stdout, Write};
use std::process::ExitCode;

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
                pass, *compress_level, buf_size)?;
            if !no_check {
                let cfg_path = cfg_from_pattern(&out_template);
                eprintln!("verifying...");
                check(None::<StdoutWriter>, &cfg_path, pass, buf_size, &None::<&str>)
            } else {
                Ok(())
            }
        },
        Commands::Restore { config, pass, buf_size, check_free_space, no_check } => {
            let buf_size = *buf_size * 1_048_576;
            if !no_check {
                eprintln!("verifying before restore...");
                check(None::<StdoutWriter>, &config, pass, buf_size, &None)
                    .map_err(|e| format!("will not restore data, integrity check error: {}", e))?;
            }
            eprintln!("restoring...");
            let may_be_check = check_free_space.as_ref().map(|s| s.as_str());
            check(Some(StdoutWriter{}), &config, pass, 
                buf_size, &may_be_check)
                    .map_err(|e| format!("error restoring data: {}", e))
        },
        Commands::Check { config, pass, buf_size } => {
            eprintln!("verifying...");
            let buf_size = *buf_size * 1_048_576;
            check(None::<StdoutWriter>, &config, pass, 
                buf_size, &None)
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
