use bigarchiver::arg_opts::{ArgOpts, ArgModeSpecificOpts};
use bigarchiver::{backup, check};
use bigarchiver::file_set::cfg_from_pattern;
use bigarchiver::finalizable::DataSink;
use std::io::{stdout, Write};

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
