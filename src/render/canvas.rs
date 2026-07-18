use aviutl2::AnyResult;
use skia_safe::paint::Style as PaintStyle;
use skia_safe::{Paint, Path};

use super::path::{
    apply_boolean, apply_path_duplicate_array, apply_path_duplicate_grid,
    apply_path_duplicate_random, apply_path_noise, apply_path_transform, apply_round,
    apply_simplify, create_path, parse_boolean_op,
};
use super::stroke::{StrokeStyle, draw_stroke, parse_stroke_data, stroke_to_paths};
use super::text::{create_text_paths, parse_text_path_command};
use super::transform::apply_svg_transform;
use super::util::{invalid_input, parse_fill_color, parse_optional_f32, parse_optional_i32};

fn parse_command(command: &str) -> (&str, &str) {
    if let Some((name, data)) = command.split_once(':') {
        return (name.trim(), data.trim_start_matches(' '));
    }
    (command.trim(), "")
}

pub(crate) fn run_canvas_commands(canvas: &skia_safe::Canvas, commands: &str) -> AnyResult<()> {
    let mut paths: Vec<Path> = Vec::new();
    let mut pending_stroke: Option<StrokeStyle> = None;

    for command in commands.lines() {
        if command.trim().is_empty() {
            continue;
        }

        let (name, data) = parse_command(command);
        match name {
            "path" => {
                if let Some(stroke) = pending_stroke.take() {
                    draw_stroke(canvas, &paths, &stroke);
                }
                paths.push(create_path(data)?);
            }
            "text_path" => {
                if let Some(stroke) = pending_stroke.take() {
                    draw_stroke(canvas, &paths, &stroke);
                }
                paths.extend(create_text_paths(parse_text_path_command(data)?));
            }
            "transform" => {
                if let Some(stroke) = pending_stroke.take() {
                    draw_stroke(canvas, &paths, &stroke);
                }
                apply_svg_transform(canvas, data)?;
            }
            "stroke" => {
                if !paths.is_empty() {
                    if let Some(stroke) = pending_stroke.replace(parse_stroke_data(data)?) {
                        draw_stroke(canvas, &paths, &stroke);
                    }
                }
            }
            "stroke_to_path" => {
                if let Some(stroke) = pending_stroke.take() {
                    paths = stroke_to_paths(canvas, &paths, &stroke);
                }
            }
            "boolean" => {
                if let Some(stroke) = pending_stroke.take() {
                    draw_stroke(canvas, &paths, &stroke);
                }
                paths = apply_boolean(&paths, parse_boolean_op(data)?);
            }
            "round" => {
                if let Some(stroke) = pending_stroke.take() {
                    draw_stroke(canvas, &paths, &stroke);
                }
                let radius = data
                    .trim()
                    .parse::<f32>()
                    .map_err(|_| invalid_input(format!("round radius is not a number: {data}")))?;
                paths = apply_round(&paths, radius);
            }
            "simplify" => {
                if let Some(stroke) = pending_stroke.take() {
                    draw_stroke(canvas, &paths, &stroke);
                }
                paths = apply_simplify(&paths);
            }
            "path_noise" => {
                if let Some(stroke) = pending_stroke.take() {
                    draw_stroke(canvas, &paths, &stroke);
                }
                let tokens = data.split_whitespace().collect::<Vec<_>>();
                let segment_length = parse_optional_f32(&tokens, 0, 8.0, "noise segment length")?;
                let deviation = parse_optional_f32(&tokens, 1, 2.0, "noise deviation")?;
                let seed = parse_optional_i32(&tokens, 2, 0, "noise seed")? as u32;
                let sketch_count =
                    parse_optional_i32(&tokens, 3, 1, "noise sketch count")?.max(1) as usize;
                paths = apply_path_noise(&paths, segment_length, deviation, seed, sketch_count);
            }
            "path_duplicate_array" => {
                if let Some(stroke) = pending_stroke.take() {
                    draw_stroke(canvas, &paths, &stroke);
                }
                let tokens = data.split_whitespace().collect::<Vec<_>>();
                let count = parse_optional_i32(&tokens, 0, 2, "array count")?.max(1) as usize;
                let dx = parse_optional_f32(&tokens, 1, 0.0, "array x offset")?;
                let dy = parse_optional_f32(&tokens, 2, 0.0, "array y offset")?;
                let rotation = parse_optional_f32(&tokens, 3, 0.0, "array rotation offset")?;
                let pivot_x = parse_optional_f32(&tokens, 4, 0.0, "array rotation pivot x")?;
                let pivot_y = parse_optional_f32(&tokens, 5, 0.0, "array rotation pivot y")?;
                let separate_copies =
                    parse_optional_i32(&tokens, 6, 1, "array separate copies")? != 0;
                paths = apply_path_duplicate_array(
                    &paths,
                    count,
                    dx,
                    dy,
                    rotation,
                    pivot_x,
                    pivot_y,
                    separate_copies,
                );
            }
            "path_duplicate_grid" => {
                if let Some(stroke) = pending_stroke.take() {
                    draw_stroke(canvas, &paths, &stroke);
                }
                let tokens = data.split_whitespace().collect::<Vec<_>>();
                let width = parse_optional_f32(&tokens, 0, 200.0, "grid width")?;
                let height = parse_optional_f32(&tokens, 1, 200.0, "grid height")?;
                let x_count = parse_optional_i32(&tokens, 2, 3, "grid x count")?.max(1) as usize;
                let y_count = parse_optional_i32(&tokens, 3, 3, "grid y count")?.max(1) as usize;
                let column_order = parse_optional_i32(&tokens, 4, 0, "grid order")? != 0;
                let separate_copies =
                    parse_optional_i32(&tokens, 5, 1, "grid separate copies")? != 0;
                paths = apply_path_duplicate_grid(
                    &paths,
                    width,
                    height,
                    x_count,
                    y_count,
                    column_order,
                    separate_copies,
                );
            }
            "path_duplicate_random" => {
                if let Some(stroke) = pending_stroke.take() {
                    draw_stroke(canvas, &paths, &stroke);
                }
                let tokens = data.split_whitespace().collect::<Vec<_>>();
                let count = parse_optional_i32(&tokens, 0, 10, "random count")?.max(1) as usize;
                let spread_x = parse_optional_f32(&tokens, 1, 400.0, "random x spread")?;
                let spread_y = parse_optional_f32(&tokens, 2, 400.0, "random y spread")?;
                let seed = parse_optional_i32(&tokens, 3, 0, "random seed")? as u32;
                let separate_copies =
                    parse_optional_i32(&tokens, 4, 1, "random separate copies")? != 0;
                paths = apply_path_duplicate_random(
                    &paths,
                    count,
                    spread_x,
                    spread_y,
                    seed,
                    separate_copies,
                );
            }
            "path_transform" => {
                if let Some(stroke) = pending_stroke.take() {
                    draw_stroke(canvas, &paths, &stroke);
                }
                let tokens = data.split_whitespace().collect::<Vec<_>>();
                let dx = parse_optional_f32(&tokens, 0, 0.0, "path transform x offset")?;
                let dy = parse_optional_f32(&tokens, 1, 0.0, "path transform y offset")?;
                let scale = parse_optional_f32(&tokens, 2, 1.0, "path transform scale")?;
                let rotation = parse_optional_f32(&tokens, 3, 0.0, "path transform rotation")?;
                let pivot_x = parse_optional_f32(&tokens, 4, 0.0, "path transform pivot x")?;
                let pivot_y = parse_optional_f32(&tokens, 5, 0.0, "path transform pivot y")?;
                paths = apply_path_transform(&paths, dx, dy, scale, rotation, pivot_x, pivot_y);
            }
            "fill" => {
                if let Some(stroke) = pending_stroke.take() {
                    draw_stroke(canvas, &paths, &stroke);
                }
                if !paths.is_empty() {
                    let mut paint = Paint::default();
                    paint.set_anti_alias(true);
                    paint.set_style(PaintStyle::Fill);
                    paint.set_color(parse_fill_color(data)?);
                    for path in &paths {
                        canvas.draw_path(path, &paint);
                    }
                }
            }
            _ => {}
        }
    }

    if let Some(stroke) = pending_stroke.take() {
        draw_stroke(canvas, &paths, &stroke);
    }

    Ok(())
}
