// Server-side feed fetching for the desktop build.
//
// The renderer asks /proxy?url=...; this module performs the request itself,
// so no remote server needs to send CORS headers. Only http(s) targets are
// accepted, and the request runs with a fixed User-Agent like the web build's
// server function does.

pub fn fetch_remote(query: Option<&str>) -> Result<(u16, String, Vec<u8>), String> {
    let query = query.unwrap_or("");
    let mut target: Option<String> = None;
    for pair in query.split('&') {
        let mut parts = pair.split('=');
        let key = parts.next().unwrap_or("");
        let value = parts.next().unwrap_or("");
        if key == "url" {
            target = Some(
                percent_encoding::percent_decode_str(value)
                    .decode_utf8_lossy()
                    .into_owned(),
            );
            break;
        }
    }

    let target = match target {
        Some(target) if !target.is_empty() => target,
        _ => return Err("A feed URL is required.".to_string()),
    };

    let parsed = url::Url::parse(&target).map_err(|error| format!("Invalid URL: {error}"))?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err("Only http and https feeds are supported.".to_string());
    }

    let client = reqwest::blocking::Client::builder()
        .user_agent("MarginaliaRSSReader/1.0")
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|error| error.to_string())?;

    let response = client
        .get(parsed)
        .send()
        .map_err(|error| error.to_string())?;

    let status = response.status().as_u16();
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("application/xml")
        .to_string();
    let body = response.bytes().map_err(|error| error.to_string())?;

    if body.is_empty() {
        return Err("The feed returned an empty response.".to_string());
    }

    Ok((status, content_type, body.to_vec()))
}
