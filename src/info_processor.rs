use crate::utils::parse_time;
use anyhow::{Result, bail};
use std::io::{self, BufRead};

#[allow(dead_code)]
#[derive(Debug, Default, Clone)]
pub struct MeasInfoData {
    pub timestamp: f64,
    pub temp: f64,
    pub pixel_short: f64,
    pub pixel_long: f64,
    pub pixel_saved: f64,
    pub pixel_not_saved: f64,
    pub error_id: String,
}

#[allow(dead_code)]
pub struct MeasInfoProcessor {}

#[allow(dead_code)]
impl MeasInfoProcessor {
    pub fn new() -> MeasInfoProcessor {
        MeasInfoProcessor {}
    }

    fn parse_line(line: &str) -> Result<MeasInfoData> {
        // TIMESTAMP,Temp,N°pixel_short,N°pixel_long,N°pixel_saved,N°pixel_not_saved,Error_id
        //2024-03-01 00:00:51.297,-4,5,35,320,0,
        let parts: Vec<&str> = line.trim().splitn(7, ',').collect();
        if parts.len() != 7 {
            bail!("Invalid line format");
        }

        let timestamp = parse_time(parts[0])?;
        let temp: i32 = parts[1].parse()?;
        let pixel_short: i32 = parts[2].parse()?;
        let pixel_long: i32 = parts[3].parse()?;
        let pixel_saved: i32 = parts[4].parse()?;
        let pixel_not_saved: i32 = parts[5].parse()?;
        let error_id: String = parts[6].to_string();
        Ok(MeasInfoData {
            timestamp,
            temp: temp as f64,
            pixel_short: pixel_short as f64,
            pixel_long: pixel_long as f64,
            pixel_saved: pixel_saved as f64,
            pixel_not_saved: pixel_not_saved as f64,
            error_id: error_id,
        })
    }

    pub fn get_next_meas_info<R>(&self, reader: &mut io::BufReader<R>) -> Result<MeasInfoData>
    where
        R: io::Read,
    {
        for line in reader.lines() {
            let line = line?;
            let line = line.trim();
            if line.starts_with("TIMESTAMP") {
                continue; // Skip header line
            }

            return Ok(MeasInfoProcessor::parse_line(&line)?);
        }
        bail!("No more info data available");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_parse_line() {
        let line = "2024-03-01 00:00:51.297,-4,5,35,320,0,";
        let info_data = MeasInfoProcessor::parse_line(line).unwrap();
        assert_eq!(info_data.timestamp, 1709251251.297);
        assert_eq!(info_data.temp, -4.0);
        assert_eq!(info_data.pixel_short, 5.0);
        assert_eq!(info_data.pixel_long, 35.0);
        assert_eq!(info_data.pixel_saved, 320.0);
        assert_eq!(info_data.pixel_not_saved, 0.0);
        assert_eq!(info_data.error_id, "");

        let line = "2024-03-01 04:28:46.297,-3,172,3614,1396,0,\"255, 255, 255, 255, 255, 255, 31, 32, 32, 64, 64\"";
        let info_data = MeasInfoProcessor::parse_line(line).unwrap();
        assert_eq!(info_data.timestamp, 1709267326.297);
        assert_eq!(info_data.temp, -3.0);
        assert_eq!(info_data.pixel_short, 172.0);
        assert_eq!(info_data.pixel_long, 3614.0);
        assert_eq!(info_data.pixel_saved, 1396.0);
        assert_eq!(info_data.pixel_not_saved, 0.0);
        assert_eq!(
            info_data.error_id,
            "\"255, 255, 255, 255, 255, 255, 31, 32, 32, 64, 64\""
        );
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
        let gps_processor = MeasInfoProcessor::new();
        let info_data = gps_processor.get_next_meas_info(&mut reader).unwrap();
        assert_eq!(info_data.timestamp, 1709251209.0);
    }
}
