pub fn analyze_pattern(patt: &str) -> Result<(usize, usize), String> { // offset inside original string - length
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

//TODO refactor/rewrite this mess of functions below

pub fn gen_cfg_path(patt_str: &str, patt_offs: usize, patt_size: usize) -> Result<String, String> {
    Ok(format!("{}.cfg", gen_chunk_path(patt_str, 0, patt_offs, patt_size)?))
}

fn replace_only_last_path_component(s: String, from: char, to: char) -> String {
    let last_slash = s.rfind(std::path::MAIN_SEPARATOR_STR).unwrap_or(0);
    let left = &s[..last_slash];
    let right = &s[last_slash..];
    format!("{}{}", left, right.replace(&from.to_string(), &to.to_string()))
}

pub fn pattern_from_cfg(p: &str) -> Result<String, String> {
    if !p.ends_with(".cfg") {
        Err("metadata file should end with .cfg".to_owned())
    } else {
        let p = String::from(&p[..p.len()-4]);
        Ok(replace_only_last_path_component(p, '0', '%'))
    }
}

pub fn cfg_from_pattern(p: &str) -> Result<String, String> {
    Ok(replace_only_last_path_component(format!("{}.cfg", p), '%', '0'))
}

pub fn gen_chunk_path(patt_str: &str, nr: usize, patt_offs: usize, patt_size: usize) -> Result<String, String> {
    let s_chunk = nr.to_string();
    if s_chunk.len() > patt_size {
        return Err(format!("too small pattern ({}) to fit next chunk number {}", patt_size, s_chunk));
    }
    let mut out = patt_str.to_owned();
    let nr_zeros = patt_size - s_chunk.len();
    if nr_zeros > 0 {
        out.replace_range(patt_offs..patt_offs + nr_zeros, &"0".repeat(nr_zeros));
    }
    out.replace_range(patt_offs + nr_zeros..patt_offs + patt_size, &s_chunk);
    Ok(out)
}

mod tests {
    use crate::patterns::{analyze_pattern, gen_chunk_path, pattern_from_cfg, cfg_from_pattern};

    #[test]
    fn empty_or_no_percent() {
        assert_eq!(Err("pattern character % not found".to_owned()), analyze_pattern(""));
        assert_eq!(Err("pattern character % not found".to_owned()), analyze_pattern("askjd"));
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
    }

    #[test]
    fn good_two_patterns_len2() {
        assert_eq!(Err("ambigous pattern".to_owned()), analyze_pattern("asd%%li%%kj"));
        assert_eq!(Err("ambigous pattern".to_owned()), analyze_pattern("%%asdli%%kj"));
        assert_eq!(Err("ambigous pattern".to_owned()), analyze_pattern("%%asdlikj%%"));
    }

    #[test]
    fn gen_path_enough() {
        assert_eq!(Ok("ab015def".to_owned()), gen_chunk_path("ab%%%def", 15, 2, 3));
        assert_eq!(Ok("a0015def".to_owned()), gen_chunk_path("a%%%%def", 15, 1, 4));
        assert_eq!(Ok("a1234def".to_owned()), gen_chunk_path("a%%%%def", 1234, 1, 4));
        assert_eq!(Ok("0015def".to_owned()), gen_chunk_path("%%%%def", 15, 0, 4));
        assert_eq!(Ok("1234def".to_owned()), gen_chunk_path("%%%%def", 1234, 0, 4));
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
        assert_eq!(Ok("/path/to0/di0r/out00.cfg".to_owned()), cfg_from_pattern("/path/to0/di0r/out%%"));
        assert_eq!(Ok("out00.cfg".to_owned()), cfg_from_pattern("out%%"));
        assert_eq!(Ok("/out00.cfg".to_owned()), cfg_from_pattern("/out%%"));
    }

}
