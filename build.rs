use std::process::Command;

fn git_cmd_val(args: Vec<&str>) -> String {
    let out = Command::new("git").args(args).output().unwrap();
    String::from_utf8(out.stdout).unwrap().trim().to_owned()
}

fn main() {
    println!("cargo:rustc-env=VERSION=0.0.2/{}/{}", 
        git_cmd_val(vec!["rev-parse", "--short", "HEAD"]),
        git_cmd_val(vec!["branch", "--quiet", "--show-current"]));
}
