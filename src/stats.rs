use std::io::Read;
use std::collections::HashMap;
use std::num::ParseIntError;

#[derive(Default, PartialEq, Eq, Debug)]
pub struct Stats {
    pub in_data_len: usize,
    pub in_data_hash: u64,
    pub hash_seed: u64,
    pub compressed_len: usize,
    pub out_nr_chunks: usize,
    pub out_chunk_size: usize,
    pub alg: String,
    pub auth_string: String,
    pub auth_chunk_size: usize,
    pub misc_info: Option<String>,
}

impl Stats {
    pub fn new() -> Self {
        Self { ..Default::default() }
    }

    pub fn from_readable(mut r: impl Read) -> Result<Self, String> {
        let mut s = String::new();
        let mut map = HashMap::new();

        let _ = r
            .read_to_string(&mut s)
            .map_err(|e|format!("cannot read metadata: {}", e))?;

        for line in s.split("\n").map(|ln| ln.trim()).filter(|ln| ln.len() > 0) {
            let delim_pos = line.find('=').ok_or(format!("invalid metadata line: '{}'", line))?;
            if delim_pos == 0 {
                return Err(format!("empty param name in metadata: '{}'", line));
            }
            let param = &line[.. delim_pos];
            let val = &line[delim_pos + 1 ..];            
            if !map.insert(param, val).is_none() {
                return Err(format!("duplicate key: '{}'", param));
            }
        }

        Ok(Self {
                in_data_len: Self::get_and_parse::<_, _>(&map, "in_len", |v| { v.parse::<usize>() })?,
                in_data_hash: Self::get_and_parse::<_, _>(&map, "in_hash", |v| { u64::from_str_radix(v, 16) })?,
                hash_seed: Self::get_and_parse::<_, _>(&map, "hash_seed", |v| { u64::from_str_radix(v, 16) })?,
                compressed_len: Self::get_and_parse::<_, _>(&map, "xz_len", |v| { v.parse::<usize>() })?,
                out_nr_chunks: Self::get_and_parse::<_, _>(&map, "nr_chunks", |v| { v.parse::<usize>() })?,
                out_chunk_size: Self::get_and_parse::<_, _>(&map, "chunk_len", |v| { v.parse::<usize>() })?,
                alg: Self::get(&map, "alg")?.to_owned(),
                auth_chunk_size: Self::get_and_parse::<_, _>(&map, "auth_len", |v| { v.parse::<usize>() })?,
                auth_string: Self::get(&map, "auth")?.to_owned(),
                misc_info: map.get("misc_info").map(|s| s.to_string())
        })
    }

    pub fn as_string(&self) -> String {
        format!("\
                in_len={}\n\
                in_hash={:016x}\n\
                hash_seed={:016x}\n\
                xz_len={}\n\
                nr_chunks={}\n\
                chunk_len={}\n\
                alg={}\n\
                auth={}\n\
                auth_len={}\n\
                misc_info={}\n",
                self.in_data_len,
                self.in_data_hash,
                self.hash_seed,
                self.compressed_len,
                self.out_nr_chunks,
                self.out_chunk_size,
                self.alg,
                self.auth_string, 
                self.auth_chunk_size,
                self.misc_info.as_ref().unwrap_or(&String::new()))
    }

    fn get_and_parse<T, P>(map: &HashMap<&str, &str>, field_name: &str, parser: P) -> Result<T, String>
    where
        P: FnOnce(&str) -> Result<T, ParseIntError>
    {
        parser(map.get(field_name).ok_or(format!("numeric field '{}' not found", field_name))?)
                .map_err(|e| format!("could not parse numeric field '{}': {}", field_name, e))
    }

    fn get(map: &HashMap<&str, &str>, field_name: &str) -> Result<String, String> {
        map.get(field_name)
            .map(|s| s.to_string())
            .ok_or(format!("field '{}' not found", field_name))
    }

}

#[cfg(test)]
mod tests {
    use crate::stats::Stats;

    #[test]
    fn parse_good() {
        assert_eq!(
            Stats::from_readable("\
                in_len=12345\n\
                in_hash=abcde\n\
                hash_seed=edcba\n\
                xz_len=54321\n\
                nr_chunks=1\n\
                chunk_len=2\n\
                alg=aes128-gcm\n\
                auth=Author Name\n\
                auth_len=3\n
                misc_info=ABC=1, XYZ=2".as_bytes().to_vec().as_slice()).unwrap(),
            Stats {
                in_data_len: 12345,
                in_data_hash: 0xabcde,
                hash_seed: 0xedcba,
                compressed_len: 54321,
                out_nr_chunks: 1,
                out_chunk_size: 2,
                alg: "aes128-gcm".to_owned(),
                auth_string: "Author Name".to_owned(),
                auth_chunk_size: 3,
                misc_info: Some("ABC=1, XYZ=2".to_owned())
            }
        );
    }

    #[test]
    fn parse_bad() {
        // duplicate key
        assert!(
            Stats::from_readable("\
                in_len=12345\n\
                in_hash=abcde\n\
                in_len=edcba\n\
                ".as_bytes().to_vec().as_slice()).is_err());

        // empty key
        assert!(
            Stats::from_readable("\
                in_len=12345\n\
                =abcde\n\
                in_len=edcba\n\
                ".as_bytes().to_vec().as_slice()).is_err());

        // empty value
        assert!(
            Stats::from_readable("\
                in_len=12345\n\
                nr_chunks=\n\
                auth_len=edcba\n\
                ".as_bytes().to_vec().as_slice()).is_err());

        // invalid number
        assert!(
            Stats::from_readable("\
            in_len=12345\n\
            in_hash=abcde\n\
            hash_seed=xyz\n\
            xz_len=54321\n\
            nr_chunks=1\n\
            chunk_len=2\n\
            auth=Author Name\n\
            auth_len=3\n
            misc_info=XXX".as_bytes().to_vec().as_slice()).is_err());
    }
}
