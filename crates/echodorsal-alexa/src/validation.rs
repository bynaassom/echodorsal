//! Fail-closed Alexa authentication, before behavior or external project access.
use crate::RequestEnvelope;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use reqwest::{Client, Url};
use rustls_pki_types::{CertificateDer, TrustAnchor, UnixTime, pem::PemObject};
use serde::Deserialize;
use std::{
    collections::{HashMap, HashSet},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use tokio::sync::{Mutex, Semaphore};
use webpki::EndEntityCert;

pub const MAX_BODY_BYTES: usize = 256 * 1024;
const MAX_CHAIN_BYTES: usize = 64 * 1024;
const CACHE_TTL: Duration = Duration::from_secs(3600);
const MAX_CACHE_ENTRIES: usize = 16;

/// No request content, identifiers, URLs or crypto details are exposed in errors.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ValidationError {
    #[error("invalid Alexa request")]
    Invalid,
    #[error("Alexa caller not allowed")]
    Unauthorized,
}

/// Independent nonempty allowlists. Configuration has no insecure default.
#[derive(Deserialize)]
pub struct Allowlist {
    pub application_ids: HashSet<String>,
    pub user_ids: HashSet<String>,
}
impl Allowlist {
    /// # Errors
    /// Rejects empty lists and blank identifiers.
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.application_ids.is_empty()
            || self.user_ids.is_empty()
            || self
                .application_ids
                .iter()
                .chain(&self.user_ids)
                .any(|id| id.trim().is_empty())
        {
            return Err(ValidationError::Unauthorized);
        }
        Ok(())
    }
}

struct CachedChain {
    certificates: Vec<CertificateDer<'static>>,
    fetched_at: SystemTime,
}

/// Root trust is fixed to Mozilla's compiled roots, never to the downloaded chain.
pub struct Validator {
    allowlist: Allowlist,
    client: Client,
    cache: Mutex<HashMap<String, CachedChain>>,
    fetch_permits: Semaphore,
    roots: Vec<TrustAnchor<'static>>,
}

impl Validator {
    /// # Errors
    /// Fails on invalid configuration or TLS client initialization.
    pub fn new(allowlist: Allowlist) -> Result<Self, ValidationError> {
        allowlist.validate()?;
        let client = Client::builder()
            .https_only(true)
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .connect_timeout(Duration::from_secs(2))
            .timeout(Duration::from_secs(3))
            .build()
            .map_err(|_| ValidationError::Invalid)?;
        Ok(Self {
            allowlist,
            client,
            cache: Mutex::new(HashMap::new()),
            fetch_permits: Semaphore::new(4),
            roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
        })
    }

    /// Verifies the signature over the exact bytes before deserializing behavior.
    /// # Errors
    /// Invalid authentication/freshness yields `Invalid`; denied caller yields `Unauthorized`.
    pub async fn validate(
        &self,
        raw_body: &[u8],
        cert_url: &str,
        signature: &str,
    ) -> Result<RequestEnvelope, ValidationError> {
        if raw_body.len() > MAX_BODY_BYTES || signature.len() > 2048 {
            return Err(ValidationError::Invalid);
        }
        let url = certificate_url(cert_url)?;
        let signature = STANDARD
            .decode(signature)
            .map_err(|_| ValidationError::Invalid)?;
        if signature.is_empty() {
            return Err(ValidationError::Invalid);
        }
        // Never hold the shared cache lock across untrusted network I/O.
        let cached = {
            let mut cache = self.cache.lock().await;
            let now = SystemTime::now();
            cache.retain(|_, chain| {
                now.duration_since(chain.fetched_at)
                    .is_ok_and(|age| age < CACHE_TTL)
            });
            cache
                .get(url.as_str())
                .map(|chain| chain.certificates.clone())
        };
        let certificates = if let Some(certificates) = cached {
            certificates
        } else {
            let _permit =
                tokio::time::timeout(Duration::from_secs(4), self.fetch_permits.acquire())
                    .await
                    .map_err(|_| ValidationError::Invalid)?
                    .map_err(|_| ValidationError::Invalid)?;
            // Another fetch may have filled this entry while a permit was acquired.
            let cached = {
                let cache = self.cache.lock().await;
                cache
                    .get(url.as_str())
                    .map(|chain| chain.certificates.clone())
            };
            if let Some(certificates) = cached {
                certificates
            } else {
                let certificates = self.fetch_chain(url.clone()).await?;
                verify_chain(&certificates, &self.roots, SystemTime::now())?;
                let mut cache = self.cache.lock().await;
                if cache.len() >= MAX_CACHE_ENTRIES {
                    cache.clear();
                }
                cache.insert(
                    url.to_string(),
                    CachedChain {
                        certificates: certificates.clone(),
                        fetched_at: SystemTime::now(),
                    },
                );
                certificates
            }
        };
        let now = SystemTime::now();
        // Dates and path constraints are checked again on EVERY cache hit.
        let certificate = verify_chain(&certificates, &self.roots, now)?;
        certificate
            .verify_signature(
                webpki::ring::RSA_PKCS1_2048_8192_SHA256,
                raw_body,
                &signature,
            )
            .map_err(|_| ValidationError::Invalid)?;
        self.validate_envelope(raw_body, now)
    }

    async fn fetch_chain(&self, url: Url) -> Result<Vec<CertificateDer<'static>>, ValidationError> {
        let mut response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|_| ValidationError::Invalid)?;
        if response.status() != reqwest::StatusCode::OK
            || response
                .content_length()
                .is_some_and(|size| size > MAX_CHAIN_BYTES as u64)
        {
            return Err(ValidationError::Invalid);
        }
        let mut pem = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| ValidationError::Invalid)?
        {
            if chunk.len() > MAX_CHAIN_BYTES.saturating_sub(pem.len()) {
                return Err(ValidationError::Invalid);
            }
            pem.extend_from_slice(&chunk);
        }
        let chain: Vec<_> = CertificateDer::pem_slice_iter(&pem)
            .collect::<Result<_, _>>()
            .map_err(|_| ValidationError::Invalid)?;
        if chain.is_empty() || chain.len() > 8 {
            return Err(ValidationError::Invalid);
        }
        Ok(chain)
    }

    fn validate_envelope(
        &self,
        raw_body: &[u8],
        now: SystemTime,
    ) -> Result<RequestEnvelope, ValidationError> {
        let envelope: SecurityEnvelope =
            serde_json::from_slice(raw_body).map_err(|_| ValidationError::Invalid)?;
        let timestamp = OffsetDateTime::parse(&envelope.request.timestamp, &Rfc3339)
            .map_err(|_| ValidationError::Invalid)?;
        let elapsed = now
            .duration_since(UNIX_EPOCH)
            .map_err(|_| ValidationError::Invalid)?;
        let current =
            i128::from(elapsed.as_secs()) * 1_000_000_000 + i128::from(elapsed.subsec_nanos());
        if (current - timestamp.unix_timestamp_nanos()).abs() > 150_000_000_000 {
            return Err(ValidationError::Invalid);
        }
        let system = envelope.context.system;
        let application = system.application.and_then(|app| app.application_id);
        let user = system.user.and_then(|user| user.user_id);
        if !application
            .as_ref()
            .is_some_and(|id| self.allowlist.application_ids.contains(id))
            || !user
                .as_ref()
                .is_some_and(|id| self.allowlist.user_ids.contains(id))
        {
            return Err(ValidationError::Unauthorized);
        }
        serde_json::from_slice(raw_body).map_err(|_| ValidationError::Invalid)
    }
}

fn verify_chain<'a>(
    chain: &'a [CertificateDer<'static>],
    roots: &[TrustAnchor<'static>],
    now: SystemTime,
) -> Result<EndEntityCert<'a>, ValidationError> {
    let leaf = chain.first().ok_or(ValidationError::Invalid)?;
    let cert = EndEntityCert::try_from(leaf).map_err(|_| ValidationError::Invalid)?;
    if !cert
        .valid_dns_names()
        .any(|name| name == "echo-api.amazon.com")
    {
        return Err(ValidationError::Invalid);
    }
    let now = UnixTime::since_unix_epoch(
        now.duration_since(UNIX_EPOCH)
            .map_err(|_| ValidationError::Invalid)?,
    );
    cert.verify_for_usage(
        webpki::ALL_VERIFICATION_ALGS,
        roots,
        &chain[1..],
        now,
        webpki::KeyUsage::server_auth(),
        None,
        None,
    )
    .map_err(|_| ValidationError::Invalid)?;
    Ok(cert)
}

/// Normalize first, then restrict to the Amazon certificate directory.
fn certificate_url(input: &str) -> Result<Url, ValidationError> {
    // Reject ambiguous WHATWG URL forms, encodings, credentials, and query strings.
    if input.len() > 2048
        || !input.is_ascii()
        || input.bytes().any(|b| b.is_ascii_control() || b == b' ')
        || input.contains(['\\', '%'])
    {
        return Err(ValidationError::Invalid);
    }
    let mut url = Url::parse(input).map_err(|_| ValidationError::Invalid)?;
    if url.scheme() != "https"
        || url.host_str() != Some("s3.amazonaws.com")
        || url.port_or_known_default() != Some(443)
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
    {
        return Err(ValidationError::Invalid);
    }
    let path = format!(
        "/{}",
        url.path()
            .split('/')
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("/")
    );
    if !path.starts_with("/echo.api/") {
        return Err(ValidationError::Invalid);
    }
    url.set_path(&path);
    url.set_fragment(None);
    Ok(url)
}

#[derive(Deserialize)]
struct SecurityEnvelope {
    request: Timestamp,
    context: Context,
}
#[derive(Deserialize)]
struct Timestamp {
    timestamp: String,
}
#[derive(Deserialize)]
struct Context {
    #[serde(rename = "System")]
    system: System,
}
#[derive(Deserialize)]
struct System {
    application: Option<Application>,
    user: Option<User>,
}
#[derive(Deserialize)]
struct Application {
    #[serde(rename = "applicationId")]
    application_id: Option<String>,
}
#[derive(Deserialize)]
struct User {
    #[serde(rename = "userId")]
    user_id: Option<String>,
}

#[cfg(test)]
mod tests;
