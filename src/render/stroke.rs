use aviutl2::AnyResult;
use skia_safe::paint::{Cap, Join, Style as PaintStyle};
use skia_safe::{Paint, Path, PathEffect, dash_path_effect, path_utils, trim_path_effect};

use super::util::{
    color_with_opacity, invalid_input, parse_hex_color, parse_optional_f32, parse_optional_i32,
};

#[derive(Clone)]
pub(crate) struct StrokeStyle {
    color: skia_safe::Color,
    width: f32,
    dash_intervals: Option<Vec<f32>>,
    dash_phase: f32,
    cap: Cap,
    join: Join,
    miter_limit: f32,
    trim: Option<TrimEffect>,
}

#[derive(Clone, Copy)]
struct TrimEffect {
    start: f32,
    stop: f32,
    overlap: f32,
}

type TrimRange = (f32, f32);

pub(crate) fn parse_stroke_data(data: &str) -> AnyResult<StrokeStyle> {
    let tokens = data.split_whitespace().collect::<Vec<_>>();
    let color = parse_hex_color(
        tokens
            .first()
            .ok_or_else(|| invalid_input("stroke color is missing"))?,
    )?;
    let width = parse_optional_f32(&tokens, 1, 4.0, "stroke width")?.max(0.0);
    let dashed = parse_optional_i32(&tokens, 2, 0, "stroke dashed flag")? != 0;

    let mut index = 3;
    let dash_intervals = if dashed {
        let (intervals, next_index) = parse_dash_intervals(&tokens, index)?;
        index = next_index;
        normalize_dash_intervals(intervals)
    } else {
        if tokens.get(index) == Some(&"{}") {
            index += 1;
        } else if tokens.get(index) == Some(&"{") {
            index = skip_braced_block(&tokens, index)?;
        }
        None
    };

    let dash_phase = parse_optional_f32(&tokens, index, 0.0, "dash phase")?;
    let cap = parse_cap(parse_optional_i32(&tokens, index + 1, 0, "stroke cap")?)?;
    let join = parse_join(parse_optional_i32(&tokens, index + 2, 0, "stroke join")?)?;
    let miter_limit = parse_optional_f32(&tokens, index + 3, 4.0, "miter limit")?.max(0.0);
    let trim = parse_stroke_trim(&tokens, index + 4)?;
    let opacity = parse_optional_f32(&tokens, index + 7, 1.0, "stroke opacity")?;

    Ok(StrokeStyle {
        color: color_with_opacity(color, opacity),
        width,
        dash_intervals,
        dash_phase,
        cap,
        join,
        miter_limit,
        trim,
    })
}

fn parse_dash_intervals(tokens: &[&str], start_index: usize) -> AnyResult<(Vec<f32>, usize)> {
    if tokens.get(start_index) == Some(&"{}") {
        return Ok((Vec::new(), start_index + 1));
    }

    if tokens.get(start_index) != Some(&"{") {
        return Err(invalid_input("dash intervals must start with '{'").into());
    }

    let mut intervals = Vec::new();
    let mut index = start_index + 1;
    while let Some(token) = tokens.get(index) {
        if *token == "}" {
            return Ok((intervals, index + 1));
        }
        intervals.push(
            token
                .parse::<f32>()
                .map_err(|_| invalid_input(format!("dash interval is not a number: {token}")))?,
        );
        index += 1;
    }

    Err(invalid_input("dash intervals must end with '}'").into())
}

fn skip_braced_block(tokens: &[&str], start_index: usize) -> AnyResult<usize> {
    let mut index = start_index + 1;
    while let Some(token) = tokens.get(index) {
        if *token == "}" {
            return Ok(index + 1);
        }
        index += 1;
    }
    Err(invalid_input("braced block must end with '}'").into())
}

fn normalize_dash_intervals(mut intervals: Vec<f32>) -> Option<Vec<f32>> {
    intervals.retain(|value| *value > 0.0);
    if intervals.is_empty() {
        return None;
    }
    if intervals.len() % 2 == 1 {
        let repeat = intervals.clone();
        intervals.extend(repeat);
    }
    Some(intervals)
}

fn parse_cap(value: i32) -> AnyResult<Cap> {
    match value {
        0 => Ok(Cap::Butt),
        1 => Ok(Cap::Square),
        2 => Ok(Cap::Round),
        _ => Err(invalid_input(format!("unsupported stroke cap: {value}")).into()),
    }
}

fn parse_join(value: i32) -> AnyResult<Join> {
    match value {
        0 => Ok(Join::Miter),
        1 => Ok(Join::Round),
        2 => Ok(Join::Bevel),
        _ => Err(invalid_input(format!("unsupported stroke join: {value}")).into()),
    }
}

fn parse_stroke_trim(tokens: &[&str], start_index: usize) -> AnyResult<Option<TrimEffect>> {
    if tokens.len() <= start_index {
        return Ok(None);
    }
    if tokens.len() < start_index + 3 {
        return Err(invalid_input("stroke trim expects start, stop and overlap").into());
    }

    let start = parse_optional_f32(tokens, start_index, 0.0, "trim start")?;
    let stop = parse_optional_f32(tokens, start_index + 1, 1.0, "trim stop")?;
    let overlap = parse_optional_f32(tokens, start_index + 2, 1.0, "trim overlap")?;
    let (start, stop) = ordered_range(start.clamp(0.0, 1.0), stop.clamp(0.0, 1.0));
    Ok(Some(TrimEffect {
        start,
        stop,
        overlap: overlap.clamp(0.0, 1.0),
    }))
}

fn ordered_range(start: f32, stop: f32) -> TrimRange {
    if start <= stop {
        (start, stop)
    } else {
        (stop, start)
    }
}

fn visible_range(start: f32, stop: f32) -> Option<TrimRange> {
    (start < stop).then_some((start, stop))
}

fn make_path_effect(
    dash_intervals: Option<&[f32]>,
    dash_phase: f32,
    trim_range: Option<TrimRange>,
) -> Option<PathEffect> {
    let dash = dash_intervals.and_then(|intervals| dash_path_effect::new(intervals, dash_phase));
    let trim = trim_range.and_then(|(start, stop)| trim_path_effect::new(start, stop, None));

    match (dash, trim) {
        (Some(dash), Some(trim)) => Some(PathEffect::compose(dash, trim)),
        (Some(dash), None) => Some(dash),
        (None, Some(trim)) => Some(trim),
        (None, None) => None,
    }
}

fn make_stroke_paint(stroke: &StrokeStyle, trim_range: Option<TrimRange>) -> Paint {
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_style(PaintStyle::Stroke);
    paint.set_color(stroke.color);
    paint.set_stroke_width(stroke.width);
    paint.set_stroke_cap(stroke.cap);
    paint.set_stroke_join(stroke.join);
    paint.set_stroke_miter(stroke.miter_limit);
    paint.set_path_effect(make_path_effect(
        stroke.dash_intervals.as_deref(),
        stroke.dash_phase,
        trim_range,
    ));
    paint
}

fn calc_trim_range(trim: TrimEffect, path_index: usize, path_count: usize) -> Option<TrimRange> {
    if path_count <= 1 {
        return visible_range(trim.start, trim.stop);
    }

    let path_count = path_count as f32;
    let span = 1.0 / (1.0 + (path_count - 1.0) * (1.0 - trim.overlap));
    let offset = path_index as f32 * span * (1.0 - trim.overlap);

    let start = ((trim.start - offset) / span).clamp(0.0, 1.0);
    let stop = ((trim.stop - offset) / span).clamp(0.0, 1.0);

    visible_range(start, stop)
}

fn trim_range_for_path(
    stroke: &StrokeStyle,
    path_index: usize,
    path_count: usize,
) -> Option<Option<TrimRange>> {
    if let Some(trim) = stroke.trim {
        calc_trim_range(trim, path_index, path_count).map(Some)
    } else {
        Some(None)
    }
}

pub(crate) fn draw_stroke(canvas: &skia_safe::Canvas, paths: &[Path], stroke: &StrokeStyle) {
    for (path_index, path) in paths.iter().enumerate() {
        let Some(trim_range) = trim_range_for_path(stroke, path_index, paths.len()) else {
            continue;
        };
        let paint = make_stroke_paint(stroke, trim_range);
        canvas.draw_path(path, &paint);
    }
}

pub(crate) fn stroke_to_paths(
    canvas: &skia_safe::Canvas,
    paths: &[Path],
    stroke: &StrokeStyle,
) -> Vec<Path> {
    let mut stroked_paths = Vec::new();
    let ctm = canvas.local_to_device_as_3x3();

    for (path_index, path) in paths.iter().enumerate() {
        let Some(trim_range) = trim_range_for_path(stroke, path_index, paths.len()) else {
            continue;
        };
        let paint = make_stroke_paint(stroke, trim_range);
        let mut builder = skia_safe::PathBuilder::new();
        if path_utils::fill_path_with_paint(path, &paint, &mut builder, None, Some(ctm)) {
            let stroked_path = builder.detach();
            let normalized_path = stroked_path
                .simplify()
                .or_else(|| stroked_path.as_winding())
                .unwrap_or(stroked_path);
            stroked_paths.push(normalized_path);
        }
    }

    stroked_paths
}
