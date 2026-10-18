//! Credentials never implement Serialize and never appear in Debug/errors.
use zeroize::Zeroizing;

const SERVICE: &str = "io.yuri.BinanceDesk";
const USER: &str = "binance-usdm";

#[derive(Clone)]
pub struct Credentials {
    pub key: Zeroizing<String>,
    pub secret: Zeroizing<String>,
}
impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Credentials(<redacted>)")
    }
}
impl Credentials {
    pub fn new(key: String, secret: String) -> Result<Self, VaultError> {
        let credentials = Self {
            key: Zeroizing::new(key),
            secret: Zeroizing::new(secret),
        };
        let valid = |text: &str| {
            (16..=512).contains(&text.len())
                && text
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
        };
        if !valid(&credentials.key) || !valid(&credentials.secret) {
            return Err(VaultError::Invalid);
        }
        Ok(credentials)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VaultError {
    Missing,
    Unavailable,
    Invalid,
}
impl VaultError {
    pub fn message(self) -> &'static str {
        match self {
            Self::Missing => "Belum ada API key tersimpan. Masukkan key dan secret di aplikasi.",
            Self::Unavailable => {
                "KWallet belum tersedia atau terkunci. Buka KWallet, lalu coba lagi."
            }
            Self::Invalid => {
                "Format key/secret tidak valid. Gunakan pasangan API HMAC Binance tanpa spasi."
            }
        }
    }
}

fn entry() -> Result<keyring::Entry, VaultError> {
    keyring::Entry::new(SERVICE, USER).map_err(|_| VaultError::Unavailable)
}
pub fn load() -> Result<Credentials, VaultError> {
    let text = Zeroizing::new(entry()?.get_password().map_err(|error| match error {
        keyring::Error::NoEntry => VaultError::Missing,
        _ => VaultError::Unavailable,
    })?);
    let (key, secret) = text.split_once('\n').ok_or(VaultError::Invalid)?;
    Credentials::new(key.to_owned(), secret.to_owned())
}
pub fn save(credentials: &Credentials) -> Result<(), VaultError> {
    let value = Zeroizing::new(format!("{}\n{}", *credentials.key, *credentials.secret));
    entry()?
        .set_password(&value)
        .map_err(|_| VaultError::Unavailable)
}
pub fn delete() -> Result<(), VaultError> {
    match entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(_) => Err(VaultError::Unavailable),
    }
}

/// Checks only this app's credential; never prints the secret or account values.
pub fn check() -> &'static str {
    match load() {
        Ok(_) => "KWALLET_CREDENTIAL_READY",
        Err(VaultError::Missing) => "KWALLET_CREDENTIAL_MISSING",
        Err(VaultError::Invalid) => "KWALLET_CREDENTIAL_INVALID",
        Err(VaultError::Unavailable) => "KWALLET_UNAVAILABLE",
    }
}

/// Roundtrip in a disposable, dedicated local keyring entry; no Binance request.
pub fn self_test() -> anyhow::Result<()> {
    let service = format!("{SERVICE}.selftest.{}", std::process::id());
    let test = keyring::Entry::new(&service, "test-only")
        .map_err(|_| anyhow::anyhow!("Keyring test init failed"))?;
    let result = (|| {
        test.set_password("local-test-not-a-binance-credential")
            .map_err(|_| anyhow::anyhow!("Keyring test write failed"))?;
        let read = Zeroizing::new(
            test.get_password()
                .map_err(|_| anyhow::anyhow!("Keyring test read failed"))?,
        );
        anyhow::ensure!(
            &*read == "local-test-not-a-binance-credential",
            "Keyring roundtrip mismatch"
        );
        Ok(())
    })();
    let deleted = test
        .delete_credential()
        .map_err(|_| anyhow::anyhow!("Keyring test cleanup failed"));
    result?;
    deleted?;
    anyhow::ensure!(
        matches!(test.get_password(), Err(keyring::Error::NoEntry)),
        "Keyring test item still exists"
    );
    println!("KWALLET_ROUNDTRIP_AND_CLEANUP_PASS");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn credentials_and_errors_never_format_secrets() {
        let credentials =
            Credentials::new("dummy_public_key_123".into(), "dummy_secret_123456".into()).unwrap();
        assert_eq!(format!("{credentials:?}"), "Credentials(<redacted>)");
        assert!(Credentials::new("invalid\nheader".into(), "dummy_secret_123456".into()).is_err());
        for error in [
            VaultError::Missing,
            VaultError::Invalid,
            VaultError::Unavailable,
        ] {
            assert!(!error.message().contains(&*credentials.secret));
        }
    }
}
