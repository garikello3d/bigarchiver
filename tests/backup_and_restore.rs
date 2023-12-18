use bigarchiver::{backup,check};

mod common;
use common::clear_archives_if_any;

#[test]
fn all_ok() {
    clear_archives_if_any();
}