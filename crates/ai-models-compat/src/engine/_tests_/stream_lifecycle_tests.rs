//! Provider ownership and pull semantics through the public model boundary.

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

use ai_interface::{ConversationMessage, Model, ModelRequest};
use json_http::{
    DynJsonHttpSseStream, JsonHttpSseEvent, JsonHttpSseStreamMock, JsonHttpTransportMock,
    TransportBackedJsonHttpClient,
};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::deepseek::DeepSeekModel;

#[tokio::test]
async fn reads_only_on_next_and_drop_closes_the_provider_stream() {
    let reads = Arc::new(AtomicUsize::new(0));
    let closed = Arc::new(AtomicBool::new(false));
    let source: DynJsonHttpSseStream = Box::new(Unimock::new(
        JsonHttpSseStreamMock::next
            .each_call(matching!())
            .answers_arc({
                let reads = reads.clone();
                let close = CloseSignal(closed.clone());
                Arc::new(move |_| {
                    let _retained_close = &close;
                    reads.fetch_add(1, Ordering::SeqCst);
                    Ok(Some(JsonHttpSseEvent {
                        event: None,
                        data: json!({"choices": [{"index": 0, "delta": {"content": "first"}}]})
                            .to_string(),
                    }))
                })
            })
            .at_least_times(0),
    ));
    let transport = Arc::new(Unimock::new(
        JsonHttpTransportMock::execute_sse
            .some_call(matching!(_))
            .answers_arc({
                let source = Mutex::new(Some(source));
                Arc::new(move |_, _| {
                    Ok(source
                        .lock()
                        .expect("source lock")
                        .take()
                        .expect("one stream open"))
                })
            }),
    ));
    let client = Arc::new(TransportBackedJsonHttpClient::new(transport));
    let model = DeepSeekModel::new(client, "key");
    let request = ModelRequest {
        system_prompt: String::new(),
        messages: vec![ConversationMessage::user("hello")],
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    };
    {
        let mut stream = model
            .stream(&request)
            .await
            .expect("stream opens without reading");

        assert_eq!(reads.load(Ordering::SeqCst), 0);
        assert!(!closed.load(Ordering::SeqCst));
        let event = stream
            .next()
            .await
            .expect("first read succeeds")
            .expect("text event");
        assert_eq!(
            serde_json::to_value(event).expect("event serializes"),
            json!({"TextDelta": {"part_index": 0, "delta": "first"}})
        );
        assert_eq!(reads.load(Ordering::SeqCst), 1);
    }

    assert!(closed.load(Ordering::SeqCst));
}

struct CloseSignal(Arc<AtomicBool>);

impl Drop for CloseSignal {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}
