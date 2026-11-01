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

pub async fn router(req: ServerRequest) -> Result<BuiltResponse, std::convert::Infallible> {
    if req.method == Method::POST {
        let cross_site = req
            .headers
            .get("sec-fetch-site")
            .and_then(|value| value.to_str().ok())
            == Some("cross-site");
        let origin_mismatch = req
            .headers
            .get(header::ORIGIN)
            .and_then(|value| value.to_str().ok())
            .map(|origin| {
                let origin_authority = origin
                    .parse::<hyper::Uri>()
                    .ok()
                    .and_then(|uri| uri.authority().cloned());
                let host = req
                    .headers
                    .get(header::HOST)
                    .and_then(|value| value.to_str().ok());
                origin_authority
                    .as_ref()
                    .map(|authority| authority.as_str())
                    != host
            })
            .unwrap_or(false);
        if cross_site || origin_mismatch {
            return Ok(ServerResponse::new()
                .status(StatusCode::FORBIDDEN)
                .body("Cross-site request rejected")
                .build());
        }
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
