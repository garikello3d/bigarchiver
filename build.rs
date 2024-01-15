use std::io::{Write, Read};
use std::process::Command;
use std::env;
use std::fs::File;
use std::io::{Error, ErrorKind};

const VER_FILE_NAME: &str = ".VERSION";

fn git_cmd_val(args: Vec<&str>) -> Result<String, String> {
    let out = Command::new("git")
        .args(args)
        .output()
        .map_err(|e| format!("could not run git: {}", e))?;
    if !out.status.success() {
        return Err("git command failed".to_owned());
    }
    Ok(String::from_utf8(out.stdout).unwrap().trim().to_owned())
}

fn version_from_git() -> Result<(String, String), String> {
    Ok((
        git_cmd_val(vec!["rev-parse", "--short", "HEAD"])?,
        git_cmd_val(vec!["branch", "--quiet", "--show-current"])?
    ))
}

fn version_from_file(filename: &str) -> Result<(String, String), String> {
    let mut f = File::open(filename)
        .map_err(|e| format!("could not open version file {}: {}", filename, e))?;

    let mut contents = String::new();
    f.read_to_string(&mut contents)
        .map_err(|e| format!("could not read version file {}: {}", filename, e))?;

    let splitted = contents.split(' ').collect::<Vec<&str>>();
    if splitted.len() != 2 {
        return Err(format!("bad contents of version file {}: {}", filename, contents));
    }

    Ok((splitted[0].to_owned(), splitted[1].to_owned()))
}

fn version_to_file(filename: &str, rev: &str, branch: &str) -> Result<(), String> {
    let mut f = File::create(filename)
        .map_err(|e| format!("could not create/truncate version file {}: {}", filename, e))?;

    f.write_all(format!("{} {}", rev, branch).as_bytes())
        .map_err(|e| format!("could write to version file {}: {}", filename, e))?;

    Ok(())
}

fn main() -> Result<(), std::io::Error> {
    let base_dir = env::vars()
        .find(|(name, _)| name == "CARGO_MANIFEST_DIR")
        .ok_or(Error::new(ErrorKind::Other, "CARGO_MANIFEST_DIR env not found"))?
        .1;
    println!("base dir = {}", &base_dir);

    let version_file = &format!("{}/{}", base_dir, VER_FILE_NAME);

    let (rev, branch) = if let Ok((rev, branch)) = version_from_git() {
        println!("got version from git, saving to file");
        version_to_file(version_file, &rev, &branch)
            .map_err(|e| Error::new(ErrorKind::Other, e))?;
        println!("saved to file");
        (rev, branch)
    }
    else {
        println!("could not get version from git, trying from file");
        version_from_file(version_file).map_err(|_| std::io::ErrorKind::Other)?
    };

    println!("cargo:rustc-env=VERSION=0.0.2/{}/{}", rev, branch);
        
    Ok(())
}
