use std::{ffi::OsString, collections::{HashMap, HashSet}};

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
    modes: HashSet<Mode>,
    kind: Kind,
    val: Option<String>
}

impl ArgOpts {
    pub fn from_os_args(args: &Vec<OsString>) -> Result<ArgOpts, String> {
        let mut cfg: HashMap<&str, OptProp> = HashMap::from_iter(
            [
                ("backup",              false,  vec![Mode::Backup],                             Kind::Single),
                ("restore",             false,  vec![Mode::Restore],                            Kind::Single),
                ("check",               false,  vec![Mode::Check],                              Kind::Single),

                ("pass",                true,   vec![Mode::Backup, Mode::Restore, Mode::Check], Kind::Valued),
                ("buf-size",             true,   vec![Mode::Backup, Mode::Restore, Mode::Check], Kind::Valued),

                ("out-template",        true,   vec![Mode::Backup],                             Kind::Valued),
                ("no-check",            false,  vec![Mode::Backup, Mode::Restore],              Kind::Single),
                ("auth",                true,   vec![Mode::Backup],                             Kind::Valued),
                ("auth-every",          true,   vec![Mode::Backup],                             Kind::Valued),
                ("split-size",          true,   vec![Mode::Backup],                             Kind::Valued),
                ("compress-level",      true,   vec![Mode::Backup],                             Kind::Valued),

                ("check-free-space",    false,  vec![Mode::Restore],                            Kind::Valued),
                ("config",              true,   vec![Mode::Restore, Mode::Check],               Kind::Valued),
            ].into_iter().map(|(c, must, m, k)|(c, OptProp{ 
                must, modes: HashSet::from_iter(m.into_iter()), kind: k, val: None }
            ))
        );

        let mut args = args
            .iter()
            .cloned()
            .map(|os| os.into_string())
            .collect::<Result<Vec<String>, OsString>>()
            .map_err(|_| "invalid encoding".to_string())?
            .into_iter();

        //println!("{cfg:#?}");

        // track which options are given
        while let Some(arg) = args.next() {
            //println!("processing arg '{arg}'");
            if !arg.starts_with("--") {
                return Err(format!("invalid argument '{}'", arg));
            }
            let arg = &arg[2..];

            if let Some(prop) = cfg.get_mut(arg) {
                match prop.kind {
                    Kind::Single => prop.val = Some(String::new()),
                    Kind::Valued => prop.val = Some(args.next().ok_or(format!("missing parameter for option '--{}'", arg))?)
                }
            } else {
                return Err(format!("unknown argument '{}'", arg));
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
            return Err("--backup, --restore and --check are mututally-exclusive".to_owned());
        }
        if mode.is_none() {
            return Err("either --backup or --restore or --check must be provided".to_owned());
        }
        let mode = mode.unwrap();

        // must-have mode-specific options must be given depending on the mode
        if !cfg.iter()
            .filter(|(_,p)| p.must && p.modes.contains(&mode))
            .all(|(_,p)| p.val.is_some())
        {
            return Err("not all mandatory arguments are provided for chosen mode".to_owned());
        }

        // options for other mode(s) must no be present
        if !cfg.iter()
            .filter(|(_,p)| p.val.is_some())
            .all(|(_,p)| p.modes.contains(&mode) )
        {
            return Err("excessive options are provided for chosen mode".to_owned());
        }

        Ok(Self {
            pass: cfg.get("pass").unwrap().val.clone().unwrap(),
            buf_size: cfg.get("buf-size").unwrap().val.clone().unwrap().parse::<usize>()
                .map_err(|_| "invalid numeric value for '--buf-size'".to_owned())? * 1_048_576,
            mode_specific_opts: match mode {
                Mode::Backup => ArgModeSpecificOpts::Backup {
                    out_template: cfg.get("out-template").unwrap().val.clone().unwrap(),
                    no_check: cfg.get("no-check").unwrap().val.is_some(),
                    auth: cfg.get("auth").unwrap().val.clone().unwrap(),
                    auth_every: cfg.get("auth-every").unwrap().val.clone().unwrap().parse::<usize>()
                        .map_err(|_| "invalid numeric value for '--auth-every'".to_owned())? * 1_048_576,
                    split_size: cfg.get("split-size").unwrap().val.clone().unwrap().parse::<usize>()
                        .map_err(|_| "invalid numeric value for '--split-size'".to_owned())? * 1_048_576,
                    compress_level: cfg.get("compress-level").unwrap().val.clone().unwrap().parse::<u8>()
                        .map_err(|_| "invalid numeric value for '--compress-level'".to_owned())?,
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
        assert!(
            ArgOpts::from_os_args(&to_os(&vec![
                "--check", "--config", "configval", "--pass", "passval", "--buf-size", "123", "--split-size", "200"
                ])).is_err());
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
