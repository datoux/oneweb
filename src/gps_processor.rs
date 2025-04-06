use crate::utils::parse_time;
use anyhow::{Context, Result, bail};
use std::io::{self, BufRead};

#[allow(dead_code)]
#[derive(Debug, Default, Clone)]
pub struct GpsData {
    pub timestamp: f64,
    pub j2000_x: f64,
    pub j2000_y: f64,
    pub j2000_z: f64,
    pub q_est_prop_bj_scalar: f64,
    pub q_est_prop_bj_vector_1: f64,
    pub q_est_prop_bj_vector_2: f64,
    pub q_est_prop_bj_vector_3: f64,
}

#[allow(dead_code)]
pub struct GpsProcessor {}

#[allow(dead_code)]
impl GpsProcessor {
    pub fn new() -> GpsProcessor {
        GpsProcessor {}
    }

    fn parse_line(line: &str) -> Result<GpsData> {
        //"TIME","J2000_X (m)","J2000_Y (m)","J2000_Z (m)","iae_qEstProp_BJ.scalar","iae_qEstProp_BJ.vector(1)","iae_qEstProp_BJ.vector(2)","iae_qEstProp_BJ.vector(3)"
        //2024-03-01 00:00:09.000,2.51279e+6,5.64324e+5,-6.50431e+6,9.64920e-1,5.96500e-3,-1.87169e-1,1.84013e-1

        let parts: Vec<&str> = line.trim().split(',').collect();
        if parts.len() != 8 {
            bail!("Invalid line format");
        }

        let timestamp = parse_time(parts[0]).context(format!("invalid line: {}", &parts[0]))?;
        let j2000_x: f64 = parts[1].parse().unwrap_or(0.0);
        let j2000_y: f64 = parts[2].parse().unwrap_or(0.0);
        let j2000_z: f64 = parts[3].parse().unwrap_or(0.0);
        let q_est_prop_bj_scalar: f64 = parts[4].parse().unwrap_or(0.0);
        let q_est_prop_bj_vector_1: f64 = parts[5].parse().unwrap_or(0.0);
        let q_est_prop_bj_vector_2: f64 = parts[6].parse().unwrap_or(0.0);
        let q_est_prop_bj_vector_3: f64 = parts[7].parse().unwrap_or(0.0);
        Ok(GpsData {
            timestamp,
            j2000_x,
            j2000_y,
            j2000_z,
            q_est_prop_bj_scalar,
            q_est_prop_bj_vector_1,
            q_est_prop_bj_vector_2,
            q_est_prop_bj_vector_3,
        })
    }

    pub fn get_next_gps_data<R>(&self, reader: &mut io::BufReader<R>) -> Result<GpsData>
    where
        R: io::Read,
    {
        for line in reader.lines() {
            let line = line?;
            let line = line.trim();
            if !line.starts_with("20") {
                continue; // Skip header line
            }
            return Ok(
                GpsProcessor::parse_line(&line).context(format!("cannot parse gps: {}", &line))?
            );
        }
        bail!("No more GPS data available");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_parse_line() {
        let line = "2024-03-01 00:00:09.000,2.51279e+6,5.64324e+5,-6.50431e+6,9.64920e-1,5.96500e-3,-1.87169e-1,1.84013e-1";
        let gps_data = GpsProcessor::parse_line(line).unwrap();
        assert_eq!(gps_data.timestamp, 1709251209.0);
        assert_eq!(gps_data.j2000_x, 2.51279e+6);
        assert_eq!(gps_data.j2000_y, 5.64324e+5);
        assert_eq!(gps_data.j2000_z, -6.50431e+6);
        assert_eq!(gps_data.q_est_prop_bj_scalar, 9.64920e-1);
        assert_eq!(gps_data.q_est_prop_bj_vector_1, 5.96500e-3);
        assert_eq!(gps_data.q_est_prop_bj_vector_2, -1.87169e-1);
        assert_eq!(gps_data.q_est_prop_bj_vector_3, 1.84013e-1);
    }

    #[test]
    fn test_get_next_gps_data() {
        let lines = vec![
            "\"TIME\",\"J2000_X (m)\",\"J2000_Y (m)\",\"J2000_Z (m)\",\"iae_qEstProp_BJ.scalar\",\"iae_qEstProp_BJ.vector(1)\",\"iae_qEstProp_BJ.vector(2)\",\"iae_qEstProp_BJ.vector(3)\"",
            "2024-03-01 00:00:09.000,2.51279e+6,5.64324e+5,-6.50431e+6,9.64920e-1,5.96500e-3,-1.87169e-1,1.84013e-1",
        ];
        let data = lines.join("\n");
        let cursor = Cursor::new(data);
        let mut reader = io::BufReader::new(cursor);
        let gps_processor = GpsProcessor::new();
        let gps_data = gps_processor.get_next_gps_data(&mut reader).unwrap();
        assert_eq!(gps_data.timestamp, 1709251209.0);
    }
}
