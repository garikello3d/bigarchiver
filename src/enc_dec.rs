use ring::error::Unspecified;
use ring::aead::AES_128_GCM;
use ring::aead::UnboundKey;
use ring::aead::BoundKey;
use ring::aead::SealingKey;
use ring::aead::OpeningKey;
use ring::aead::Aad;
use ring::aead::NonceSequence;
use ring::aead::NONCE_LEN;
use ring::aead::Nonce;
use ring::pbkdf2;
use std::num::NonZeroU32;
use crate::finalizable::DataSink;

fn create_unbound_key(pass_str: &str) -> UnboundKey {
    let mut key: Vec<u8> = vec![0; AES_128_GCM.key_len()];
    let nr_iters = NonZeroU32::new(100000).unwrap();
    pbkdf2::derive(pbkdf2::PBKDF2_HMAC_SHA256, nr_iters, &[],pass_str.as_bytes(), &mut key);

    // SAFE because algorithm is hardcoded, it cannot come from a user
    UnboundKey::new(&AES_128_GCM, &key).unwrap()
}

pub struct Encryptor<'a, T: DataSink> {
    write_to: &'a mut T,
    sealing_key: SealingKey<NonceFromCounter>,
    assoc_data: Aad<String>
}

impl<'a, T: DataSink> Encryptor<'a, T> {
    pub fn new(to: &'a mut T, pass_str: &str, aad_str: &str) -> Encryptor<'a, T> {
        Encryptor { 
            write_to: to, 
            sealing_key: SealingKey::new( create_unbound_key(pass_str), NonceFromCounter{ cnt: 0 }),
            assoc_data: Aad::from(aad_str.to_owned())
        }
    }
}

impl<'a, T: DataSink> DataSink for Encryptor<'a, T> {
    fn add(&mut self, data: &[u8]) -> Result<(), String> {
        //eprintln!("Encryptor: writing {} bytes", data.len());
        let mut inout_buf = data.to_vec().clone();
        self.sealing_key
            .seal_in_place_append_tag(self.assoc_data.clone(), &mut inout_buf)
            .map_err(|e| format!("encrypt error: {}", e))?;
        self.write_to.add(inout_buf.as_slice())
    }

    fn finish(&mut self) -> Result<(), String> {
        //eprintln!("Encryptor: finish");
        self.write_to.finish()
    }
}

pub struct Decryptor<'a, T: DataSink> {
    write_to: &'a mut T,
    opening_key: OpeningKey<NonceFromCounter>,
    assoc_data: Aad<String>
}

impl<'a, T: DataSink> Decryptor<'a, T> {
    pub fn new(to: &'a mut T, pass_str: &str, aad_str: &str) -> Decryptor<'a, T> {
        Decryptor { 
            write_to: to, 
            opening_key: OpeningKey::new( create_unbound_key(pass_str), NonceFromCounter{ cnt: 0 }),
            assoc_data: Aad::from(aad_str.to_owned())
        }
    }
}

impl<'a, T: DataSink> DataSink for Decryptor<'a, T> {
    fn add(&mut self, data: &[u8]) -> Result<(), String> {
        //eprintln!("Decryptor: writing {} bytes", data.len());
        let mut inout_buf = data.to_vec().clone();
        let out_ref = self.opening_key
            .open_in_place(self.assoc_data.clone(), &mut inout_buf)
            .map_err(|e| format!("decrypt error: {}", e))?;
        self.write_to.add(out_ref)
    }

    fn finish(&mut self) -> Result<(), String> {
        //eprintln!("Decryptor: finish");
        self.write_to.finish()
    }
}


struct NonceFromCounter {
    cnt: u64
}

impl NonceSequence for NonceFromCounter {
    fn advance(&mut self) -> Result<Nonce, Unspecified> {
        let mut buf = [0u8; NONCE_LEN];
        let counter_bytes = self.cnt.to_be_bytes();
        buf[NONCE_LEN-counter_bytes.len()..].copy_from_slice(&counter_bytes);
        self.cnt += 1;
        Nonce::try_assume_unique_for_key(&buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct CipherReceiver(Vec<u8>);

    impl DataSink for CipherReceiver {
        fn add(&mut self, data: &[u8]) -> Result<(), String> {
            //eprintln!("encrypted data of size {}: {}", data.len(), as_hex(data));
            self.0.extend_from_slice(data);
            Ok(())
        }
        fn finish(&mut self) -> Result<(), String> {
            Ok(())
        }
    }

    struct PlaintextReceiver(String);

    impl DataSink for PlaintextReceiver {
        fn add(&mut self, data: &[u8]) -> Result<(), String> {
            //eprintln!("decrypted data of size {}: {}", data.len(), as_hex(data));
            let s = String::from_utf8(data.to_vec()).unwrap();
            //eprintln!("decrypted string of size {}: {}", data.len(), s);
            self.0.push_str(s.as_str());
            Ok(())
        }
        fn finish(&mut self) -> Result<(), String> {
            Ok(())
        }
    }

    fn _as_hex(data: &[u8]) -> String {
        let mut s = String::new();
        for d in data {
            s.push_str(format!("{d:02x} ").as_str());
        }
        s
    }

    #[test]
    fn encrypt_decrypt_once_good() {
        let mut cipher = CipherReceiver(Vec::new());
        let mut enc = Encryptor::<CipherReceiver>::new(&mut cipher, "password", "data11111111");

        let mut text = PlaintextReceiver(String::new());
        let mut dec = Decryptor::<PlaintextReceiver>::new(&mut text, "password", "data11111111");

        enc.add(b"AAAAAAAAAA").unwrap();
        //enc.write(b"BBB").unwrap();
        dec.add(&cipher.0).unwrap();
    }


}
