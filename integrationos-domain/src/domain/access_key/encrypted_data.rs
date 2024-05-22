use crate::{IntegrationOSError, InternalError};
use aes::cipher::{generic_array::GenericArray, KeyIvInit, StreamCipher};
use sha2::{Digest, Sha256};

type Aes256Ctr = ctr::Ctr64BE<aes::Aes256>;
const HASH_LENGTH: usize = 32;
pub const IV_LENGTH: usize = 16;
pub const PASSWORD_LENGTH: usize = 32;
type HashBuf = [u8; HASH_LENGTH];

const HASH_PREFIX: &str = "\x19Event Signed Message:\n";

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct EncryptedData {
    data: Vec<u8>,
}

impl EncryptedData {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }

    pub fn verify_and_decrypt(
        &mut self,
        password: &[u8; PASSWORD_LENGTH],
    ) -> Result<&[u8], IntegrationOSError> {
        if !self.verify_hash(password)? {
            return Err(InternalError::invalid_argument("Hash does not match", None));
        }

        self.decrypt(password)
    }

    pub fn encrypt(
        mut content: Vec<u8>,
        iv: &[u8; IV_LENGTH],
        password: &[u8; PASSWORD_LENGTH],
    ) -> Result<Vec<u8>, IntegrationOSError> {
        let mut cipher = Aes256Ctr::new(password.into(), iv.into());
        cipher
            .try_apply_keystream(&mut content)
            .map_err(|e| InternalError::io_err(&format!("Could not encode content: {e}"), None))?;
        let hash = Self::compute_hash(&content, iv, password)?;
        content.extend(iv);
        content.extend(hash);
        Ok(content)
    }

    fn decrypt(&mut self, password: &[u8; PASSWORD_LENGTH]) -> Result<&[u8], IntegrationOSError> {
        let mut cipher = Aes256Ctr::new(password.into(), self.get_iv().into());
        let content = self.get_content_mut();
        cipher
            .try_apply_keystream(content)
            .map_err(|e| InternalError::io_err(&format!("Could not decode content: {e}"), None))?;
        Ok(content)
    }

    fn verify_hash(&self, password: &[u8; PASSWORD_LENGTH]) -> Result<bool, IntegrationOSError> {
        let actual_hash = Self::compute_hash(self.get_content(), self.get_iv(), password)?;
        Ok(actual_hash == *self.get_hash())
    }

    pub fn compute_hash(
        content: &[u8],
        iv: &[u8],
        password: &[u8],
    ) -> Result<HashBuf, IntegrationOSError> {
        let message_len = content.len() + iv.len() + password.len();
        let mut hasher = Sha256::new();
        hasher.update(HASH_PREFIX);
        hasher.update(message_len.to_string());
        hasher.update(content);
        hasher.update(iv);
        hasher.update(password);
        let mut actual_hash = [0u8; HASH_LENGTH];
        hasher.finalize_into(GenericArray::from_mut_slice(&mut actual_hash));
        Ok(actual_hash)
    }

    fn get_iv(&self) -> &[u8] {
        &self.data[self.data.len() - HASH_LENGTH - IV_LENGTH..self.data.len() - HASH_LENGTH]
    }

    fn get_hash(&self) -> &[u8] {
        &self.data[self.data.len() - HASH_LENGTH..]
    }

    fn get_content_mut(&mut self) -> &mut [u8] {
        let len = self.data.len();
        &mut self.data[..len - HASH_LENGTH - IV_LENGTH]
    }

    fn get_content(&self) -> &[u8] {
        let len = self.data.len();
        &self.data[..len - HASH_LENGTH - IV_LENGTH]
    }
}

#[cfg(test)]
mod test {

    use base64ct::{Base64UrlUnpadded, Encoding};

    use super::*;

    const ENCRYPTED_DATA: &str = "Q71YUIZydcgSwJQNOUCHhaTMqmIvslIafF5LluORJfJKydMGELHtYe_ydtBIrVuomEvIMurKaAUqlujQ8xzs4LBOxyf_lJ2unwqyFzk1TnCKBMNyRJybyL9RTBp90BExEwf2WwMtU4FDBUhP2bhWmxm7eQAAAAAAAAAAAAAAAAAAAADPLKD188CZczQr7eWGtyipuCZLZKQ2lBKL3S_R-nEgBA";
    const INVALID_ENCRYPTED_DATA: &str = "h72AgcpeanqdZgXkeblx4zAygpn1r9Kx5wZDvH67K5QC8pGYCwxUvSsGiIWkAIswPUA6OPOEtGNPB4Je9zpppnnYX8YmbhghsBt7ClICnaOcs2a5i0IAzaUphhYkoz1r8BWLSCi9m0gpEw7tH1JhxK-k5My-7TgjfA";

    const PASSWORD: &[u8; PASSWORD_LENGTH] = b"32KFFT_i4UpkJmyPwY2TGzgHpxfXs7zS";
    const DECRYPTED_DATA: &str = "\n&build-2e76c839f5fd419db6b34682f4cdff1e\u{12}\u{7}default\u{18}\u{1}\"\u{7}webhook*\nmy-webhook2\u{e}event.received:\u{7}foo.barB\u{7}foo.barJ\u{7}foo.bar";

    #[test]
    fn test_verify_and_decrypt() {
        let data = Base64UrlUnpadded::decode_vec(ENCRYPTED_DATA).unwrap();
        let mut data: EncryptedData = EncryptedData::new(data);
        let decrypted = data.verify_and_decrypt(PASSWORD).unwrap();
        assert_eq!(decrypted, DECRYPTED_DATA.as_bytes());
    }

    #[test]
    fn test_verify_hash() {
        let data = Base64UrlUnpadded::decode_vec(ENCRYPTED_DATA).unwrap();
        let data: EncryptedData = EncryptedData::new(data);
        assert!(data.verify_hash(PASSWORD).unwrap());
    }

    #[test]
    fn test_incorrect_hash() {
        let data = Base64UrlUnpadded::decode_vec(INVALID_ENCRYPTED_DATA).unwrap();
        let data: EncryptedData = EncryptedData::new(data);
        assert!(!data.verify_hash(PASSWORD).unwrap());
    }

    #[test]
    fn test_decrypt() {
        let data = Base64UrlUnpadded::decode_vec(ENCRYPTED_DATA).unwrap();
        let mut data: EncryptedData = EncryptedData::new(data);
        let decrypted = data.decrypt(PASSWORD).unwrap();
        assert_eq!(decrypted, DECRYPTED_DATA.as_bytes());
    }

    #[test]
    fn test_incorrect_decrypt() {
        let data = Base64UrlUnpadded::decode_vec(INVALID_ENCRYPTED_DATA).unwrap();
        let mut data: EncryptedData = EncryptedData::new(data);
        let decrypted = data.decrypt(PASSWORD).unwrap();
        let res = std::str::from_utf8(decrypted);
        assert!(res.is_err());
    }
}
