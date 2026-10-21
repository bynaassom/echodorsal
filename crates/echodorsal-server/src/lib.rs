//! Local-only Alexa endpoint. Authentication precedes deterministic behavior.
use axum::{
    Json, Router,
    body::{Body, to_bytes},
    extract::{Request, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
};
use echodorsal_alexa::{
    hello_response,
    validation::{MAX_BODY_BYTES, ValidationError, Validator},
};
use std::{sync::Arc, time::Duration};

pub fn router(validator: Arc<Validator>) -> Router {
    Router::new()
        .route("/alexa", post(alexa))
        .with_state(validator)
}

async fn alexa(State(validator): State<Arc<Validator>>, request: Request) -> Response {
    match tokio::time::timeout(Duration::from_secs(5), authenticate(validator, request)).await {
        Ok(Ok(reply)) => Json(reply).into_response(),
        Ok(Err(ValidationError::Unauthorized)) => empty(StatusCode::UNAUTHORIZED),
        Ok(Err(ValidationError::Invalid)) | Err(_) => empty(StatusCode::BAD_REQUEST),
    }
}

async fn authenticate(
    validator: Arc<Validator>,
    request: Request,
) -> Result<echodorsal_alexa::ResponseEnvelope, ValidationError> {
    let (parts, body) = request.into_parts();
    let unique_header = |name: &str| {
        let mut values = parts.headers.get_all(name).iter();
        let value = values.next().ok_or(ValidationError::Invalid)?;
        if values.next().is_some() {
            return Err(ValidationError::Invalid);
        }
        value.to_str().map_err(|_| ValidationError::Invalid)
    };
    let cert_url = unique_header("signaturecertchainurl")?;
    let signature = unique_header("signature-256")?;
    let raw_body = to_bytes(body, MAX_BODY_BYTES)
        .await
        .map_err(|_| ValidationError::Invalid)?;
    let envelope = validator.validate(&raw_body, cert_url, signature).await?;
    Ok(hello_response(&envelope.request))
}

fn empty(status: StatusCode) -> Response {
    Response::builder()
        .status(status)
        .body(Body::empty())
        .unwrap_or_else(|_| Response::new(Body::empty()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{Request, header::HeaderValue};
    use echodorsal_alexa::validation::Allowlist;
    use tower::ServiceExt as _;

    #[tokio::test]
    async fn malformed_requests_return_empty_400() {
        let validator = Arc::new(
            Validator::new(Allowlist {
                application_ids: ["test-app".into()].into(),
                user_ids: ["test-user".into()].into(),
            })
            .unwrap(),
        );
        for duplicate in [false, true] {
            let mut request = Request::post("/alexa").body(Body::from("{}")).unwrap();
            if duplicate {
                request
                    .headers_mut()
                    .append("signature-256", HeaderValue::from_static("AA=="));
                request
                    .headers_mut()
                    .append("signature-256", HeaderValue::from_static("AA=="));
                request.headers_mut().insert(
                    "signaturecertchainurl",
                    HeaderValue::from_static("http://localhost/private"),
                );
            }
            let response = router(validator.clone()).oneshot(request).await.unwrap();
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
            assert!(
                to_bytes(response.into_body(), 1024)
                    .await
                    .unwrap()
                    .is_empty()
            );
        }
    }
}
