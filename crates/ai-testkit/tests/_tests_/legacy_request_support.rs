//! Capture converted history at the production adapter's HTTP boundary.

use std::sync::{Arc, Mutex};

use ai_interface::{ConversationMessage, ModelRequest};
use ai_testkit::{
    provider::{ModelFactory, Provider, ProviderModelFactory},
    scenario::Scenario,
};
use json_http::{
    DynJsonHttpSseStream, JsonHttpRequest, JsonHttpTransportMock, TransportBackedJsonHttpClient,
};
use serde_json::Value;
use unimock::{MockFn, Unimock, matching};

pub(super) async fn request_body(provider: Provider, message: ConversationMessage) -> Value {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let transport = Arc::new(Unimock::new(
        JsonHttpTransportMock::execute_sse
            .some_call(matching!(_))
            .answers_arc({
                let requests = requests.clone();
                Arc::new(
                    move |_,
                          request: &JsonHttpRequest|
                          -> json_http::Result<DynJsonHttpSseStream> {
                        requests.lock().unwrap().push(
                            request
                                .body
                                .as_ref()
                                .unwrap()
                                .as_json_bytes()
                                .map(|bytes| {
                                    serde_json::from_slice::<serde_json::Value>(bytes)
                                        .expect("valid JSON request body")
                                })
                                .unwrap()
                                .clone(),
                        );
                        Ok(Box::new(Unimock::new(())))
                    },
                )
            }),
    ));
    let spec = provider.model(Scenario::TextTurn).unwrap();
    let model = ProviderModelFactory
        .build(
            provider,
            &spec.text_id(),
            Arc::new(TransportBackedJsonHttpClient::new(transport)),
            provider.auth("test-key".to_owned()),
        )
        .unwrap();
    let _stream = model
        .stream(&ModelRequest {
            system_prompt: String::new(),
            messages: vec![message],
            tools: Vec::new(),
            response_schema: None,
            controls: Default::default(),
        })
        .await
        .unwrap();
    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 1);
    requests[0].clone()
}
