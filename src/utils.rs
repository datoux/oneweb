use anyhow::Result;

pub fn parse_time(datetime: &str) -> Result<f64> {
    let format = "%Y-%m-%d %H:%M:%S%.3f";
    if datetime.ends_with(" Z") {
        // Remove the 'Z' at the end
        let datetime = &datetime[..datetime.len() - 2];
        return parse_time(datetime);
    }

    Ok(chrono::NaiveDateTime::parse_from_str(datetime, format)?
        .and_utc()
        .timestamp_millis() as f64
        / 1000.0)
}

#[allow(dead_code)]
pub fn print_buff_hex(buff: &[u8]) {
    let mut s = String::new();
    for b in buff {
        s.push_str(&format!("{:02X} ", b));
    }
    println!("{}", s);
}

// pub trait LevelFromU32 {
//     fn from_u32(value: u32) -> log::Level;
// }

// impl LevelFromU32 for log::Level {
//     fn from_u32(value: u32) -> log::Level {
//         match value {
//             0 => log::Level::Error,
//             1 => log::Level::Warn,
//             2 => log::Level::Info,
//             3 => log::Level::Debug,
//             4 => log::Level::Trace,
//             _ => log::Level::Info,
//         }
//     }
// }

#[allow(dead_code)]
pub fn nearly_equal(a: f64, b: f64) -> bool {
    let abs_a = a.abs();
    let abs_b = b.abs();
    let diff = (a - b).abs();

    if a == b {
        // Handle infinities.
        true
    } else if a == 0.0 || b == 0.0 || diff < f64::MIN_POSITIVE {
        // One of a or b is zero (or both are extremely close to it,) use absolute error.
        let res = diff < (f64::EPSILON * f64::MIN_POSITIVE);
        if !res {
            println!("a: {}, b: {}, diff: {}", a, b, diff);
        }
        res
    } else {
        // Use relative error.
        let res = (diff / f64::min(abs_a + abs_b, f64::MAX)) < f64::EPSILON;
        if !res {
            println!("a: {}, b: {}, diff: {}", a, b, diff);
        }
        res
    }
}

// pub fn load_ascii_matrix<T: std::str::FromStr>(file_path: &Path) -> Result<Vec<T>> {
//     let mut matrix: Vec<T> = Vec::new();
//     let content = fs::read_to_string(file_path).context(format!(
//         "Cannot load matrix from file {}",
//         file_path.to_string_lossy()
//     ))?;

//     content.lines().for_each(|line| {
//         line.split_whitespace().for_each(|s| {
//             if let Ok(value) = s.parse::<T>() {
//                 matrix.push(value);
//             }
//         });
//     });
//     Ok(matrix)
// }

// #[allow(dead_code)]
// pub fn save_ascii_matrix<T: std::fmt::Display>(
//     file_path: &Path,
//     matrix: &Vec<T>,
//     columns: u32,
// ) -> Result<()> {
//     let mut content = String::new();
//     for (i, value) in matrix.iter().enumerate() {
//         content.push_str(&format!("{} ", value));
//         if (i + 1) % columns as usize == 0 {
//             content.push_str("\n");
//         } else {
//             content.push_str(" ");
//         }
//     }
//     fs::write(file_path, &content).context(format!(
//         "Cannot save matrix to file {}",
//         &file_path.to_string_lossy()
//     ))
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_time() {
        let datetime = "2023-10-01 12:34:56.789";
        let result = parse_time(datetime).unwrap();
        assert_eq!(result, 1696163696.789);

        let datetime = "2023-10-01 12:34:56.789 Z";
        let result = parse_time(datetime).unwrap();
        assert_eq!(result, 1696163696.789);
    }
}
