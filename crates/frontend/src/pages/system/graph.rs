use maud::{Render, html};
use pretty_bytes_typed::pretty_bytes;

use crate::pages::template::Icon;

const GRAPH_Y_LINES: u32 = 11;
const GRAPH_X_LINES: u32 = 20;
const LINE_SPACING: u32 = 10;

pub struct GraphSeries {
    points: Vec<(u32, f32)>,
    color: &'static str,
    label: &'static str,
}

#[derive(Clone, Copy)]
pub enum Axis {
    Percent,
    Temp,
    Bytes,
}

impl Axis {
    fn format_val(self, val: f32) -> String {
        match self {
            Self::Percent => format!("{}%", val),
            Self::Temp => format!("{}\u{00b0}C", val),
            Self::Bytes => pretty_bytes(val as u64, Some(0)).to_string(),
        }
    }

    fn get_labels(self) -> [String; GRAPH_Y_LINES as usize] {
        let generator_fn = |x| {
            self.format_val(match self {
                Self::Percent => (10 * x) as f32,
                Self::Temp => (10 * x + 20) as f32,
                Self::Bytes => 10_u64.pow(x as u32) as f32,
            })
        };
        std::array::from_fn(generator_fn)
    }

    // Translates data from [min, max] to [0, 100]
    // Percent: [0, 100]
    // Temp: [20, 120]
    // Bytes: [1, 10^10] (log)
    fn interpolate(self, data: f32) -> f32 {
        match self {
            Self::Percent => data,
            Self::Temp => data - 20.,
            Self::Bytes => {
                if data > 0. {
                    10. * data.log10()
                } else {
                    0.
                }
            }
        }
    }
}

pub struct SvgGraph {
    series: Vec<GraphSeries>,
    axis: Axis,
}

impl SvgGraph {
    pub fn new(axis: Axis) -> Self {
        Self {
            series: Vec::new(),
            axis,
        }
    }

    pub fn add_series(
        &mut self,
        points: impl Iterator<Item = f32>,
        color: &'static str,
        label: &'static str,
    ) {
        // Creates (x, y) pairs starting from the right
        let points: Vec<_> = (0..GRAPH_X_LINES).rev().zip(points).collect();

        let series = GraphSeries {
            points,
            color,
            label,
        };

        self.series.push(series);
    }
}

impl Render for SvgGraph {
    fn render(&self) -> maud::Markup {
        let left_margin = LINE_SPACING * 3;
        let right_margin = LINE_SPACING / 2;
        let top_margin = LINE_SPACING / 2;
        let bottom_margin = LINE_SPACING / 2;

        let graph_width = LINE_SPACING * (GRAPH_X_LINES - 1);
        let graph_height = LINE_SPACING * (GRAPH_Y_LINES - 1);

        let x_end = left_margin + graph_width;
        let y_end = top_margin + graph_height;

        let total_width = left_margin + graph_width + right_margin;
        let total_height = top_margin + graph_height + bottom_margin;

        let axis = self.axis.get_labels().into_iter().enumerate();

        html! {
            div .graph-wrapper {
                svg .graph viewBox={"0 0 " (total_width) " " (total_height)} role="img" aria-label="Recent readings graph" {
                    @for (i, label) in axis {
                        @if i % 2 == 0 {
                            @let y = y_end - (i as u32) * LINE_SPACING;
                            line .graph-grid x1=(left_margin) y1=(y) x2=(x_end) y2=(y) {}
                            text x="1" y=(y + 1) { (label) }
                        }
                    }
                    @for series in &self.series {
                        @let polyline_points = {
                            use core::fmt::Write;
                            let mut acc = String::new();
                            for &(x, value) in &series.points {
                                let y = y_end as f32 - self.axis.interpolate(value).clamp(0., 100.);
                                let _ = write!(acc, "{},{} ", left_margin + (LINE_SPACING * x), y);
                            }
                            acc
                        };
                        @if self.series.len() == 1 && !series.points.is_empty() {
                            @let right = left_margin + LINE_SPACING * series.points.first().unwrap().0;
                            @let left = left_margin + LINE_SPACING * series.points.last().unwrap().0;
                            polygon .graph-area points={(polyline_points) " " (left) "," (y_end) " " (right) "," (y_end)} fill=(&series.color) {}
                        }
                        polyline .graph-series points=(polyline_points) stroke=(&series.color) fill="none" {}
                        @if let Some(&(x, value)) = series.points.first() {
                            circle .graph-current cx=(left_margin + LINE_SPACING * x) cy=(y_end as f32 - self.axis.interpolate(value).clamp(0., 100.)) r="1.6" fill=(&series.color) {}
                        }
                    }
                    line .graph-guide x1=(x_end) x2=(x_end) y1=(top_margin) y2=(y_end) visibility="hidden" {}
                    rect .graph-hitbox width=(graph_width) height=(graph_height) x=(left_margin) y=(top_margin) fill="transparent" {}
                }
            }
            div .legend {
                @for series in &self.series {
                    @let point_vals = series.points.iter().map(|&(_, value)| self.axis.format_val(value)).collect::<Vec<_>>().join("|");
                    p {
                        span style={"color:"(series.color)} { (Icon::new("fa6-solid-square").size(12)) }
                        (series.label)
                        span .legend-value data-values=(point_vals) {
                            @if let Some((_, value)) = series.points.first() {
                                (self.axis.format_val(*value))
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Axis;

    #[test]
    fn formats_temperature_with_a_single_degree_symbol() {
        assert_eq!(Axis::Temp.format_val(43.33), "43.33\u{00b0}C");
    }
}
