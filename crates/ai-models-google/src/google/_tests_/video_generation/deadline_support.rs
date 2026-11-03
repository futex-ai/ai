//! Transport callbacks advance the injected polling clock without waiting.

use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use ai_models_core::{ApiKeyScheme, PollingRuntimeMock, ProviderConfig};
use json_http::{
    JsonHttpRequest, JsonHttpResponse, JsonHttpTransportMock, TransportBackedJsonHttpClient,
};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::{GoogleVideoGenerator, PROVIDER_ID, google::error_context_support::unused_clock};

use super::client_tests::stationary_runtime;

const OPERATION: &str = "models/veo-current/operations/held_request";
const SAMPLE: &str = "https://google.test/files/1";

type Requests = Arc<Mutex<Vec<JsonHttpRequest>>>;

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum Phase {
    Submission,
    Poll,
    Download,
}

#[derive(Clone, Copy)]
pub(super) enum Failure {
    Transport,
    BufferedStatus,
    StatusError,
}

pub(super) fn fixture(phase: Phase, failure: Failure) -> (GoogleVideoGenerator, Requests) {
    let time = Arc::new(Mutex::new(stationary_runtime().now()));
    let now = PollingRuntimeMock::now.each_call(matching!()).answers_arc({
        let time = time.clone();
        Arc::new(move |_| *time.lock().unwrap())
    });
    let runtime = if phase == Phase::Submission {
        Unimock::new(now)
    } else {
        Unimock::new((
            now,
            PollingRuntimeMock::sleep
                .each_call(matching!(_))
                .answers_arc({
                    let time = time.clone();
                    Arc::new(move |_, duration| *time.lock().unwrap() += duration)
                }),
        ))
    };
    let requests: Requests = Arc::default();
    let execute = JsonHttpTransportMock::execute
        .each_call(matching!(_))
        .answers_arc({
            let time = time.clone();
            let requests = requests.clone();
            Arc::new(move |_, request| {
                let call = {
                    let mut requests = requests.lock().unwrap();
                    requests.push(request.clone());
                    requests.len()
                };
                let pending = (call == 1 && phase != Phase::Submission)
                    || (phase == Phase::Poll && call == 2);
                if pending || phase == Phase::Download {
                    *time.lock().unwrap() += Duration::from_secs(1);
                    return Ok(JsonHttpResponse {
                        status: 200,
                        headers: Default::default(),
                        body: if pending {
                            json!({"name":OPERATION})
                        } else {
                            json!({"name":OPERATION,"done":true,"response":{"generateVideoResponse":{"generatedSamples":[{"video":{"uri":SAMPLE}}]}}})
                        },
                    });
                }
                match failure {
                    Failure::Transport => Err(held_failure(&time, request)),
                    Failure::BufferedStatus | Failure::StatusError => {
                        *time.lock().unwrap() += Duration::from_secs(1);
                        let body = json!({"error":{"message":"provider unavailable"}});
                        if matches!(failure, Failure::StatusError) {
                            Err(json_http::Error::HttpStatus {
                                status: 500,
                                headers: Box::default(),
                                body,
                            })
                        } else {
                            Ok(JsonHttpResponse {
                                status: 500,
                                headers: Default::default(),
                                body,
                            })
                        }
                    }
                }
            })
        });
    let transport = if phase == Phase::Download {
        Unimock::new((
            execute,
            JsonHttpTransportMock::execute_bytes
                .some_call(matching!(_))
                .answers_arc({
                    let requests = requests.clone();
                    Arc::new(move |_, request| {
                        requests.lock().unwrap().push(request.clone());
                        Err(held_failure(&time, request))
                    })
                }),
        ))
    } else {
        Unimock::new(execute)
    };
    let client = Arc::new(TransportBackedJsonHttpClient::new(Arc::new(transport)));
    let generator = GoogleVideoGenerator::with_auth_and_runtime(
        client,
        "veo-current",
        ProviderConfig::new(PROVIDER_ID, "placeholder-video-key")
            .auth(ApiKeyScheme::Header("x-goog-api-key")),
        Arc::new(runtime),
    )
    .with_base_url("https://google.test/v1beta")
    .with_polling(Duration::from_secs(1), Duration::from_secs(10))
    .with_clock(unused_clock());
    (generator, requests)
}

fn held_failure(time: &Mutex<Instant>, request: &JsonHttpRequest) -> json_http::Error {
    *time.lock().unwrap() += request.timeout + Duration::from_secs(1);
    json_http::Error::transport("held request failed placeholder-video-key")
        .with_response_headers(Default::default())
}
