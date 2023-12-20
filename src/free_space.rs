use std::ffi::CString;
use std::mem;

pub fn get_free_space(mount_point: &str) -> Result<usize, String> {
    let c_mount_point = CString::new(mount_point.as_bytes())
        .map_err(|_| "null string passed as mountpoint".to_owned())?;

    let mut statvfs_struct = mem::MaybeUninit::<libc::statvfs>::uninit();
    unsafe {
        let ret_code = libc::statvfs(c_mount_point.as_ptr(), statvfs_struct.as_mut_ptr());
        if ret_code == 0 {
            let statvfs_struct = statvfs_struct.assume_init();

            let bsize = statvfs_struct.f_bsize as usize;
            let blocks = statvfs_struct.f_blocks as usize;
            let bfree = statvfs_struct.f_bfree as usize;
            let bavail = statvfs_struct.f_bavail as usize;

            if bsize == 0 || blocks == 0 || bfree > blocks || bavail > blocks {
                return Err("inconsitent filesystem data".to_owned());
            }

            return Ok(bfree * bsize);
        }
        else {
            return Err("bad mountpoint or filesystem to query".to_owned());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn good() {
        let x = get_free_space("/tmp").unwrap();
        println!("/tmp => {}", x);
        assert!(x > 0);
    }

    #[test]
    fn bad() {
        assert!(get_free_space("/sdkjfsd/sdkjfdk/sdkjh").is_err());
    }
}
