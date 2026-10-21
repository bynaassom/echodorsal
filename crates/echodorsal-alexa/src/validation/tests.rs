use super::*;
use rcgen::{
    BasicConstraints, CertificateParams, ExtendedKeyUsagePurpose, IsCa, Issuer, KeyPair,
    KeyUsagePurpose, PKCS_RSA_SHA256, SigningKey,
};

const URL: &str = "https://s3.amazonaws.com/echo.api/test.pem";

fn allowlist() -> Allowlist {
    Allowlist {
        application_ids: ["test-app".into()].into(),
        user_ids: ["test-user".into()].into(),
    }
}
fn body(timestamp: OffsetDateTime, app: &str, user: &str) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "request": { "type": "LaunchRequest", "timestamp": timestamp.format(&Rfc3339).unwrap() },
        "context": { "System": { "application": {"applicationId": app}, "user": {"userId": user} } }
    }))
    .unwrap()
}
struct TestChain {
    root: CertificateDer<'static>,
    chain: Vec<CertificateDer<'static>>,
    key: KeyPair,
}
fn chain(san: &str, expired: bool, intermediate_ca: bool) -> TestChain {
    let root_key = KeyPair::generate().unwrap();
    let mut root = CertificateParams::new(Vec::<String>::new()).unwrap();
    root.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    root.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
    let root_cert = root.self_signed(&root_key).unwrap();
    let intermediate_key = KeyPair::generate().unwrap();
    let mut intermediate = CertificateParams::new(Vec::<String>::new()).unwrap();
    intermediate
        .distinguished_name
        .push(rcgen::DnType::CommonName, "Test intermediate");
    if intermediate_ca {
        intermediate.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        intermediate.key_usages = vec![KeyUsagePurpose::KeyCertSign];
    }
    let intermediate_cert = intermediate
        .signed_by(&intermediate_key, &Issuer::from_params(&root, &root_key))
        .unwrap();
    let key = KeyPair::generate_for(&PKCS_RSA_SHA256).unwrap();
    let mut leaf = CertificateParams::new(vec![san.into()]).unwrap();
    leaf.key_usages = vec![KeyUsagePurpose::DigitalSignature];
    leaf.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
    if expired {
        leaf.not_before = rcgen::date_time_ymd(2000, 1, 1);
        leaf.not_after = rcgen::date_time_ymd(2001, 1, 1);
    }
    let leaf = leaf
        .signed_by(&key, &Issuer::from_params(&intermediate, &intermediate_key))
        .unwrap();
    TestChain {
        root: root_cert.der().clone(),
        chain: vec![
            leaf.der().clone(),
            intermediate_cert.der().clone(),
            root_cert.der().clone(),
        ],
        key,
    }
}
async fn validator(chain: &TestChain) -> Validator {
    let mut validator = Validator::new(allowlist()).unwrap();
    validator.roots = vec![
        webpki::anchor_from_trusted_cert(&chain.root)
            .unwrap()
            .to_owned(),
    ];
    // Offline tests seed only an already-verified chain; production has no injection API.
    verify_chain(&chain.chain, &validator.roots, SystemTime::now()).unwrap();
    validator.cache.lock().await.insert(
        URL.into(),
        CachedChain {
            certificates: chain.chain.clone(),
            fetched_at: SystemTime::now(),
        },
    );
    validator
}

#[test]
fn strict_url_normalization() {
    for input in [
        URL,
        "HTTPS://S3.AMAZONAWS.COM:443/echo.api/../echo.api/test.pem",
        "https://s3.amazonaws.com//echo.api//test.pem#fragment",
    ] {
        assert_eq!(certificate_url(input).unwrap().as_str(), URL);
    }
    for input in [
        "http://s3.amazonaws.com/echo.api/x",
        "https://s3.amazonaws.com:444/echo.api/x",
        "https://s3.amazonaws.com.evil.test/echo.api/x",
        "https://s3.amazonaws.com@evil.test/echo.api/x",
        "https://user@s3.amazonaws.com/echo.api/x",
        "https://s3.amazonaws.com/ECHO.API/x",
        "https://s3.amazonaws.com/echo.api/../evil",
        "https://s3.amazonaws.com/echo.api/%2e%2e/evil",
        "https://s3.amazonaws.com/echo.api/x?other",
        "https://s3.amazonaws.com/echo.api\\x",
        "https://s3.amazonaws.com/echo.api/x\n",
        "https://s3.amazonaws.com./echo.api/x",
    ] {
        assert!(certificate_url(input).is_err(), "accepted {input}");
    }
}

#[tokio::test]
async fn authenticated_body_and_tampering() {
    let chain = chain("echo-api.amazon.com", false, true);
    let validator = validator(&chain).await;
    let body = body(OffsetDateTime::now_utc(), "test-app", "test-user");
    let signature = STANDARD.encode(chain.key.sign(&body).unwrap());
    assert!(validator.validate(&body, URL, &signature).await.is_ok());
    let mut tampered = body.clone();
    tampered.push(b' '); // Same JSON semantics, different signed bytes.
    assert_eq!(
        validator
            .validate(&tampered, URL, &signature)
            .await
            .unwrap_err(),
        ValidationError::Invalid
    );
    assert_eq!(
        validator
            .validate(&body, URL, "not-base64")
            .await
            .unwrap_err(),
        ValidationError::Invalid
    );
    assert_eq!(
        validator.validate(&body, URL, "").await.unwrap_err(),
        ValidationError::Invalid
    );
    let other_key = KeyPair::generate_for(&PKCS_RSA_SHA256).unwrap();
    let other_signature = STANDARD.encode(other_key.sign(&body).unwrap());
    assert_eq!(
        validator
            .validate(&body, URL, &other_signature)
            .await
            .unwrap_err(),
        ValidationError::Invalid
    );
}

#[test]
fn chain_requires_independent_root_exact_san_dates_and_ca_constraints() {
    let good = chain("echo-api.amazon.com", false, true);
    let roots = vec![
        webpki::anchor_from_trusted_cert(&good.root)
            .unwrap()
            .to_owned(),
    ];
    assert!(verify_chain(&good.chain, &roots, SystemTime::now()).is_ok());
    assert!(
        verify_chain(
            &good.chain,
            webpki_roots::TLS_SERVER_ROOTS,
            SystemTime::now()
        )
        .is_err()
    );
    assert!(verify_chain(&good.chain[..1], &roots, SystemTime::now()).is_err());
    for (san, expired, ca) in [
        ("*.amazon.com", false, true),
        ("other.example", false, true),
        ("echo-api.amazon.com", true, true),
        ("echo-api.amazon.com", false, false),
    ] {
        let bad = chain(san, expired, ca);
        let root = [webpki::anchor_from_trusted_cert(&bad.root)
            .unwrap()
            .to_owned()];
        assert!(verify_chain(&bad.chain, &root, SystemTime::now()).is_err());
    }
    let future = UNIX_EPOCH + Duration::from_secs(100_000_000_000);
    assert!(verify_chain(&good.chain, &roots, future).is_err());
}

#[tokio::test]
async fn signed_timestamp_and_caller_fail_closed() {
    let chain = chain("echo-api.amazon.com", false, true);
    let validator = validator(&chain).await;
    let now = OffsetDateTime::now_utc();
    for (offset, app, user, expected) in [
        (-151, "test-app", "test-user", ValidationError::Invalid),
        (151, "test-app", "test-user", ValidationError::Invalid),
        (0, "other-app", "test-user", ValidationError::Unauthorized),
        (0, "test-app", "other-user", ValidationError::Unauthorized),
        (0, "", "test-user", ValidationError::Unauthorized),
    ] {
        let body = body(now + time::Duration::seconds(offset), app, user);
        let signature = STANDARD.encode(chain.key.sign(&body).unwrap());
        assert_eq!(
            validator
                .validate(&body, URL, &signature)
                .await
                .unwrap_err(),
            expected
        );
    }
    for offset in [-150, 150] {
        let now_system = UNIX_EPOCH + Duration::from_secs(1_800_000_000);
        let now = OffsetDateTime::from_unix_timestamp(1_800_000_000).unwrap();
        let body = body(
            now + time::Duration::seconds(offset),
            "test-app",
            "test-user",
        );
        assert!(validator.validate_envelope(&body, now_system).is_ok());
    }
    let malformed = b"{\"request\":{\"type\":\"LaunchRequest\",\"timestamp\":\"invalid\"},\"context\":{\"System\":{}}}";
    let signature = STANDARD.encode(chain.key.sign(malformed).unwrap());
    assert_eq!(
        validator
            .validate(malformed, URL, &signature)
            .await
            .unwrap_err(),
        ValidationError::Invalid
    );
}

#[tokio::test]
async fn cached_certificates_are_revalidated() {
    let chain = chain("echo-api.amazon.com", false, true);
    let mut validator = validator(&chain).await;
    validator.roots.clear(); // Cached chain cannot survive a failed trust check.
    let body = body(OffsetDateTime::now_utc(), "test-app", "test-user");
    let signature = STANDARD.encode(chain.key.sign(&body).unwrap());
    assert_eq!(
        validator
            .validate(&body, URL, &signature)
            .await
            .unwrap_err(),
        ValidationError::Invalid
    );
}

#[test]
fn empty_or_blank_allowlists_are_rejected() {
    assert!(
        Validator::new(Allowlist {
            application_ids: HashSet::new(),
            user_ids: HashSet::new()
        })
        .is_err()
    );
    assert!(
        Validator::new(Allowlist {
            application_ids: [" ".into()].into(),
            user_ids: ["test-user".into()].into()
        })
        .is_err()
    );
}

#[tokio::test]
async fn waiting_fetch_reuses_cache_without_blocking_cache_hits() {
    const UNCACHED_URL: &str = "https://s3.amazonaws.com/echo.api/uncached.pem";
    let chain = chain("echo-api.amazon.com", false, true);
    let validator = std::sync::Arc::new(validator(&chain).await);
    let body = body(OffsetDateTime::now_utc(), "test-app", "test-user");
    let signature = STANDARD.encode(chain.key.sign(&body).unwrap());

    let permits = validator.fetch_permits.acquire_many(4).await.unwrap();

    assert!(validator.validate(&body, URL, &signature).await.is_ok());

    let waiting_validator = std::sync::Arc::clone(&validator);
    let waiting_body = body.clone();
    let waiting_signature = signature.clone();
    let waiting = tokio::spawn(async move {
        waiting_validator
            .validate(&waiting_body, UNCACHED_URL, &waiting_signature)
            .await
    });
    tokio::task::yield_now().await;
    assert!(!waiting.is_finished());
    validator.cache.lock().await.insert(
        UNCACHED_URL.into(),
        CachedChain {
            certificates: chain.chain.clone(),
            fetched_at: SystemTime::now(),
        },
    );
    drop(permits);
    assert!(waiting.await.unwrap().is_ok());
}
