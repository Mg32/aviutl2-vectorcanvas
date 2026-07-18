use aviutl2::AnyResult;
use skia_safe::Matrix;

use super::util::invalid_input;

fn parse_transform_numbers(args: &str) -> AnyResult<Vec<f32>> {
    args.replace(',', " ")
        .split_whitespace()
        .map(|value| {
            value.parse::<f32>().map_err(|_| {
                invalid_input(format!("transform value is not a number: {value}")).into()
            })
        })
        .collect()
}

fn expect_arg_count(name: &str, args: &[f32], min: usize, max: usize) -> AnyResult<()> {
    if args.len() >= min && args.len() <= max {
        return Ok(());
    }

    let mes = format!("{name} expects {min}..{max} arguments, got {}", args.len());
    Err(invalid_input(mes).into())
}

pub(crate) fn apply_svg_transform(canvas: &skia_safe::Canvas, data: &str) -> AnyResult<()> {
    let mut remain = data.trim();

    while !remain.is_empty() {
        remain = remain.trim_start();

        // get command name
        let Some(open_index) = remain.find('(') else {
            return Err(invalid_input(format!("invalid transform: {remain}")).into());
        };
        let name = remain[..open_index].trim();

        // get argument string
        let after_open = &remain[open_index + 1..];
        let Some(close_index) = after_open.find(')') else {
            return Err(invalid_input(format!("transform is missing ')': {remain}")).into());
        };
        let args = parse_transform_numbers(&after_open[..close_index])?;
        remain = &after_open[close_index + 1..];

        // apply command
        match name {
            "matrix" => {
                expect_arg_count(name, &args, 6, 6)?;
                let matrix = Matrix::new_all(
                    args[0], args[2], args[4], args[1], args[3], args[5], 0.0, 0.0, 1.0,
                );
                canvas.concat(&matrix);
            }
            "translate" => {
                expect_arg_count(name, &args, 1, 2)?;
                canvas.translate((args[0], args.get(1).copied().unwrap_or(0.0)));
            }
            "scale" => {
                expect_arg_count(name, &args, 1, 2)?;
                canvas.scale((args[0], args.get(1).copied().unwrap_or(args[0])));
            }
            "rotate" => {
                expect_arg_count(name, &args, 1, 3)?;
                if args.len() == 3 {
                    canvas.rotate(args[0], Some((args[1], args[2]).into()));
                } else {
                    canvas.rotate(args[0], None);
                }
            }
            "skewX" => {
                expect_arg_count(name, &args, 1, 1)?;
                canvas.skew((args[0].to_radians().tan(), 0.0));
            }
            "skewY" => {
                expect_arg_count(name, &args, 1, 1)?;
                canvas.skew((0.0, args[0].to_radians().tan()));
            }
            _ => {
                return Err(
                    invalid_input(format!("unsupported transform function: {name}")).into(),
                );
            }
        }
    }

    Ok(())
}
