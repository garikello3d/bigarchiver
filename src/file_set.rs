pub struct FileSet {
    pattern_path: String,
    config_path: String,
    offset: usize,
    len: usize
}

impl FileSet {
    pub fn from_pattern(pattern: &str) -> Result<Self, String> {
        let (offset, len) = analyze_pattern(pattern)?;
        Ok(Self{
            pattern_path: String::from(pattern),
            config_path: cfg_from_pattern(pattern),
            offset,
            len})
    }

    pub fn from_cfg_path(cfg_path: &str) -> Result<Self, String> {
        let pattern = pattern_from_cfg(cfg_path)?;
        let (offset, len) = analyze_pattern(pattern.as_str())?;
        Ok(Self{
            pattern_path: pattern,
            config_path: String::from(cfg_path),
            offset,
            len})
    }

    pub fn pattern(&self) -> String {
        self.pattern_path.clone()
    }

    pub fn cfg_path(&self) -> String {
        self.config_path.clone()
    }

    pub fn gen_file_path(&self, n: usize) -> String {
        let s_chunk = n.to_string();
        let mut out = self.pattern_path.clone();
        let mut nr_zeros: i32 = self.len as i32 - s_chunk.len() as i32;
        if nr_zeros > 0 {
            out.replace_range(self.offset .. self.offset + nr_zeros as usize, &"0".repeat(nr_zeros as usize));
        }
        if nr_zeros < 0 {
            nr_zeros = 0;
        }
        out.replace_range(self.offset + nr_zeros as usize .. self.offset + self.len, &s_chunk);
        out
    }
}

fn analyze_pattern(patt: &str) -> Result<(usize, usize), String> { // offset inside original string and length
    let mut nr_seqs = 0;
    let mut seq_len = 0;
    let mut finished_seq_len = 0;
    let mut prev_char = None::<char>;
    let mut pos = 0;
    let mut pat_start_pos = 0;
    for c in patt.chars() {
        if seq_len == 0 {
            if c == '%' {
                seq_len = 1;
                pat_start_pos = pos;
            }
            prev_char = Some(c);
        } else {
            if prev_char.unwrap() == '%' { // SAFE: checked for not None above
                if c == '%' { // continue to collect current sequence
                    seq_len += 1;
                } else { // current sequence is over
                    nr_seqs += 1;
                    if nr_seqs > 1 {
                        return Err("ambigous pattern".to_owned());
                    }
                    finished_seq_len = seq_len;
                    seq_len = 0;
                }
            }
            prev_char = Some(c);
        }
        pos += 1;
    }
    if seq_len != 0 {
        finished_seq_len = seq_len;
    }
    if seq_len == 0 && nr_seqs == 0 {
        return Err("pattern character % not found".to_owned());
    }
    if seq_len > 0 && nr_seqs > 0 {
        return Err("ambigous pattern".to_owned());
    }
    Ok((pat_start_pos, finished_seq_len))
}

fn replace_only_last_path_component(s: String, from: char, to: char) -> String {
    let last_slash = s.rfind(std::path::MAIN_SEPARATOR_STR).unwrap_or(0);
    let left = &s[..last_slash];
    let right = &s[last_slash..];
    format!("{}{}", left, right.replace(&from.to_string(), &to.to_string()))
}

fn pattern_from_cfg(p: &str) -> Result<String, String> {
    if !p.ends_with(".cfg") {
        Err("metadata file should end with .cfg".to_owned())
    } else {
        let p = String::from(&p[..p.len()-4]);
        Ok(replace_only_last_path_component(p, '0', '%'))
    }
}

pub fn cfg_from_pattern(p: &str) -> String {
    replace_only_last_path_component(format!("{}.cfg", p), '%', '0')
}

#[cfg(test)]
mod tests {
    use super::{analyze_pattern, pattern_from_cfg, cfg_from_pattern};
    use super::FileSet;

    #[test]
    fn empty_or_no_percent() {
        assert_eq!(Err("pattern character % not found".to_owned()), analyze_pattern(""));
        assert_eq!(Err("pattern character % not found".to_owned()), analyze_pattern("askjd"));
        assert!(FileSet::from_pattern("sdfsdf").is_err());
    }

    #[test]
    fn good_single_pattern_len1() {
        assert_eq!(Ok((3, 1)), analyze_pattern("asd%likj"));
        assert_eq!(Ok((0, 1)), analyze_pattern("%asdlikj"));
        assert_eq!(Ok((7, 1)), analyze_pattern("asdlikj%"));
    }

    #[test]
    fn good_single_pattern_len2() {
        assert_eq!(Ok((3, 2)), analyze_pattern("asd%%likj"));
        assert_eq!(Ok((0, 2)), analyze_pattern("%%asdlikj"));
        assert_eq!(Ok((7, 2)), analyze_pattern("asdlikj%%"));
    }

    #[test]
    fn good_single_pattern_len5() {
        assert_eq!(Ok((2, 5)), analyze_pattern("as%%%%%kj"));
        assert_eq!(Ok((0, 5)), analyze_pattern("%%%%%likj"));
        assert_eq!(Ok((4, 5)), analyze_pattern("asdl%%%%%"));
    }

    #[test]
    fn good_two_patterns_len1() {
        assert_eq!(Err("ambigous pattern".to_owned()), analyze_pattern("asd%li%kj"));
        assert_eq!(Err("ambigous pattern".to_owned()), analyze_pattern("%asdli%kj"));
        assert_eq!(Err("ambigous pattern".to_owned()), analyze_pattern("%asdlikj%"));
        assert!(FileSet::from_pattern("%asdlikj%").is_err());
    }

    #[test]
    fn good_two_patterns_len2() {
        assert_eq!(Err("ambigous pattern".to_owned()), analyze_pattern("asd%%li%%kj"));
        assert_eq!(Err("ambigous pattern".to_owned()), analyze_pattern("%%asdli%%kj"));
        assert_eq!(Err("ambigous pattern".to_owned()), analyze_pattern("%%asdlikj%%"));
    }

    #[test]
    fn gen_path_enough() {
        assert_eq!("ab015def".to_owned(), FileSet::from_pattern("ab%%%def").unwrap().gen_file_path(15));
        assert_eq!("a0015def".to_owned(), FileSet::from_pattern("a%%%%def").unwrap().gen_file_path(15));
        assert_eq!("a1234def".to_owned(), FileSet::from_pattern("a%%%%def").unwrap().gen_file_path(1234));
        assert_eq!("0015def".to_owned(),  FileSet::from_pattern("%%%%def").unwrap().gen_file_path(15));
        assert_eq!("1234def".to_owned(),  FileSet::from_pattern("%%%%def").unwrap().gen_file_path(1234));
    }

    #[test]
    fn gen_path_patt_expanding() {
        assert_eq!("ab1234def".to_owned(), FileSet::from_pattern("ab%%%def").unwrap().gen_file_path(1234));
        assert_eq!("a12345def".to_owned(), FileSet::from_pattern("a%%%%def").unwrap().gen_file_path(12345));
        assert_eq!("a12345def".to_owned(), FileSet::from_pattern("a%%%%def").unwrap().gen_file_path(12345));
        assert_eq!("123456def".to_owned(), FileSet::from_pattern("%%%%def").unwrap().gen_file_path(123456));
        assert_eq!("12345def".to_owned(),  FileSet::from_pattern("%%%%def").unwrap().gen_file_path(12345));
    }

    #[test]
    fn gen_path_cfg_expanding() {
        assert_eq!("ab1234def".to_owned(), FileSet::from_cfg_path("ab000def.cfg").unwrap().gen_file_path(1234));
        assert_eq!("a12345def".to_owned(), FileSet::from_cfg_path("a0000def.cfg").unwrap().gen_file_path(12345));
        assert_eq!("a12345def".to_owned(), FileSet::from_cfg_path("a0000def.cfg").unwrap().gen_file_path(12345));
        assert_eq!("123456def".to_owned(), FileSet::from_cfg_path("0000def.cfg").unwrap().gen_file_path(123456));
        assert_eq!("12345def".to_owned(),  FileSet::from_cfg_path("0000def.cfg").unwrap().gen_file_path(12345));
    }

    #[test]
    fn gen_long_path_cfg_expanding() {
        assert_eq!("/p0ath/p00ath/p000ath/p00a00th/ab1234def".to_owned(), FileSet::from_cfg_path("/p0ath/p00ath/p000ath/p00a00th/ab000def.cfg").unwrap().gen_file_path(1234));
        assert_eq!("/p0ath/p00ath/p000ath/p00a00th/a12345def".to_owned(), FileSet::from_cfg_path("/p0ath/p00ath/p000ath/p00a00th/a0000def.cfg").unwrap().gen_file_path(12345));
        assert_eq!("/p0ath/p00ath/p000ath/p00a00th/a12345def".to_owned(), FileSet::from_cfg_path("/p0ath/p00ath/p000ath/p00a00th/a0000def.cfg").unwrap().gen_file_path(12345));
        assert_eq!("/p0ath/p00ath/p000ath/p00a00th/123456def".to_owned(), FileSet::from_cfg_path("/p0ath/p00ath/p000ath/p00a00th/0000def.cfg").unwrap().gen_file_path(123456));
        assert_eq!("/p0ath/p00ath/p000ath/p00a00th/12345def".to_owned(),  FileSet::from_cfg_path("/p0ath/p00ath/p000ath/p00a00th/0000def.cfg").unwrap().gen_file_path(12345));
    }

    #[test]
    fn patt_from_cfg() {
        assert_eq!(Ok("/path/to0/di0r/out%%".to_owned()), pattern_from_cfg("/path/to0/di0r/out00.cfg"));
        assert_eq!(Ok("out%%".to_owned()), pattern_from_cfg("out00.cfg"));
        assert_eq!(Ok("/out%%".to_owned()), pattern_from_cfg("/out00.cfg"));
        assert!(pattern_from_cfg("out00.cfgg").is_err());
    }

    #[test]
    fn cfg_from_patt() {
        assert_eq!("/path/to0/di0r/out00.cfg".to_owned(), cfg_from_pattern("/path/to0/di0r/out%%"));
        assert_eq!("out00.cfg".to_owned(), cfg_from_pattern("out%%"));
        assert_eq!("/out00.cfg".to_owned(), cfg_from_pattern("/out%%"));
    }

}
