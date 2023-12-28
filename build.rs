use std::process::Command;

fn main() {
    for (export_env, git_args) in [
        ("GIT_REV",     &["rev-parse", "--short", "HEAD"]),
        ("GIT_BRANCH",  &["branch", "--quiet", "--show-current"])
    ]
    {
        let out = Command::new("git").args(git_args).output().unwrap();
        let out_str = String::from_utf8(out.stdout).unwrap();
        println!("cargo:rustc-env={}={}", export_env, out_str);
    }
}
