//! Alexa models, deterministic hello behavior and request authentication.
//!
//! Deserialization is not authentication: use `validation::Validator` before behavior.

pub mod validation;

use serde::{Deserialize, Serialize};

/// Minimal behavior input, not the full security envelope required by F0-03.
/// Unknown envelope fields are deliberately ignored and are not validated here.
#[derive(Debug, Deserialize)]
pub struct RequestEnvelope {
    pub request: Request,
}

/// Request kinds understood by the hello spike.
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum Request {
    LaunchRequest,
    IntentRequest {
        intent: Intent,
    },
    SessionEndedRequest,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Deserialize)]
pub struct Intent {
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct ResponseEnvelope {
    version: &'static str,
    response: Response,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct Response {
    #[serde(skip_serializing_if = "Option::is_none")]
    output_speech: Option<Speech>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reprompt: Option<Reprompt>,
    #[serde(skip_serializing_if = "Option::is_none")]
    should_end_session: Option<bool>,
}

#[derive(Debug, Serialize)]
struct Speech {
    #[serde(rename = "type")]
    kind: &'static str,
    ssml: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Reprompt {
    output_speech: Speech,
}

const HELLO: &str = "<speak>Eco dorsal no ar.</speak>";
const HELP: &str = "<speak>Você pode perguntar: o orca está aberto. Neste teste, respondo apenas se o eco dorsal está no ar.</speak>";
const FALLBACK: &str = "<speak>Não entendi. Diga: o orca está aberto, ou peça ajuda.</speak>";
const REPROMPT: &str = "<speak>Diga: o orca está aberto, ou ajuda.</speak>";

fn speech(ssml: &'static str) -> Speech {
    Speech { kind: "SSML", ssml }
}

/// Builds a deterministic response without I/O, Orca access, or user text in SSML.
///
/// Only the eventual validated HTTP adapter may call this for remote requests.
#[must_use]
pub fn hello_response(request: &Request) -> ResponseEnvelope {
    let response = match request {
        Request::LaunchRequest => spoken(HELLO, false),
        Request::IntentRequest { intent } => match intent.name.as_str() {
            "SaudeSistemaIntent" => spoken(HELLO, true),
            "AMAZON.HelpIntent" => spoken(HELP, false),
            "AMAZON.StopIntent" | "AMAZON.CancelIntent" => spoken("<speak>Até logo.</speak>", true),
            _ => spoken(FALLBACK, false),
        },
        // Alexa does not permit speech in a SessionEndedRequest response.
        Request::SessionEndedRequest | Request::Unknown => Response::default(),
    };
    ResponseEnvelope {
        version: "1.0",
        response,
    }
}

fn spoken(ssml: &'static str, end: bool) -> Response {
    Response {
        output_speech: Some(speech(ssml)),
        reprompt: (!end).then(|| Reprompt {
            output_speech: speech(REPROMPT),
        }),
        should_end_session: Some(end),
    }
}
