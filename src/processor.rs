use crate::data_processor::{DataProcessor, Frame};
use crate::gps_processor::{GpsData, GpsProcessor};
use crate::info_processor::{MeasInfoData, MeasInfoProcessor};
use anyhow::{Result, bail};
use chrono::{self, TimeZone};
use std::env;
use std::io::prelude::*;
use std::path::Path;

pub struct Processor {
    last_gps_data: GpsData,
    last_info_data: MeasInfoData,
    frame_index: usize,
    lend: String,
}

impl Processor {
    pub fn new() -> Self {
        Processor {
            last_gps_data: GpsData {
                ..Default::default()
            },
            last_info_data: MeasInfoData {
                ..Default::default()
            },
            frame_index: 0,
            lend: if env::consts::OS == "windows" {
                String::from("\r\n")
            } else {
                String::from("\n")
            },
        }
    }

    fn find_next_closest_gps_data(
        &mut self,
        proc: &GpsProcessor,
        reader: &mut std::io::BufReader<std::fs::File>,
        timestamp: f64,
    ) -> Result<GpsData> {
        loop {
            let last_data = self.last_gps_data.clone();

            if let Ok(data) = proc.get_next_gps_data(reader) {
                let diff_last = (last_data.timestamp - timestamp).abs();
                let diff_cur = (data.timestamp - timestamp).abs();
                self.last_gps_data = data.clone();

                if data.timestamp < timestamp {
                    continue;
                }

                if diff_last < diff_cur {
                    return Ok(last_data);
                } else {
                    return Ok(data);
                }
            } else {
                // If we reach the end of the file, return the last GPS data
                if last_data.timestamp > 0.0 {
                    self.last_gps_data.timestamp = 0.0;
                    return Ok(last_data);
                } else {
                    bail!("No more data available");
                }
            }
        }
    }

    fn find_next_closest_info_data(
        &mut self,
        proc: &MeasInfoProcessor,
        reader: &mut std::io::BufReader<std::fs::File>,
        timestamp: f64,
    ) -> Result<MeasInfoData> {
        loop {
            let last_data = self.last_info_data.clone();

            if let Ok(data) = proc.get_next_meas_info(reader) {
                let diff_last = (last_data.timestamp - timestamp).abs();
                let diff_cur = (data.timestamp - timestamp).abs();
                self.last_info_data = data.clone();

                if data.timestamp < timestamp {
                    continue;
                }

                if diff_last < diff_cur {
                    return Ok(last_data);
                } else {
                    return Ok(data);
                }
            } else {
                // If we reach the end of the file, return the last GPS data
                if last_data.timestamp > 0.0 {
                    self.last_gps_data.timestamp = 0.0;
                    return Ok(last_data);
                } else {
                    bail!("No mor edata available");
                }
            }
        }
    }

    fn calculate_acq_time(info_data: &MeasInfoData, max_pix_count: usize) -> f64 {
        let pix_short = info_data.pixel_short as f64;
        let pix_long = info_data.pixel_long as f64;
        let time_short = 0.1;
        let time_long = 1.0;
        let a = (pix_long - pix_short) / (time_long - time_short);
        let b = pix_long - a * time_long;
        let mut acq_time = if a != 0.0 {
            (max_pix_count as f64 - b) / a
        } else {
            0.0
        };
        if acq_time > 25.0 {
            acq_time = 25.0;
        }
        acq_time
    }

    fn fmt_acq_time(acq_time: f64) -> String {
        let acq_time_fmt = format!("{:.6}", acq_time);
        if acq_time_fmt.contains('.') {
            acq_time_fmt
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_string()
        } else {
            acq_time_fmt
        }
    }

    fn save_frame_to_clusterlog<R>(
        &mut self,
        frame: &Frame,
        info_data: &MeasInfoData,
        acq_time: f64,
        writer: &mut std::io::BufWriter<R>,
    ) -> Result<()>
    where
        R: std::io::Write,
    {
        //Frame 1 (1484036406.350515, 85.762486 s)
        write!(
            writer,
            "Frame {} ({}, {} s){}",
            self.frame_index + 1,
            info_data.timestamp,
            Self::fmt_acq_time(acq_time),
            &self.lend,
        )?;

        for cluster in &frame.clusters {
            for pix in &cluster.pixels {
                write!(
                    writer,
                    "[{}, {}, {}, {}] ",
                    pix.x, pix.y, pix.value, pix.value2
                )?;
            }
            write!(writer, "{}", &self.lend)?;
        }
        write!(writer, "{}", self.lend)?;

        Ok(())
    }

    fn save_metadata<R>(
        &mut self,
        frame: &Frame,
        info_data: &MeasInfoData,
        gps_data: &GpsData,
        writer: &mut std::io::BufWriter<R>,
    ) -> Result<()>
    where
        R: std::io::Write,
    {
        if self.frame_index == 0 {
            write!(
                writer,
                "Frame Index\tTimestamp\tFrame Timestamp\tTemp\tGPS J2000 X\tGPS J2000 Y\tGPS J2000 Z\tGPS Q Scalar\tGPS Q Vector 1\tGPS Q Vector 2\tGPS Q Vector 3{}",
                self.lend,
            )?;
        }
        write!(
            writer,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}{}",
            self.frame_index + 1,
            info_data.timestamp,
            frame.timestamp,
            info_data.temp,
            gps_data.j2000_x,
            gps_data.j2000_y,
            gps_data.j2000_z,
            gps_data.q_est_prop_bj_scalar,
            gps_data.q_est_prop_bj_vector_1,
            gps_data.q_est_prop_bj_vector_2,
            gps_data.q_est_prop_bj_vector_3,
            self.lend,
        )?;
        Ok(())
    }

    fn save_to_files<R>(
        &mut self,
        frame: &Frame,
        info_data: &MeasInfoData,
        gps_data: &GpsData,
        acq_time: f64,
        clog_writer: &mut std::io::BufWriter<R>,
        meta_writer: &mut std::io::BufWriter<R>,
    ) -> Result<()>
    where
        R: std::io::Write,
    {
        self.save_frame_to_clusterlog(&frame, &info_data, acq_time, clog_writer)?;
        self.save_metadata(&frame, &info_data, &gps_data, meta_writer)?;
        self.frame_index += 1;
        Ok(())
    }

    pub fn process_files(
        &mut self,
        gps_file: &str,
        meas_file: &str,
        data_file: &str,
        out_dir: &str,
        max_pix_count: usize,
    ) -> Result<(), anyhow::Error> {
        let gps_processor = GpsProcessor::new();
        let info_processor = MeasInfoProcessor::new();
        let mut data_processor = DataProcessor::new();

        let gps_file = std::fs::File::open(gps_file)?;
        let meas_file = std::fs::File::open(meas_file)?;
        let data_file = std::fs::File::open(data_file)?;
        let mut gps_reader = std::io::BufReader::new(gps_file);
        let mut meas_reader = std::io::BufReader::new(meas_file);
        let mut data_reader = std::io::BufReader::new(data_file);
        let mut clog_write: Option<std::io::BufWriter<std::fs::File>> = None;
        let mut meta_write: Option<std::io::BufWriter<std::fs::File>> = None;

        let dir_path = Path::new(out_dir);
        let mut idx = 0;
        let mut date = String::from("");

        loop {
            let frame = data_processor.get_next_frame(&mut data_reader)?;

            let gps_data =
                self.find_next_closest_gps_data(&gps_processor, &mut gps_reader, frame.timestamp)?;

            let info_data = self.find_next_closest_info_data(
                &info_processor,
                &mut meas_reader,
                frame.timestamp,
            )?;

            idx += 1;

            let info_date = chrono::Utc
                .timestamp_opt(info_data.timestamp as i64, 0 as u32)
                .unwrap();

            let cur_date = info_date.format("%Y-%m-%d").to_string();
            let acq_time = Self::calculate_acq_time(&info_data, max_pix_count);

            if clog_write.is_none() || meta_write.is_none() || date != cur_date {
                // Reuse existing files
                self.frame_index = 0;
                let time_suffix = info_date.format("%Y-%m-%d").to_string();
                let clog_file_path = dir_path.join(format!("data_{}.clog", time_suffix));
                let meta_file_path = dir_path.join(format!("data_{}.info", time_suffix));
                let clog_file = std::fs::File::create(&clog_file_path)?;
                let meta_file = std::fs::File::create(&meta_file_path)?;
                clog_write = Some(std::io::BufWriter::new(clog_file));
                meta_write = Some(std::io::BufWriter::new(meta_file));
                date = cur_date;
            }

            if clog_write.is_some() && meta_write.is_some() {
                let clog_writer = clog_write.as_mut().unwrap();
                let meta_writer = meta_write.as_mut().unwrap();
                self.save_to_files(
                    &frame,
                    &info_data,
                    &gps_data,
                    acq_time,
                    clog_writer,
                    meta_writer,
                )?;
            }

            println!(
                "Processing frame {} ({}, {} s) ...",
                idx,
                info_date,
                Self::fmt_acq_time(acq_time)
            );
        }
    }
}
