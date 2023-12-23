use std::{ffi::OsString, collections::HashMap};

#[derive(Debug, PartialEq, Eq)]
pub enum ArgModeSpecificOpts {
    Backup {
        out_template: String,
        no_check: bool,
        auth: String,
        auth_every: usize,
        split_size: usize,
        compress_level: u8
    },
    Restore {
        config_path: String,
        check_free_space: Option<String>,
        no_check: bool,
    },
    Check {
        config_path: String
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ArgOpts {
    pub pass: String,
    pub buf_size: usize,
    pub mode_specific_opts: ArgModeSpecificOpts
}

#[derive(Debug, PartialEq, Eq, Hash)]
enum Mode {
    Backup, Restore, Check
}

#[derive(Debug, PartialEq, Eq)]
enum Kind {
    Single, Valued
}

#[derive(Debug, PartialEq, Eq)]
struct OptProp {
    must: bool,
    modes: HashMap<Mode, &'static str>,
    kind: Kind,
    val: Option<String>,
    sample_param: Option<&'static str>
}

impl ArgOpts {
    pub fn from_os_args(args: &Vec<OsString>) -> Result<ArgOpts, (String, String)> {
        let mut cfg: HashMap<&str, OptProp> = HashMap::from_iter(
            [
                ("backup", false, vec![(Mode::Backup, "select Backup mode: read data from stdin and write into output files(s)")], Kind::Single, None),
                ("restore", false, vec![(Mode::Restore, "select Restore mode: restore data from file(s) and write into stdout")], Kind::Single, None),
                ("check", false, vec![(Mode::Check, "select Check mode: check integrity of data from file(s)")], Kind::Single, None),

                ("pass", true, vec![
                    (Mode::Backup, "password to encrypt data with"),
                    (Mode::Restore, "password to decrypt data with"),
                    (Mode::Check, "password to use to check data with")
                ], Kind::Valued, Some("mysecret")),

                ("buf-size", true, vec![
                    (Mode::Backup, "buffer size for reading stdin data, in MB"),
                    (Mode::Restore, "buffer size for reading disk files, in MB"),
                    (Mode::Check, "buffer size for reading disk files, in MB")
                ], Kind::Valued, Some("256")),

                ("out-template", true, vec![(Mode::Backup, "template for output chunks; '%' symbols will transform into a sequence number")], Kind::Valued, Some("/path/to/files%%%%%%")),

                ("no-check", false, vec![
                    (Mode::Backup, "do not check the integrity of the whole archive after backup is done (the default is to always check)"),
                    (Mode::Restore, "do not check the integrity of the whole archive before actual restore (the default is to always check)")
                    ], Kind::Single, None),

                ("auth", true, vec![(Mode::Backup, "public authentication data to embed")], Kind::Valued, Some("\"My Full Name\"")),

                ("auth-every", true, vec![(Mode::Backup, "apply authentication to every portion of data of indicated size, in MB")], Kind::Valued, Some("32")),

                ("split-size", true, vec![(Mode::Backup, "size of output chunks, in MB")], Kind::Valued, Some("1024")),

                ("compress-level", true, vec![(Mode::Backup, "XZ compression level, 0 - 9")], Kind::Valued, Some("6")),

                ("check-free-space", false, vec![(Mode::Restore, "check free space available on the indicated filesystem before restore")], Kind::Valued, Some("/data")),

                ("config", true, vec![
                    (Mode::Restore, "full path to config file of the archive to restore"),
                    (Mode::Check, "full path to config file of the archive to check")
                    ], Kind::Valued, Some("/path/to/files000000.cfg")),

            ].into_iter().map(|(c, must, m, k, sample)|(c, OptProp{ 
                must, modes: HashMap::from_iter(m.into_iter()), kind: k, val: None, sample_param: sample }
            ))
        );

        let mut usage = String::from("Usage:\n\n");
        for (title, mode, selector_option) in [
            ("1. to pack data coming from stdin into files", Mode::Backup, "backup"),
            ("2. to unpack data from files to stdout", Mode::Restore, "restore"),
            ("3. to verify the integrify of data from files", Mode::Check, "check")]
        {
            usage.push_str(title);
            usage.push_str(":\n\n./bigarchiver --");

            let mode_cfg = cfg.iter().filter_map(|(opt_name, opt_prop)| {
                if let Some(descr) = opt_prop.modes.get(&mode) {
                    Some((opt_name, descr, opt_prop.must, opt_prop.sample_param))
                } else {
                    None
                }
            }).collect::< Vec<(&&str, &&str, bool, Option<&str>)> >();

            usage.push_str(selector_option);
            for (opt_name, _, must, opt_sample) in mode_cfg
                .iter()
                .filter(|(opt_name, _, _, _)| opt_name != &&selector_option)
            {
                if !*must { usage.push_str(" ["); }
                usage.push_str(" --");
                usage.push_str(opt_name);
                if let Some(opt_sample) = opt_sample {
                    usage.push_str(" ");
                    usage.push_str(opt_sample);
                }
                if !*must { usage.push_str(" ]"); }
            }
            usage.push_str("\n\nwhere:\n\n");

            for (opt_name, descr, _, _) in mode_cfg {
                usage.push_str(format!("\t--{}\n\t\t{}\n", opt_name, descr).as_str());
            }
            usage.push_str("\n\n");
        }

        let mut args = args
            .iter()
            .cloned()
            .map(|os| os.into_string())
            .collect::<Result<Vec<String>, OsString>>()
            .map_err(|_| ("invalid encoding".to_string(), usage.clone()))?
            .into_iter();

        //println!("{cfg:#?}");

        // track which options are given
        while let Some(arg) = args.next() {
            //println!("processing arg '{arg}'");
            if !arg.starts_with("--") {
                return Err((format!("invalid argument '{}'", arg), usage.clone()));
            }
            let arg = &arg[2..];

            if let Some(prop) = cfg.get_mut(arg) {
                match prop.kind {
                    Kind::Single => prop.val = Some(String::new()),
                    Kind::Valued => prop.val = Some(args.next().ok_or((format!("missing parameter for option '--{}'", arg), usage.clone()))?)
                }
            } else {
                return Err((format!("unknown argument '{}'", arg), usage.clone()));
            }
        }

        // detect mode
        let mut mode: Option<Mode> = None;
        let mut mode_counter = 0;
        if cfg.get("backup").is_some_and(|v| v.val.is_some()) {
            mode = Some(Mode::Backup);
            mode_counter += 1;
        } 
        if cfg.get("restore").is_some_and(|v| v.val.is_some()) {
            mode = Some(Mode::Restore);
            mode_counter += 1;
        }
        if cfg.get("check").is_some_and(|v| v.val.is_some()) {
            mode = Some(Mode::Check);
            mode_counter += 1;
        }

        if mode_counter > 1 {
            return Err(("--backup, --restore and --check are mututally-exclusive".to_owned(), usage));
        }
        if mode.is_none() {
            return Err(("either --backup or --restore or --check must be provided".to_owned(), usage));
        }
        let mode = mode.unwrap();

        // must-have mode-specific options must be given depending on the mode
        if !cfg.iter()
            .filter(|(_,p)| p.must && p.modes.contains_key(&mode))
            .all(|(_,p)| p.val.is_some())
        {
            return Err(("not all mandatory arguments are provided for chosen mode".to_owned(), usage));
        }

        // options for other mode(s) must no be present
        if !cfg.iter()
            .filter(|(_,p)| p.val.is_some())
            .all(|(_,p)| p.modes.contains_key(&mode) )
        {
            return Err(("excessive options are provided for chosen mode".to_owned(), usage));
        }

        Ok(Self {
            pass: cfg.get("pass").unwrap().val.clone().unwrap(),
            buf_size: cfg.get("buf-size").unwrap().val.clone().unwrap().parse::<usize>()
                .map_err(|_| ("invalid numeric value for '--buf-size'".to_owned(), usage.clone()))? * 1_048_576,
            mode_specific_opts: match mode {
                Mode::Backup => ArgModeSpecificOpts::Backup {
                    out_template: cfg.get("out-template").unwrap().val.clone().unwrap(),
                    no_check: cfg.get("no-check").unwrap().val.is_some(),
                    auth: cfg.get("auth").unwrap().val.clone().unwrap(),
                    auth_every: cfg.get("auth-every").unwrap().val.clone().unwrap().parse::<usize>()
                        .map_err(|_| ("invalid numeric value for '--auth-every'".to_owned(), usage.clone()))? * 1_048_576,
                    split_size: cfg.get("split-size").unwrap().val.clone().unwrap().parse::<usize>()
                        .map_err(|_| ("invalid numeric value for '--split-size'".to_owned(), usage.clone()))? * 1_048_576,
                    compress_level: cfg.get("compress-level").unwrap().val.clone().unwrap().parse::<u8>()
                        .map_err(|_| ("invalid numeric value for '--compress-level'".to_owned(), usage.clone()))?,
                },
                Mode::Restore => ArgModeSpecificOpts::Restore {
                    config_path: cfg.get("config").unwrap().val.clone().unwrap(),
                    no_check: cfg.get("no-check").unwrap().val.is_some(),
                    check_free_space: cfg.get("check-free-space").unwrap().val.clone()
                },
                Mode::Check => ArgModeSpecificOpts::Check {
                    config_path: cfg.get("config").unwrap().val.clone().unwrap(),
                },
            }
        })

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    fn to_os(vs: &Vec<&str>) -> Vec<OsString> {
        vs.into_iter().map(|s| OsString::from(s)).collect::<Vec<OsString>>()
    }

    #[test]
    fn unknown_opt() {
        ArgOpts::from_os_args(&to_os(&vec!["--dir", "--b"])).unwrap_err();
    }

    #[test]
    fn missing_param() {
        ArgOpts::from_os_args(&to_os(&vec!["--out-template", "outval"])).unwrap_err();
    }
    
    #[test]
    fn missing_val() {
        ArgOpts::from_os_args(&to_os(&vec!["--out-template", "outval", "--buf-size"])).unwrap_err();
    }

    #[test]
    fn bad_num() {
        ArgOpts::from_os_args(&to_os(&vec!["--out-template", "outval", "--buf-size", "x123", "--no-check"])).unwrap_err();
    }

    #[test]
    fn backup_opts() {
        assert_eq!(
            ArgOpts::from_os_args(&to_os(&vec![
                "--backup", "--out-template", "outval", "--pass", "passval", "--auth", "authval",
                "--auth-every", "100", "--split-size", "1000", "--compress-level", "5",
                "--buf-size", "10", "--no-check"
                ])).unwrap(),
            ArgOpts{
                    pass: "passval".to_owned(),
                    buf_size: 10485760,
                    mode_specific_opts: ArgModeSpecificOpts::Backup {
                        out_template: "outval".to_owned(),
                        auth: "authval".to_owned(),
                        auth_every: 104857600,
                        split_size: 1048576000,
                        compress_level: 5,
                        no_check: true
                    }
            });
        assert_eq!(
            ArgOpts::from_os_args(&to_os(&vec![
                "--backup", "--out-template", "outval", "--pass", "passval", "--auth", "authval",
                "--auth-every", "100", "--split-size", "1000", "--compress-level", "5",
                "--buf-size", "10"
                ])).unwrap(),
                ArgOpts{
                    pass: "passval".to_owned(),
                    buf_size: 10485760,
                    mode_specific_opts: ArgModeSpecificOpts::Backup {
                        out_template: "outval".to_owned(),
                        auth: "authval".to_owned(),
                        auth_every: 104857600,
                        split_size: 1048576000,
                        compress_level: 5,
                        no_check: false
                    }
            });
    }

    #[test]
    fn restore_opts() {
        assert_eq!(
            ArgOpts::from_os_args(&to_os(&vec![
                "--restore", "--config", "configval", "--pass", "passval", "--buf-size", "10", "--check-free-space", "/mount"
                ])).unwrap(),
            ArgOpts{
                    pass: "passval".to_owned(),
                    buf_size: 10485760,
                    mode_specific_opts: ArgModeSpecificOpts::Restore {
                        config_path: "configval".to_owned(),
                        no_check: false,
                        check_free_space: Some("/mount".to_owned())
                    }
            });
        assert_eq!(
            ArgOpts::from_os_args(&to_os(&vec![
                "--restore", "--config", "configval", "--pass", "passval", "--buf-size", "10", "--no-check"
                ])).unwrap(),
            ArgOpts{
                    pass: "passval".to_owned(),
                    buf_size: 10485760,
                    mode_specific_opts: ArgModeSpecificOpts::Restore {
                        config_path: "configval".to_owned(),
                        no_check: true,
                        check_free_space: None
                    }
            });
    }


    #[test]
    fn check_opts() {
        assert_eq!(
            ArgOpts::from_os_args(&to_os(&vec![
                "--check", "--config", "configval", "--pass", "passval", "--buf-size", "10"
                ])).unwrap(),
            ArgOpts{
                    pass: "passval".to_owned(),
                    buf_size: 10485760,
                    mode_specific_opts: ArgModeSpecificOpts::Check {
                        config_path: "configval".to_owned(),
                    }
            });

        let (e, u) = ArgOpts::from_os_args(&to_os(&vec![
            "--check", "--config", "configval", "--pass", "passval", "--buf-size", "123", "--split-size", "200"
            ])).unwrap_err();
        println!("Error: {}\n\n=== usage start ===\n{}\n=== usage stop ====", e, u);

        assert!(
            ArgOpts::from_os_args(&to_os(&vec![
                "--check", "--config", "configval", "--pass", "passval"
                ])).is_err());
        assert!(
            ArgOpts::from_os_args(&to_os(&vec![
                "--check", "--pass", "passval", "--buf-size", "123"
                ])).is_err());
    }
}
