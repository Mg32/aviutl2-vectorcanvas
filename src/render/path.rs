use aviutl2::AnyResult;
use skia_safe::{Matrix, Path, PathBuilder, PathEffect, PathOp, Rect, StrokeRec};

use super::util::invalid_input;

pub(crate) fn create_path(data: &str) -> AnyResult<Path> {
    Path::from_svg(data)
        .ok_or_else(|| invalid_input(format!("invalid SVG path data: {data}")).into())
}

pub(crate) fn parse_boolean_op(data: &str) -> AnyResult<PathOp> {
    match data.trim() {
        "OR" => Ok(PathOp::Union),
        "AND" => Ok(PathOp::Intersect),
        "XOR" => Ok(PathOp::XOR),
        value => Err(invalid_input(format!("unsupported boolean operation: {value}")).into()),
    }
}

pub(crate) fn apply_boolean(paths: &[Path], op: PathOp) -> Vec<Path> {
    let mut iter = paths.iter();
    let Some(first) = iter.next() else {
        return Vec::new();
    };

    let mut result = first.clone();
    for path in iter {
        let Some(next) = result.op(path, op) else {
            continue;
        };
        result = next;
    }

    vec![
        result
            .simplify()
            .or_else(|| result.as_winding())
            .unwrap_or(result),
    ]
}

fn duplicate_paths(paths: &[Path], matrices: &[Matrix], separate_copies: bool) -> Vec<Path> {
    if paths.is_empty() || matrices.is_empty() {
        return paths.to_vec();
    }

    if separate_copies {
        let mut results = Vec::with_capacity(matrices.len());
        for matrix in matrices {
            let mut builder = PathBuilder::new();
            for path in paths {
                builder.add_path(&path.with_transform(matrix), None);
            }
            let path = builder.detach();
            if !path.is_empty() {
                results.push(path);
            }
        }
        return results;
    }

    let mut builder = PathBuilder::new();
    for matrix in matrices {
        for path in paths {
            builder.add_path(&path.with_transform(matrix), None);
        }
    }
    let path = builder.detach();
    if path.is_empty() {
        paths.to_vec()
    } else {
        vec![path]
    }
}

fn array_duplicate_matrices(
    count: usize,
    dx: f32,
    dy: f32,
    rotation: f32,
    pivot_x: f32,
    pivot_y: f32,
) -> Vec<Matrix> {
    let mut matrices = Vec::with_capacity(count);
    for index in 0..count {
        let index = index as f32;
        let angle = (rotation * index).to_radians();
        let (sin, cos) = angle.sin_cos();
        let offset_x = dx * index;
        let offset_y = dy * index;
        matrices.push(Matrix::new_all(
            cos,
            -sin,
            pivot_x - cos * pivot_x + sin * pivot_y + cos * offset_x - sin * offset_y,
            sin,
            cos,
            pivot_y - sin * pivot_x - cos * pivot_y + sin * offset_x + cos * offset_y,
            0.0,
            0.0,
            1.0,
        ));
    }
    matrices
}

pub(crate) fn apply_path_duplicate_array(
    paths: &[Path],
    count: usize,
    dx: f32,
    dy: f32,
    rotation: f32,
    pivot_x: f32,
    pivot_y: f32,
    separate_copies: bool,
) -> Vec<Path> {
    if count <= 1
        || !dx.is_finite()
        || !dy.is_finite()
        || !rotation.is_finite()
        || !pivot_x.is_finite()
        || !pivot_y.is_finite()
    {
        return paths.to_vec();
    }

    let matrices = array_duplicate_matrices(count, dx, dy, rotation, pivot_x, pivot_y);
    duplicate_paths(paths, &matrices, separate_copies)
}

pub(crate) fn apply_path_duplicate_grid(
    paths: &[Path],
    width: f32,
    height: f32,
    x_count: usize,
    y_count: usize,
    column_order: bool,
    separate_copies: bool,
) -> Vec<Path> {
    if x_count == 0 || y_count == 0 || !width.is_finite() || !height.is_finite() {
        return paths.to_vec();
    }

    let total = x_count.saturating_mul(y_count);
    let mut matrices = Vec::with_capacity(total);
    let x_offset = |x_index: usize| {
        if x_count <= 1 {
            0.0
        } else {
            width * (x_index as f32 / (x_count - 1) as f32 - 0.5)
        }
    };
    let y_offset = |y_index: usize| {
        if y_count <= 1 {
            0.0
        } else {
            height * (y_index as f32 / (y_count - 1) as f32 - 0.5)
        }
    };

    if column_order {
        for x_index in 0..x_count {
            for y_index in 0..y_count {
                matrices.push(Matrix::translate((x_offset(x_index), y_offset(y_index))));
            }
        }
    } else {
        for y_index in 0..y_count {
            for x_index in 0..x_count {
                matrices.push(Matrix::translate((x_offset(x_index), y_offset(y_index))));
            }
        }
    }

    duplicate_paths(paths, &matrices, separate_copies)
}

fn random_generator(seed: &mut u32) -> f32 {
    *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    ((*seed >> 8) as f32) / 16_777_215.0
}

pub(crate) fn apply_path_duplicate_random(
    paths: &[Path],
    count: usize,
    spread_x: f32,
    spread_y: f32,
    seed: u32,
    separate_copies: bool,
) -> Vec<Path> {
    if count <= 1 || !spread_x.is_finite() || !spread_y.is_finite() {
        return paths.to_vec();
    }

    let mut state = if seed == 0 { 1 } else { seed };
    let mut matrices = Vec::with_capacity(count);
    for index in 0..count {
        if index == 0 {
            matrices.push(Matrix::new_identity());
            continue;
        }
        let x = (random_generator(&mut state) - 0.5) * spread_x;
        let y = (random_generator(&mut state) - 0.5) * spread_y;
        matrices.push(Matrix::translate((x, y)));
    }

    duplicate_paths(paths, &matrices, separate_copies)
}

pub(crate) fn apply_path_transform(
    paths: &[Path],
    dx: f32,
    dy: f32,
    scale: f32,
    rotation: f32,
    pivot_x: f32,
    pivot_y: f32,
) -> Vec<Path> {
    if paths.is_empty()
        || !dx.is_finite()
        || !dy.is_finite()
        || !scale.is_finite()
        || !rotation.is_finite()
        || !pivot_x.is_finite()
        || !pivot_y.is_finite()
    {
        return paths.to_vec();
    }

    let angle = rotation.to_radians();
    let (sin, cos) = angle.sin_cos();
    let matrix = Matrix::new_all(
        scale * cos,
        -scale * sin,
        dx + pivot_x - scale * cos * pivot_x + scale * sin * pivot_y,
        scale * sin,
        scale * cos,
        dy + pivot_y - scale * sin * pivot_x - scale * cos * pivot_y,
        0.0,
        0.0,
        1.0,
    );

    paths
        .iter()
        .map(|path| path.with_transform(&matrix))
        .collect()
}

pub(crate) fn apply_round(paths: &[Path], radius: f32) -> Vec<Path> {
    if radius <= 0.0 || !radius.is_finite() {
        return paths.to_vec();
    }

    let Some(effect) = PathEffect::corner_path(radius) else {
        return paths.to_vec();
    };

    let cull = Rect::new(-1_000_000.0, -1_000_000.0, 1_000_000.0, 1_000_000.0);
    let mut results = Vec::new();
    for path in paths {
        let stroke_rec = StrokeRec::new_fill();
        if let Some((mut builder, _)) = effect.filter_path(path, &stroke_rec, cull) {
            let filtered = builder.detach();
            if !filtered.is_empty() {
                results.push(filtered);
            }
        }
    }

    if results.is_empty() {
        return paths.to_vec();
    }
    results
}

pub(crate) fn apply_simplify(paths: &[Path]) -> Vec<Path> {
    paths
        .iter()
        .map(|path| path.simplify().unwrap_or_else(|| path.clone()))
        .collect()
}

pub(crate) fn apply_path_noise(
    paths: &[Path],
    segment_length: f32,
    deviation: f32,
    seed: u32,
    sketch_count: usize,
) -> Vec<Path> {
    if segment_length <= 0.0
        || deviation <= 0.0
        || !segment_length.is_finite()
        || !deviation.is_finite()
    {
        return paths.to_vec();
    }

    let count = sketch_count.max(1);
    let mut results = Vec::new();
    let cull = Rect::new(-1_000_000.0, -1_000_000.0, 1_000_000.0, 1_000_000.0);

    for path in paths {
        for index in 0..count {
            let effect_seed = seed.wrapping_add(index as u32);
            let Some(effect) = PathEffect::discrete(segment_length, deviation, effect_seed) else {
                continue;
            };
            let stroke_rec = StrokeRec::new_fill();
            if let Some((mut builder, _)) = effect.filter_path(path, &stroke_rec, cull) {
                let noisy = builder.detach();
                if !noisy.is_empty() {
                    results.push(noisy);
                }
            }
        }
    }

    if results.is_empty() {
        return paths.to_vec();
    }
    results
}
