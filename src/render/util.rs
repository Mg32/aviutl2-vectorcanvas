use aviutl2::AnyResult;
use skia_safe::Color;

pub(crate) fn invalid_input(message: impl Into<String>) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidInput, message.into())
}

pub(crate) fn parse_optional_f32(
    tokens: &[&str],
    index: usize,
    default: f32,
    name: &str,
) -> AnyResult<f32> {
    tokens
        .get(index)
        .map(|value| {
            value
                .parse::<f32>()
                .map_err(|_| invalid_input(format!("{name} is not a number: {value}")).into())
        })
        .unwrap_or(Ok(default))
}

pub(crate) fn parse_optional_i32(
    tokens: &[&str],
    index: usize,
    default: i32,
    name: &str,
) -> AnyResult<i32> {
    tokens
        .get(index)
        .map(|value| {
            value
                .parse::<i32>()
                .map_err(|_| invalid_input(format!("{name} is not an integer: {value}")).into())
        })
        .unwrap_or(Ok(default))
}

pub(crate) fn parse_bool_flag(value: &str, name: &str) -> AnyResult<bool> {
    match value {
        "0" => Ok(false),
        "1" => Ok(true),
        _ => Err(invalid_input(format!("{name} must be 0 or 1: {value}")).into()),
    }
}

pub(crate) fn parse_hex_color(value: &str) -> AnyResult<Color> {
    let hex = value.trim().strip_prefix('#').unwrap_or(value.trim());
    let rgb = u32::from_str_radix(hex, 16)
        .map_err(|_| invalid_input(format!("invalid color: {value}")))?;
    let r = ((rgb >> 16) & 0xff) as u8;
    let g = ((rgb >> 8) & 0xff) as u8;
    let b = (rgb & 0xff) as u8;
    Ok(Color::from_argb(255, r, g, b))
}

pub(crate) fn parse_fill_color(data: &str) -> AnyResult<Color> {
    let tokens = data.split_whitespace().collect::<Vec<_>>();
    let color = parse_hex_color(
        tokens
            .first()
            .ok_or_else(|| invalid_input("fill color is missing"))?,
    )?;
    let opacity = parse_optional_f32(&tokens, 1, 1.0, "fill opacity")?;
    Ok(color_with_opacity(color, opacity))
}

pub(crate) fn color_with_opacity(color: Color, opacity: f32) -> Color {
    let alpha = (opacity.clamp(0.0, 1.0) * 255.0).round() as u8;
    Color::from_argb(alpha, color.r(), color.g(), color.b())
}
