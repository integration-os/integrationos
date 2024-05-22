pub struct HashedSecret(String);

impl HashedSecret {
    pub fn new(secret: String) -> Self {
        Self(secret)
    }

    pub fn inner(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}
