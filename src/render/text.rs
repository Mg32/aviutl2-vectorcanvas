use aviutl2::AnyResult;
use skia_safe::{Font, FontMgr, FontStyle, Matrix, Path, PathBuilder};

use super::util::{invalid_input, parse_bool_flag};

pub(crate) struct TextPathCommand {
    x: f32,
    y: f32,
    size: f32,
    letter_spacing: f32,
    line_spacing: f32,
    font_name: String,
    bold: bool,
    italic: bool,
    separate_paths: bool,
    align: TextAlign,
    text: String,
}

#[derive(Clone, Copy)]
enum HorizontalAlign {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy)]
enum VerticalAlign {
    Top,
    Middle,
    Bottom,
}

#[derive(Clone, Copy)]
struct TextAlign {
    horizontal: HorizontalAlign,
    vertical: VerticalAlign,
}

fn parse_text_align(value: &str) -> AnyResult<TextAlign> {
    let index = value
        .parse::<usize>()
        .map_err(|_| invalid_input(format!("text align is not a number: {value}")))?;
    let horizontal = match index % 3 {
        0 => HorizontalAlign::Left,
        1 => HorizontalAlign::Center,
        2 => HorizontalAlign::Right,
        _ => unreachable!(),
    };
    let vertical = match index / 3 {
        0 => VerticalAlign::Top,
        1 => VerticalAlign::Middle,
        2 => VerticalAlign::Bottom,
        _ => return Err(invalid_input(format!("text align is out of range: {value}")).into()),
    };
    Ok(TextAlign {
        horizontal,
        vertical,
    })
}

fn unescape_text(value: &str) -> AnyResult<String> {
    let mut result = String::new();
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            result.push(ch);
            continue;
        }
        match chars.next() {
            Some('n') => result.push('\n'),
            Some('t') => result.push('\t'),
            Some('\\') => result.push('\\'),
            Some(other) => {
                return Err(invalid_input(format!("unsupported text escape: \\{other}")).into());
            }
            None => return Err(invalid_input("text escape is incomplete").into()),
        }
    }
    Ok(result)
}

pub(crate) fn parse_text_path_command(data: &str) -> AnyResult<TextPathCommand> {
    let fields = data.splitn(11, '\t').collect::<Vec<_>>();
    if fields.len() < 10 {
        return Err(
            invalid_input(format!("text path expects 10 fields, got {}", fields.len())).into(),
        );
    }
    let (align, text_field) = if fields.len() >= 11 {
        (parse_text_align(fields[9])?, fields[10])
    } else {
        (
            TextAlign {
                horizontal: HorizontalAlign::Left,
                vertical: VerticalAlign::Top,
            },
            fields[9],
        )
    };

    Ok(TextPathCommand {
        x: fields[0]
            .parse()
            .map_err(|_| invalid_input(format!("text x is not a number: {}", fields[0])))?,
        y: fields[1]
            .parse()
            .map_err(|_| invalid_input(format!("text y is not a number: {}", fields[1])))?,
        size: fields[2]
            .parse::<f32>()
            .map_err(|_| invalid_input(format!("text size is not a number: {}", fields[2])))?
            .max(0.0),
        letter_spacing: fields[3]
            .parse()
            .map_err(|_| invalid_input(format!("letter spacing is not a number: {}", fields[3])))?,
        line_spacing: fields[4]
            .parse()
            .map_err(|_| invalid_input(format!("line spacing is not a number: {}", fields[4])))?,
        font_name: fields[5].to_string(),
        bold: parse_bool_flag(fields[6], "bold")?,
        italic: parse_bool_flag(fields[7], "italic")?,
        separate_paths: parse_bool_flag(fields[8], "separate_paths")?,
        align,
        text: unescape_text(text_field)?,
    })
}

fn glyph_widths(font: &Font, text: &str) -> Vec<(u16, f32)> {
    let glyphs = font.text_to_glyphs_vec(text);
    let mut widths = vec![0.0; glyphs.len()];
    font.get_widths(&glyphs, &mut widths);
    glyphs.into_iter().zip(widths).collect()
}

fn text_line_width(font: &Font, line: &str, letter_spacing: f32) -> f32 {
    let mut width = 0.0;
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        for (_, glyph_width) in glyph_widths(font, ch.to_string().as_str()) {
            width += glyph_width;
        }
        if chars.peek().is_some() {
            width += letter_spacing;
        }
    }
    width
}

pub(crate) fn create_text_paths(command: TextPathCommand) -> Vec<Path> {
    let style = FontStyle::new(
        if command.bold { 700.into() } else { 400.into() },
        5.into(),
        if command.italic {
            skia_safe::font_style::Slant::Italic
        } else {
            skia_safe::font_style::Slant::Upright
        },
    );
    let typeface = FontMgr::new()
        .legacy_make_typeface(
            (!command.font_name.is_empty()).then_some(command.font_name.as_str()),
            style,
        )
        .unwrap_or_else(|| {
            FontMgr::new()
                .legacy_make_typeface(None, style)
                .expect("default typeface should exist")
        });
    let font = Font::new(typeface, command.size);
    let (_, metrics) = font.metrics();
    let default_line_height = metrics.descent - metrics.ascent + metrics.leading;
    let line_height = (default_line_height + command.line_spacing).max(0.0);
    let lines = command.text.split('\n').collect::<Vec<_>>();
    let line_count = lines.len().max(1);
    let block_height =
        (line_count.saturating_sub(1) as f32 * line_height) + metrics.descent - metrics.ascent;
    let first_baseline_y = match command.align.vertical {
        VerticalAlign::Top => command.y - metrics.ascent,
        VerticalAlign::Middle => command.y - metrics.ascent - block_height / 2.0,
        VerticalAlign::Bottom => {
            command.y - (line_count.saturating_sub(1) as f32 * line_height) - metrics.descent
        }
    };

    let mut paths = Vec::new();
    let mut combined = PathBuilder::new();
    let mut pen_y = first_baseline_y;

    for line in lines {
        let line_width = text_line_width(&font, line, command.letter_spacing);
        let mut pen_x = match command.align.horizontal {
            HorizontalAlign::Left => command.x,
            HorizontalAlign::Center => command.x - line_width / 2.0,
            HorizontalAlign::Right => command.x - line_width,
        };
        for ch in line.chars() {
            let ch_text = ch.to_string();
            let mut char_path = PathBuilder::new();
            let mut glyph_x = pen_x;

            for (glyph, width) in glyph_widths(&font, ch_text.as_str()) {
                if let Some(path) = font.get_path(glyph) {
                    let matrix = Matrix::translate((glyph_x, pen_y));
                    let path = path.with_transform(&matrix);
                    if command.separate_paths {
                        char_path.add_path(&path, None);
                    } else {
                        combined.add_path(&path, None);
                    }
                }
                glyph_x += width;
            }

            if command.separate_paths {
                let path = char_path.detach();
                if !path.is_empty() {
                    paths.push(path);
                }
            }
            pen_x = glyph_x + command.letter_spacing;
        }
        pen_y += line_height;
    }

    if !command.separate_paths {
        paths.push(combined.detach());
    }

    paths
}
