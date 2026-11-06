//! Event collection without provider helper dependencies.

use ai_interface::{
    Model, ModelError, ModelRequest, ModelResponse, ModelResponseFold, ModelResult, SafeErrorSource,
};
use serde_json::Value;

pub(super) async fn collect_model_events(
    model: &dyn Model,
    request: &ModelRequest,
) -> (ModelResult<ModelResponse>, Vec<Value>) {
    let mut events = Vec::new();
    let mut stream = match model.stream(request).await {
        Ok(stream) => stream,
        Err(error) => return (Err(error), events),
    };
    let mut fold = ModelResponseFold::new();
    loop {
        let event = match stream.next().await {
            Ok(Some(event)) => event,
            Ok(None) => return (fold.finish(), events),
            Err(error) => return (Err(error), events),
        };
        let snapshot = match serde_json::to_value(&event) {
            Ok(snapshot) => snapshot,
            Err(source) => {
                return (
                    Err(ModelError::internal(
                        "local",
                        "model-event-collector",
                        SafeErrorSource::redacted(source),
                    )),
                    events,
                );
            }
        };
        events.push(snapshot);
        if let Err(error) = fold.push(event) {
            return (Err(error), events);
        }
    }
}
