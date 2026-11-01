use maud::{Markup, html};
use pretty_bytes_typed::{pretty_bytes, pretty_bytes_binary};
use proto::backend::{CpuResponse, DiskResponse, MemResponse, NetworkResponse, TempResponse};

use crate::http::query_array::QueryArray;

use super::graph::{Axis, SvgGraph};

fn calc_percent(used: u64, total: u64) -> f32 {
    if total == 0 {
        return 0.;
    };

    let percent = used as f32 / total as f32 * 100.;
    // Round percent to 2 decimal places
    (percent * 100.).round() / 100.
}

fn calc_grid_span(num_elts: usize) -> usize {
    // Starting at two rows, we need roughly 1 row for every 2 elements
    num_elts.div_ceil(2) + 1
}

pub fn cpu_meters(cpu_data: &CpuResponse, temp_data: &TempResponse) -> Markup {
    let cpu_iter = cpu_data.cpus.iter().zip(1_u8..);

    // Add 1 row to account for CPU temperature and global CPU
    let span = calc_grid_span(cpu_data.cpus.len()) + 1;

    html! {
        section .metric-card .{"span-" (span)} {
            h2 { "Processor" }
            @if let Some(temp) = temp_data.temp {
                p { "CPU Temperature: " (temp) "ºC" }
            }
            p .metric-primary { (cpu_data.global_cpu) "%" }
            p .metric-label { "Total CPU usage" }
            .meter-container {
                .bar.cpu style={"--scale:"(cpu_data.global_cpu / 100.)} {}
            }
            @for (usage, num) in cpu_iter {
                p .metric-row { span { "Core " (num) } strong { (usage) "%" } }
                .meter-container {
                    .bar.cpu style={"--scale:"(usage / 100.)} {}
                }
            }
        }
    }
}

pub fn cpu_graph(data: &CpuResponse, points: &mut QueryArray) -> Markup {
    let mut graph = SvgGraph::new(Axis::Percent);

    let points_iter = std::iter::once(data.global_cpu)
        .chain(points.iter())
        .take(20);

    graph.add_series(points_iter.clone(), "var(--chart-cpu)", "CPU");

    *points = points_iter.collect();

    html! {
        section .span-3
        {
            h2 { "CPU activity" }
            p .chart-note { "Recent readings since this page was opened" }
            (graph)
        }
    }
}

pub fn temp_graph(data: &TempResponse, points: &mut QueryArray) -> Option<Markup> {
    data.temp.map(|temp| {
        let mut graph = SvgGraph::new(Axis::Temp);

        let points_iter = std::iter::once(temp).chain(points.iter()).take(20);

        graph.add_series(points_iter.clone(), "var(--chart-temp)", "Temperature");

        *points = points_iter.collect();

        html! {
                section .span-3
                {
                    h2 { "Temperature history" }
                    p .chart-note { "Recent readings since this page was opened" }
                    (graph)
                }
        }
    })
}

pub fn mem_meters(data: &MemResponse) -> Markup {
    let pretty_ram_used = pretty_bytes_binary(data.ram.used, Some(2));
    let pretty_ram_total = pretty_bytes_binary(data.ram.total, Some(2));
    let ram_percent = calc_percent(data.ram.used, data.ram.total);

    let pretty_swap_used = pretty_bytes_binary(data.swap.used, Some(2));
    let pretty_swap_total = pretty_bytes_binary(data.swap.total, Some(2));
    let swap_percent = calc_percent(data.swap.used, data.swap.total);

    html! {
        section .metric-card .span-2 {
            h2 { "Memory" }

            p .metric-row { span { "RAM" } strong { (pretty_ram_used) " / " (pretty_ram_total) } }
            div .meter-container {
                div .bar.ram style={"--scale:"(ram_percent / 100.)} {}
            }

            p .metric-row { span { "Swap" } strong { (pretty_swap_used) " / " (pretty_swap_total) } }
            div .meter-container {
                div .bar.swap style={"--scale:"(swap_percent / 100.)} {}
            }
        }
    }
}

pub fn mem_graph(
    data: &MemResponse,
    ram_points: &mut QueryArray,
    swap_points: &mut QueryArray,
) -> Markup {
    let mut graph = SvgGraph::new(Axis::Percent);

    let ram_percent = calc_percent(data.ram.used, data.ram.total);
    let swap_percent = calc_percent(data.swap.used, data.swap.total);

    let ram_points_iter = std::iter::once(ram_percent)
        .chain(ram_points.iter())
        .take(20);
    let swap_points_iter = std::iter::once(swap_percent)
        .chain(swap_points.iter())
        .take(20);

    graph.add_series(ram_points_iter.clone(), "var(--chart-ram)", "RAM");
    graph.add_series(swap_points_iter.clone(), "var(--chart-swap)", "Swap");

    *ram_points = ram_points_iter.collect();
    *swap_points = swap_points_iter.collect();

    html! {
        section .span-3 {
            h2 { "Memory activity" }
            p .chart-note { "Recent readings since this page was opened" }
            (graph)
        }
    }
}

pub fn disk_meters(data: &DiskResponse) -> Markup {
    let span = calc_grid_span(data.disks.len());

    html! {
        section .metric-card .{"span-" (span)} {
            h2 { "Storage" }

            @for disk in &data.disks {
                @let pretty_disk_used = pretty_bytes(disk.usage.used, Some(2));
                @let pretty_disk_total = pretty_bytes(disk.usage.total, Some(2));
                @let disk_percent = calc_percent(disk.usage.used, disk.usage.total);

                p .metric-row { span { (disk.name) " (" (disk.mnt_point) ")" } strong { (pretty_disk_used) " / " (pretty_disk_total) } }
                .meter-container {
                    .bar.disk style={"--scale:"(disk_percent / 100.)} {}
                }
            }
        }
    }
}

pub fn net_graph(
    data: &NetworkResponse,
    sent_points: &mut QueryArray,
    recv_points: &mut QueryArray,
) -> Markup {
    let mut graph = SvgGraph::new(Axis::Bytes);

    let sent_points_iter = std::iter::once(data.sent as f32)
        .chain(sent_points.iter())
        .take(20);
    let recv_points_iter = std::iter::once(data.recv as f32)
        .chain(recv_points.iter())
        .take(20);

    graph.add_series(sent_points_iter.clone(), "var(--chart-sent)", "Sent");
    graph.add_series(recv_points_iter.clone(), "var(--chart-recv)", "Received");

    *sent_points = sent_points_iter.collect();
    *recv_points = recv_points_iter.collect();

    html! {
        section .span-3 {
            h2 { "Network transfer" }
            p .chart-note { "Bytes since the backend last refreshed its network counters; intervals vary. The vertical scale is logarithmic, not a transfer rate." }
            (graph)
        }
    }
}
