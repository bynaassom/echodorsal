use echodorsal_alexa::{RequestEnvelope, hello_response};
use serde_json::{Value, json};

fn respond(request: &Value) -> Value {
    let input: RequestEnvelope = serde_json::from_value(json!({
        "version": "1.0",
        "request": request,
        "unknownFutureField": true
    }))
    .unwrap();
    serde_json::to_value(hello_response(&input.request)).unwrap()
}

#[test]
fn launch_greets_and_keeps_session_open_with_reprompt() {
    let response = respond(&json!({"type": "LaunchRequest"}));
    assert_eq!(response["version"], "1.0");
    assert_eq!(
        response["response"]["outputSpeech"],
        json!({
            "type": "SSML", "ssml": "<speak>Eco dorsal no ar.</speak>"
        })
    );
    assert_eq!(response["response"]["shouldEndSession"], false);
    assert_eq!(
        response["response"]["reprompt"]["outputSpeech"]["type"],
        "SSML"
    );
}

#[test]
fn health_only_reports_hello_and_ends_session() {
    let response = respond(&json!({
        "type": "IntentRequest", "intent": {"name": "SaudeSistemaIntent"}
    }));
    assert_eq!(
        response["response"]["outputSpeech"]["ssml"],
        "<speak>Eco dorsal no ar.</speak>"
    );
    assert_eq!(response["response"]["shouldEndSession"], true);
    assert!(response["response"].get("reprompt").is_none());
}

#[test]
fn help_fallback_and_unknown_intent_offer_only_available_behavior() {
    for name in [
        "AMAZON.HelpIntent",
        "AMAZON.FallbackIntent",
        "<script>unknown</script>",
    ] {
        let response = respond(&json!({"type": "IntentRequest", "intent": {"name": name}}));
        let ssml = response["response"]["outputSpeech"]["ssml"]
            .as_str()
            .unwrap();
        assert!(ssml.contains("o orca está aberto"));
        assert!(!ssml.contains("<script>"));
        assert_eq!(response["response"]["shouldEndSession"], false);
        assert!(response["response"].get("reprompt").is_some());
    }
}

#[test]
fn stop_and_cancel_say_goodbye_without_reprompt() {
    for name in ["AMAZON.StopIntent", "AMAZON.CancelIntent"] {
        let response = respond(&json!({"type": "IntentRequest", "intent": {"name": name}}));
        assert_eq!(
            response["response"]["outputSpeech"]["ssml"],
            "<speak>Até logo.</speak>"
        );
        assert_eq!(response["response"]["shouldEndSession"], true);
        assert!(response["response"].get("reprompt").is_none());
    }
}

#[test]
fn session_ended_and_unsupported_requests_are_silent() {
    for kind in ["SessionEndedRequest", "FutureRequest"] {
        assert_eq!(
            respond(&json!({"type": kind})),
            json!({"version": "1.0", "response": {}})
        );
    }
}

#[test]
fn malformed_intent_is_rejected_during_parsing() {
    assert!(
        serde_json::from_value::<RequestEnvelope>(json!({
            "request": {"type": "IntentRequest", "intent": {}}
        }))
        .is_err()
    );
}

#[test]
fn interaction_model_declares_supported_intents_and_invocation() {
    let model: Value = serde_json::from_str(include_str!(
        "../../../alexa/interactionModels/custom/pt-BR.json"
    ))
    .unwrap();
    let language = &model["interactionModel"]["languageModel"];
    assert_eq!(language["invocationName"], "eco dorsal");
    let intents = language["intents"].as_array().unwrap();
    let names: Vec<_> = intents
        .iter()
        .map(|intent| intent["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "SaudeSistemaIntent",
            "AMAZON.HelpIntent",
            "AMAZON.StopIntent",
            "AMAZON.CancelIntent",
            "AMAZON.FallbackIntent"
        ]
    );
    assert!(!intents[0]["samples"].as_array().unwrap().is_empty());
}
