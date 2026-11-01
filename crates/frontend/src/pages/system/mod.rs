use maud::html;

use crate::http::{request::ServerRequest, response::ServerResponse};

use super::template::{send_req, template};

mod fragments;
mod graph;

pub async fn page(req: ServerRequest) -> Result<ServerResponse, ServerResponse> {
    req.check_login()?;

    let cpu_data = send_req!(req, Cpu)?;
    let temp_data = send_req!(req, Temp)?;
    let mem_data = send_req!(req, Mem)?;
    let disk_data = send_req!(req, Disk)?;
    let net_data = send_req!(req, NetIO)?;

    let history = req.extract_backends()?.current_backend.handle.history();

    let cpu_meters = fragments::cpu_meters(&cpu_data, &temp_data);
    let mem_meters = fragments::mem_meters(&mem_data);
    let disk_meters = fragments::disk_meters(&disk_data);

    let cpu_graph = fragments::cpu_graph(&cpu_data, &history);
    let temp_graph = fragments::temp_graph(&temp_data, &history);
    let mem_graph = fragments::mem_graph(&mem_data, &history);
    let net_graph = fragments::net_graph(&net_data, &history);

    let content = html! {
            p #system-status .monitor-status role="status" hidden {}
            div #system-swap .card-grid
                nm-bind="oninit: () => startSystemMonitor(this)"
            {
                (cpu_meters)
                (cpu_graph)
                @if let Some(temp_graph) = temp_graph {
                    (temp_graph)
                }
                (mem_meters)
                (mem_graph)
                (disk_meters)
                (net_graph)
            }
    };

    template(&req, content, "")
}
