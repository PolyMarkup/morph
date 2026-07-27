use morph::format::Format;
use serde::{Deserialize, Serialize};
use worker::*;

#[derive(Deserialize)]
struct ConvertRequest {
    input: String,
    from: String,
    to: Vec<String>,
}

#[derive(Serialize)]
struct ConvertResult {
    format: String,
    output: Option<String>,
    error: Option<String>,
}

#[derive(Serialize)]
struct ConvertResponse {
    results: Vec<ConvertResult>,
}

#[derive(Serialize)]
struct FormatsResponse {
    formats: Vec<FormatInfo>,
}

#[derive(Serialize)]
struct FormatInfo {
    id: &'static str,
    name: &'static str,
}

fn supported_formats() -> Vec<FormatInfo> {
    Format::ALL
        .iter()
        .map(|f| FormatInfo {
            id: f.id(),
            name: f.name(),
        })
        .collect()
}

#[event(fetch)]
async fn fetch(req: Request, _env: Env, _ctx: Context) -> Result<Response> {
    let router = Router::new();

    router
        .post_async("/api/convert", |mut req, _ctx| async move {
            let body: ConvertRequest = match req.json().await {
                Ok(body) => body,
                Err(e) => return Response::error(format!("Invalid request body: {e}"), 400),
            };

            let from = match Format::from_name(&body.from) {
                Some(f) => f,
                None => {
                    return Response::error(format!("Unknown input format: {}", body.from), 400);
                }
            };

            // Parse once, emit once per requested target format.
            let doc = match morph::parse(&body.input, from) {
                Ok(doc) => doc,
                Err(e) => return Response::error(format!("Parse error: {e}"), 400),
            };

            let results: Vec<ConvertResult> = body
                .to
                .iter()
                .map(|to_name| {
                    let to = match Format::from_name(to_name) {
                        Some(f) => f,
                        None => {
                            return ConvertResult {
                                format: to_name.clone(),
                                output: None,
                                error: Some(format!("Unknown format: {to_name}")),
                            };
                        }
                    };

                    match morph::emit(&doc, to) {
                        Ok(output) => ConvertResult {
                            format: to_name.clone(),
                            output: Some(output),
                            error: None,
                        },
                        Err(e) => ConvertResult {
                            format: to_name.clone(),
                            output: None,
                            error: Some(e.to_string()),
                        },
                    }
                })
                .collect();

            Response::from_json(&ConvertResponse { results })
        })
        .get("/api/formats", |_req, _ctx| {
            Response::from_json(&FormatsResponse {
                formats: supported_formats(),
            })
        })
        .run(req, _env)
        .await
}
