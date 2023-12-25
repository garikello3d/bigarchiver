use bigarchiver::arg_opts::{ArgOpts, ArgModeSpecificOpts};
use bigarchiver::{backup, check};
use bigarchiver::file_set::cfg_from_pattern;
use bigarchiver::finalizable::DataSink;
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
    match &args.mode_specific_opts {
        ArgModeSpecificOpts::Backup { 
            out_template, no_check, auth, auth_every, split_size, compress_level
        } => {
            eprintln!("backing up...");
            backup(&mut std::io::stdin(),
                &auth, *auth_every, 
                *split_size, &out_template, 
                &args.pass, *compress_level, args.buf_size)?;
            if !no_check {
                let cfg_path = cfg_from_pattern(&out_template);
                eprintln!("verifying...");
                check(None::<StdoutWriter>, &cfg_path, &args.pass, args.buf_size, &None::<&str>)
            } else {
                Ok(())
            }
        },
        ArgModeSpecificOpts::Restore { config_path, no_check, check_free_space } => {
            if !no_check {
                eprintln!("verifying before restore...");
                check(None::<StdoutWriter>, &config_path, &args.pass, args.buf_size, &None)
                    .map_err(|e| format!("will not restore data, integrity check error: {}", e))?;
            }
            eprintln!("restoring...");
            let may_be_check = check_free_space.as_ref().map(|s| s.as_str());
            check(Some(StdoutWriter{}), &config_path, &args.pass, 
                args.buf_size, &may_be_check)
                    .map_err(|e| format!("error restoring data: {}", e))
        },
        ArgModeSpecificOpts::Check { config_path } => {
            eprintln!("verifying...");
            check(None::<StdoutWriter>, &config_path, &args.pass, 
                args.buf_size, &None)
        }
    }
}

fn main() -> ExitCode {
    let args = {
        let args = ArgOpts::from_os_args(&std::env::args_os().skip(1).collect());
        if let Err((err_msg, usage)) = &args {
            eprintln!("{}\n\n{}", err_msg, usage);
            return ExitCode::from(2);
        };
        args.unwrap()
    };

    if let Err(e) = process_args(&args) {
        eprintln!("\nerror: {}\n", e);
        return ExitCode::from(1);
    } else {
        eprintln!("\ndone\n");
    }
    ExitCode::SUCCESS
}
