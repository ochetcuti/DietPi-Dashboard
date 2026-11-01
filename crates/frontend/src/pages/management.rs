use std::time::Duration;

use maud::html;

use crate::http::{request::ServerRequest, response::ServerResponse};

use super::template::{send_req, template};

pub async fn page(req: ServerRequest) -> Result<ServerResponse, ServerResponse> {
    req.require_authentication()?;

    let data = send_req!(req, Host)?;

    let pretty_time = humantime::format_duration(Duration::from_secs(data.uptime));

    let content = html! {
        section {
            h2 { "Host Information" }

            table .management-table {
                tr {
                    td { "Hostname" }
                    td { (data.hostname) }
                }
                tr {
                    td { "Network Interface" }
                    td { (data.nic) }
                }
                tr {
                    td { "Uptime" }
                    td { (pretty_time) }
                }
                tr {
                    td { "Installed Packages" }
                    td { (data.num_pkgs) }
                }
                tr {
                    td { "OS Version" }
                    td { (data.os_version) }
                }
                tr {
                    td { "Kernel Version" }
                    td { (data.kernel) }
                }
                tr {
                    td { "DietPi Version" }
                    td { (data.dp_version) }
                }
                tr {
                    td { "Architecture" }
                    td { (data.arch) }
                }
            }
        }
        br;
        section {
            h2 { "Dashboard configuration" }
            p { "Configuration files remain on the host. Secrets and credentials are not displayed in the dashboard." }
            table .management-table {
                tr { td { "Authentication" } td { (if req.config().enable_login { "Enabled" } else { "Disabled" }) } }
                tr { td { "HTTPS" } td { (if req.config().enable_tls { "Enabled" } else { "Disabled" }) } }
            }
        }
        @if req.config().enable_login {
            br;
            section {
                h2 { "Dashboard Administration" }

                form action="/logout" method="POST" {
                    button .logout { "Logout" }
                }
            }
        }
    };

    template(&req, content, "")
}
