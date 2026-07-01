use ai_interface::StructuredOutputSchema;
use serde_json::json;

use crate::{parse_structured_output, validate_structured_output};

#[test]
fn parses_and_validates_structured_output() {
    let response_schema = StructuredOutputSchema {
        name: "status".to_owned(),
        schema: json!({
            "type": "object",
            "properties": {
                "message": { "type": "string" },
                "done": { "type": "boolean" }
            },
            "required": ["message", "done"]
        }),
    };

    let structured_output = parse_structured_output(
        "openai",
        "gpt-5.5",
        "{\"message\":\"done\",\"done\":true}",
        &response_schema,
    )
    .expect("structured output should parse");

    assert_eq!(
        structured_output,
        json!({
            "message": "done",
            "done": true
        })
    );
}

#[test]
fn rejects_structured_output_that_fails_schema_validation() {
    let response_schema = StructuredOutputSchema {
        name: "status".to_owned(),
        schema: json!({
            "type": "object",
            "properties": {
                "message": { "type": "string" }
            },
            "required": ["message"]
        }),
    };

    let error = validate_structured_output(
        "openai",
        "gpt-5.5",
        &json!({ "message": 1 }),
        &response_schema,
    )
    .expect_err("invalid structured output should fail");

    assert_eq!(
        error.to_string(),
        "[ai_interface/model] provider failure for `openai` model `gpt-5.5`: structured output did not match schema `status`: 1 is not of type \"string\""
    );
}
