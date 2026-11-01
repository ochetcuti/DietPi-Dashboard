use hyper::{Method, StatusCode, header};

use crate::pages::*;

use super::response::{BuiltResponse, RedirectType, ServerResponse};
use super::{request::ServerRequest, statics};

const GET: &Method = &Method::GET;
const POST: &Method = &Method::POST;

macro_rules! router {
    ($req:expr, $path:expr, {
        $( ($method:pat, $paths:pat) => $handler:expr, )*
        _ => $fallback:expr,
    }) => {{
        match (&$req.method, $path) {
            $(
                ($method, $paths) => match $handler($req).await {
                    Ok(resp) | Err(resp) => resp
                },
            )*
            _ => $fallback()
        }
    }};
}

fn same_origin_post(headers: &hyper::HeaderMap) -> bool {
    let fetch_site = headers.get("sec-fetch-site").and_then(|v| v.to_str().ok());
    if fetch_site == Some("cross-site") {
        return false;
    }
    // Browsers set Fetch Metadata themselves, including behind reverse proxies.
    if fetch_site == Some("same-origin") {
        return true;
    }
    let Some(origin) = headers.get(header::ORIGIN) else {
        return true;
    };
    let Some(origin) = origin
        .to_str()
        .ok()
        .and_then(|s| s.parse::<hyper::Uri>().ok())
    else {
        return false;
    };
    let Some(scheme) = origin.scheme_str() else {
        return false;
    };
    let default_port = match scheme {
        "http" => 80,
        "https" => 443,
        _ => return false,
    };
    let Some(authority) = origin.authority() else {
        return false;
    };
    let Some(host) = headers
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<hyper::http::uri::Authority>().ok())
    else {
        return false;
    };
    authority.host().eq_ignore_ascii_case(host.host())
        && authority.port_u16().unwrap_or(default_port) == host.port_u16().unwrap_or(default_port)
}

pub async fn router(req: ServerRequest) -> Result<BuiltResponse, std::convert::Infallible> {
    if req.method == Method::POST && !same_origin_post(&req.headers) {
        return Ok(ServerResponse::new()
            .status(StatusCode::FORBIDDEN)
            .body("Cross-site request rejected")
            .build());
    }
    if req.method == Method::POST
        && req.uri.path() != "/login"
        && req.uri.path() != "/logout"
        && let Err(response) = req.require_authentication()
    {
        return Ok(response.build());
    }
    let path_segments: Vec<_> = req.path_segments().collect();

    let resp = router!(req, &*path_segments, {
        (GET, ["static", "main.css"]) => statics::css,
        (GET, ["static", "main.js"]) => statics::js,
        (GET, ["static", "icons.svg"]) => statics::icons,
        (GET, ["favicon.svg"]) => statics::favicon,

        (GET, []) => async |_| { Ok(ServerResponse::new().redirect(RedirectType::Permanent, "/system")) },

        (GET, ["login"]) => login::page,
        (POST, ["login"]) => login::form,
        (POST, ["logout"]) => login::logout,

        (GET, ["system"]) => system::page,

        (GET, ["process"]) => process::page,
        (POST, ["process", "signal"]) => process::signal,

        (GET, ["software"]) => software::page,
        (POST, ["software"]) => software::form,

        (GET, ["service"]) => service::page,

        (GET, ["management"]) => management::page,

        (GET, ["terminal"]) => terminal::page,
        (GET, ["terminal", "stream"]) => terminal::stream,
        (POST, ["terminal", "write"]) => terminal::write,
        (POST, ["terminal", "resize"]) => terminal::resize,

        (GET, ["browser"]) => browser::page,
        (GET, ["browser", "file"]) => browser::file,
        (POST, ["browser", "file", "save"]) => browser::save,
        (GET, ["browser", "actions"]) => browser::actions,
        (POST, ["browser", "actions", "new-file"]) => browser::new_file,
        (POST, ["browser", "actions", "new-folder"]) => browser::new_folder,
        (POST, ["browser", "actions", "rename"]) => browser::rename,
        (POST, ["browser", "actions", "delete-file"]) => browser::delete_file,
        (POST, ["browser", "actions", "delete-folder"]) => browser::delete_folder,
        (GET, ["browser", "actions", "download"]) => browser::download,
        (POST, ["browser", "actions", "upload"]) => browser::upload,

        _ => || { ServerResponse::new().status(StatusCode::NOT_FOUND).body("page not found") },
    });

    let resp = resp
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .header(header::X_FRAME_OPTIONS, "sameorigin")
        .header(header::X_XSS_PROTECTION, "1; mode=block")
        .header("X-Robots-Tag", "none")
        .header(header::REFERRER_POLICY, "no-referrer");

    let resp = resp.build();

    Ok(resp)
}

#[cfg(test)]
mod tests {
    use super::same_origin_post;
    use hyper::{HeaderMap, header};

    fn headers(origin: &str, host: &str, fetch_site: Option<&str>) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(header::ORIGIN, origin.parse().unwrap());
        headers.insert(header::HOST, host.parse().unwrap());
        if let Some(value) = fetch_site {
            headers.insert("sec-fetch-site", value.parse().unwrap());
        }
        headers
    }

    #[test]
    fn permits_browser_same_origin_behind_proxy() {
        assert!(same_origin_post(&headers(
            "https://dashboard.example",
            "localhost:5252",
            Some("same-origin")
        )));
    }

    #[test]
    fn normalizes_default_ports_without_fetch_metadata() {
        assert!(same_origin_post(&headers(
            "https://dashboard.example",
            "dashboard.example:443",
            None
        )));
        assert!(same_origin_post(&headers(
            "http://dashboard.example:80",
            "DASHBOARD.EXAMPLE",
            None
        )));
        assert!(!same_origin_post(&headers(
            "https://dashboard.example:8443",
            "dashboard.example",
            None
        )));
    }

    #[test]
    fn blocks_cross_site_and_different_origins() {
        assert!(!same_origin_post(&headers(
            "https://other.example",
            "dashboard.example",
            Some("cross-site")
        )));
        assert!(!same_origin_post(&headers(
            "https://other.example",
            "dashboard.example",
            None
        )));
        assert!(!same_origin_post(&headers(
            "null",
            "dashboard.example",
            None
        )));
    }
}
