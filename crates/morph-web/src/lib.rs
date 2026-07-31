mod preservation;

use morph::format::Format;
use preservation::{PreservationReport, analyze};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use worker::*;

const MAX_INPUT_BYTES: usize = 256 * 1024;
const MAX_HTTP_BODY_BYTES: usize = 300 * 1024;
const MAX_TARGETS: usize = 10;

#[derive(Debug, Deserialize)]
struct ConvertRequest {
    input: String,
    from: String,
    to: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ConvertResult {
    format: String,
    output: Option<String>,
    error: Option<String>,
    preservation: Option<PreservationReport>,
}

#[derive(Debug, Serialize)]
struct ConvertResponse {
    results: Vec<ConvertResult>,
}

#[derive(Debug, Serialize)]
struct FormatsResponse {
    formats: Vec<FormatInfo>,
}

#[derive(Debug, Serialize)]
struct FormatInfo {
    id: &'static str,
    name: &'static str,
}

#[derive(Debug, Serialize)]
struct ApiErrorBody {
    code: &'static str,
    message: String,
}

#[derive(Debug)]
struct ApiError {
    status: u16,
    code: &'static str,
    message: String,
}

#[derive(Debug)]
struct ValidatedRequest {
    input: String,
    from: Format,
    to: Vec<Format>,
}

fn supported_formats() -> Vec<FormatInfo> {
    Format::ALL
        .iter()
        .map(|format| FormatInfo {
            id: format.id(),
            name: format.name(),
        })
        .collect()
}

fn validate_request(body: ConvertRequest) -> Result<ValidatedRequest, ApiError> {
    if body.input.trim().is_empty() {
        return Err(api_error(
            400,
            "empty_input",
            "Enter some markup to convert.",
        ));
    }
    if body.input.len() > MAX_INPUT_BYTES {
        return Err(api_error(
            413,
            "input_too_large",
            format!("Input is limited to {MAX_INPUT_BYTES} UTF-8 bytes."),
        ));
    }
    let from = Format::from_name(&body.from).ok_or_else(|| {
        api_error(
            400,
            "unknown_source_format",
            format!("Unknown input format: {}", body.from),
        )
    })?;
    if body.to.is_empty() {
        return Err(api_error(
            400,
            "missing_target",
            "Select at least one output format.",
        ));
    }
    if body.to.len() > MAX_TARGETS {
        return Err(api_error(
            400,
            "too_many_targets",
            format!("At most {MAX_TARGETS} output formats may be requested."),
        ));
    }

    let mut seen = HashSet::new();
    let mut targets = Vec::with_capacity(body.to.len());
    for name in body.to {
        let format = Format::from_name(&name).ok_or_else(|| {
            api_error(
                400,
                "unknown_target_format",
                format!("Unknown output format: {name}"),
            )
        })?;
        if !seen.insert(format.id()) {
            return Err(api_error(
                400,
                "duplicate_target",
                format!("Output format requested more than once: {}", format.name()),
            ));
        }
        targets.push(format);
    }

    Ok(ValidatedRequest {
        input: body.input,
        from,
        to: targets,
    })
}

fn convert(body: ConvertRequest) -> Result<ConvertResponse, ApiError> {
    let request = validate_request(body)?;
    let document = morph::parse(&request.input, request.from).map_err(|error| {
        api_error(
            400,
            "parse_error",
            format!("Could not parse {}: {error}", request.from.name()),
        )
    })?;
    let results = request
        .to
        .into_iter()
        .map(|format| match morph::emit(&document, format) {
            Ok(output) => ConvertResult {
                preservation: Some(analyze(&document, &output, format)),
                format: format.id().to_string(),
                output: Some(output),
                error: None,
            },
            Err(error) => ConvertResult {
                format: format.id().to_string(),
                output: None,
                error: Some(error.to_string()),
                preservation: None,
            },
        })
        .collect();

    Ok(ConvertResponse { results })
}

fn api_error(status: u16, code: &'static str, message: impl Into<String>) -> ApiError {
    ApiError {
        status,
        code,
        message: message.into(),
    }
}

fn json_response<T: Serialize>(value: &T, status: u16) -> Result<Response> {
    let mut response = Response::from_json(value)?.with_status(status);
    response.headers_mut().set("Cache-Control", "no-store")?;
    response
        .headers_mut()
        .set("X-Content-Type-Options", "nosniff")?;
    Ok(response)
}

fn error_response(error: ApiError) -> Result<Response> {
    json_response(
        &ApiErrorBody {
            code: error.code,
            message: error.message,
        },
        error.status,
    )
}

#[event(fetch)]
async fn fetch(req: Request, _env: Env, _ctx: Context) -> Result<Response> {
    let router = Router::new();

    router
        .post_async("/api/convert", |mut req, _ctx| async move {
            let content_type = req
                .headers()
                .get("Content-Type")?
                .unwrap_or_default()
                .to_ascii_lowercase();
            if !content_type.starts_with("application/json") {
                return error_response(api_error(
                    415,
                    "unsupported_media_type",
                    "Use Content-Type: application/json.",
                ));
            }
            if let Some(content_length) = req.headers().get("Content-Length")?
                && content_length.parse::<usize>().unwrap_or(usize::MAX) > MAX_HTTP_BODY_BYTES
            {
                return error_response(api_error(
                    413,
                    "request_too_large",
                    format!("Request bodies are limited to {MAX_HTTP_BODY_BYTES} bytes."),
                ));
            }
            let body: ConvertRequest = match req.json().await {
                Ok(body) => body,
                Err(error) => {
                    return error_response(api_error(
                        400,
                        "invalid_json",
                        format!("Invalid request body: {error}"),
                    ));
                }
            };

            match convert(body) {
                Ok(response) => json_response(&response, 200),
                Err(error) => error_response(error),
            }
        })
        .get("/api/formats", |_req, _ctx| {
            json_response(
                &FormatsResponse {
                    formats: supported_formats(),
                },
                200,
            )
        })
        .run(req, _env)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use preservation::PreservationStatus;

    fn request(input: &str, from: &str, to: &[&str]) -> ConvertRequest {
        ConvertRequest {
            input: input.to_string(),
            from: from.to_string(),
            to: to.iter().map(|format| (*format).to_string()).collect(),
        }
    }

    #[test]
    fn converts_and_reports_preservation() {
        let response = convert(request("# Hello\n", "md", &["html", "adoc"])).unwrap();

        assert_eq!(response.results.len(), 2);
        assert!(
            response
                .results
                .iter()
                .all(|result| result.output.is_some())
        );
        assert!(response.results.iter().all(|result| {
            result
                .preservation
                .as_ref()
                .is_some_and(|report| report.status == PreservationStatus::Preserved)
        }));
    }

    #[test]
    fn validates_empty_and_oversized_input() {
        assert_eq!(
            validate_request(request("  ", "md", &["html"]))
                .unwrap_err()
                .code,
            "empty_input"
        );
        let oversized = "a".repeat(MAX_INPUT_BYTES + 1);
        let error = validate_request(request(&oversized, "md", &["html"])).unwrap_err();
        assert_eq!(error.status, 413);
        assert_eq!(error.code, "input_too_large");
    }

    #[test]
    fn validates_source_and_target_formats() {
        assert_eq!(
            validate_request(request("# x", "nope", &["html"]))
                .unwrap_err()
                .code,
            "unknown_source_format"
        );
        assert_eq!(
            validate_request(request("# x", "md", &["nope"]))
                .unwrap_err()
                .code,
            "unknown_target_format"
        );
        assert_eq!(
            validate_request(request("# x", "md", &[]))
                .unwrap_err()
                .code,
            "missing_target"
        );
    }

    #[test]
    fn rejects_duplicate_aliases_and_excess_targets() {
        assert_eq!(
            validate_request(request("# x", "md", &["md", "markdown"]))
                .unwrap_err()
                .code,
            "duplicate_target"
        );
        let formats = vec!["md"; MAX_TARGETS + 1];
        assert_eq!(
            validate_request(request("# x", "md", &formats))
                .unwrap_err()
                .code,
            "too_many_targets"
        );
    }

    #[test]
    fn parse_errors_are_structured() {
        let error = convert(request("<script>x</script>", "html", &["md"])).unwrap_err();
        assert_eq!(error.status, 400);
        assert_eq!(error.code, "parse_error");
    }
}
