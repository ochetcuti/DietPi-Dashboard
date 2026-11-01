use maud::{Markup, html};
use proto::{
    backend::{SoftwareInfo, SoftwareResponse},
    frontend::CommandAction,
};
use serde::Deserialize;

use crate::{
    http::{query_array::QueryArray, request::ServerRequest, response::ServerResponse},
    pages::template::Icon,
};

use super::template::{send_req, template};

fn software_table(list: &[SoftwareInfo], idx: u8, pretty_action: &str, action: &str) -> Markup {
    html! {
        div nm-bind={"hidden: () => activeIdx !== " (idx)} {
            div nm-data="software: new Map()" {
                table .software-table {
                    tr {
                        th { "Name" }
                        th { "Description" }
                        th { "Dependencies" }
                        th { "Docs" }
                        th { (pretty_action) }
                    }
                    @for item in list {
                        tr data-software-row data-search=(format!("{} {}", item.name.to_lowercase(), item.desc.to_lowercase())) {
                            td { (item.name) }
                            td { (item.desc) }
                            td { (item.deps) }
                            td {
                                @if item.docs.starts_with("http") {
                                    a href=(item.docs) { (item.docs) }
                                } @else {
                                    (item.docs)
                                }
                            }
                            td {
                                input type="checkbox" data-name=(&item.name) aria-label={"Select " (item.name)} nm-bind={
                                    "onchange: () => {
                                        this.checked ? software.set("(item.id)", this.dataset.name) : software.delete("(item.id)");
                                        software = software;
                                    }"
                                };
                            }
                        }
                    }
                }
                br;
                button .software-input
                    data-action=(action)
                    nm-bind="
                        onclick: () => { if (software.size && confirm('Apply this software operation to the selected items?')) $post('/software', { software: [...software.keys()] }) },
                        disabled: () => _nmFetching
                    "
                {
                    span .spinner { (Icon::new("svg-spinners-180-ring")) }
                    (pretty_action) " "
                    span nm-bind="textContent: () => [...software.values()].join(', ')" {}
                }
            }
        }
    }
}

fn card(data: &SoftwareResponse) -> Markup {
    html! {
        section #software-card {
            h2 { "DietPi-Software" }
            p .helper-text { "This list comes from DietPi-Software. It does not include all system packages or report available updates or service status." }
            @if data.installed.is_empty() && data.uninstalled.is_empty() {
                p .inventory-warning role="status" { "No DietPi-Software inventory was returned. The tool may be unavailable or could not read its state." }
            }
            label for="software-search" { "Filter software" }
            input #software-search type="search" placeholder="Search by name or description" oninput="filterSoftwareRows()";
            .tab-container role="tablist" nm-data="activeIdx: 0" {
                .tabs {
                    button nm-bind="
                        onclick: () => activeIdx = 0,
                        ariaSelected: () => activeIdx === 0
                    " { "Available" }
                    button nm-bind="
                        onclick: () => activeIdx = 1,
                        ariaSelected: () => activeIdx === 1
                    " { "Installed" }
                }
                (software_table(&data.uninstalled, 0, "Install", "install"))
                (software_table(&data.installed, 1, "Uninstall", "uninstall"))
            }
        }
    }
}

pub async fn page(req: ServerRequest) -> Result<ServerResponse, ServerResponse> {
    req.check_login()?;

    let data = send_req!(req, Software)?;

    let content = html! {
        (card(&data))
        br;
        #output {}
    };

    template(&req, content, "")
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum SoftwareAction {
    Install,
    Uninstall,
}

#[derive(Deserialize)]
struct SoftwareForm {
    software: QueryArray,
    action: SoftwareAction,
}

pub async fn form(mut req: ServerRequest) -> Result<ServerResponse, ServerResponse> {
    req.check_login()?;

    let form: SoftwareForm = req.extract_form().await?;

    let action = match form.action {
        SoftwareAction::Install => "install",
        SoftwareAction::Uninstall => "uninstall",
    }
    .into();

    let mut args = vec![action];
    for id in form.software.iter::<u16>() {
        args.push(id.to_string());
    }

    let msg = CommandAction {
        cmd: "/boot/dietpi/dietpi-software".into(),
        args,
    };

    let resp = send_req!(req, Command(msg))?;
    let output = String::from_utf8_lossy(&resp.output);

    let data = send_req!(req, Software)?;

    let content = html! {
        (card(&data))
        br;
        section #output nm-bind="oninit: () => this.scrollIntoView()" {
            h2 { "Operation output" }
            pre {
                (output)
            }
        }
    };

    template(&req, content, "")
}
