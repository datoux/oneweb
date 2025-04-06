use std::io::{self, BufRead};

use crate::clustering::{Cluster, Clusterer};
use crate::tpx3lut::{LUT_ITOT, LUT_TOT, MAX_LUT_ITOT, MAX_LUT_TOT, WRONG_LUT_ITOT, WRONG_LUT_TOT};
use crate::utils::{parse_time, print_buff_hex};
use anyhow::{Result, bail};
use hex;

#[allow(dead_code)]
#[derive(Debug)]
pub struct Frame {
    pub itot: Vec<u16>,
    pub event: Vec<u16>,
    pub clusters: Vec<Cluster>,
    pub timestamp: f64,
}

pub struct DataProcessor {
    pub frame_data: Vec<u8>,
    pub skipped_lines: Vec<String>,
    pub timestamp: f64,
    seq_offset: usize,
}

#[allow(dead_code)]
impl DataProcessor {
    pub fn new() -> Self {
        DataProcessor {
            frame_data: Vec::new(),
            skipped_lines: Vec::new(),
            timestamp: 0.0,
            seq_offset: 0,
        }
    }

    fn parse_line(line: &str) -> Result<(f64, Vec<u8>)> {
        let parts: Vec<&str> = line.trim().split(',').collect();
        if parts.len() != 2 {
            bail!("Invalid line format");
        }
        Ok((parse_time(parts[0])?, hex::decode(parts[1])?))
    }

    fn find_sequence_in_data(seq: &[u8], data: &[u8], seq_offset: &mut usize) -> Option<usize> {
        for i in 0..data.len() {
            if seq[*seq_offset] == data[i] {
                *seq_offset += 1;
                if *seq_offset == seq.len() {
                    return Some(i);
                }
            } else {
                *seq_offset = 0;
            }
        }

        None
    }

    fn clear_data(&mut self) {
        self.frame_data.clear();
        self.skipped_lines.clear();
        self.timestamp = 0.0;
        self.seq_offset = 0;
    }

    pub fn process_next_line(&mut self, line: &str) -> Result<bool> {
        let (timestamp, data) = Self::parse_line(line)?;

        if self.frame_data.is_empty() {
            if let Some(index) =
                Self::find_sequence_in_data(&[0x71, 0xAF, 0x00, 0x00], &data, &mut self.seq_offset)
            {
                self.seq_offset = 0;
                self.frame_data.clear();
                self.frame_data.extend_from_slice(&vec![0x71, 0xAF, 0x00]);
                self.frame_data.extend_from_slice(&data[index..]);
                self.timestamp = timestamp;
            } else {
                self.skipped_lines.push(line.to_string());
            }
            return Ok(false);
        }

        if let Some(index) =
            Self::find_sequence_in_data(&[0x71, 0xA0, 0x00, 0x00], &data, &mut self.seq_offset)
        {
            self.seq_offset = 0;
            self.frame_data.extend_from_slice(&data[..=index]);
            return Ok(true);
        } else if let Some(index) =
            Self::find_sequence_in_data(&[0x00, 0x00, 0x00, 0x00], &data, &mut self.seq_offset)
        {
            self.seq_offset = 0;
            self.frame_data.extend_from_slice(&data[..=index]);
            return Ok(true);
        } else {
            self.frame_data.extend_from_slice(&data);
        }

        Ok(false)
    }

    fn parse_pixel_packet(data: &[u8]) -> (u16, u16, u16) {
        let address = (((data[0] as u16) & 0x0F) << 12)
            | ((data[1] as u16) << 4)
            | ((data[2] as u16 >> 4) & 0x0F);
        let toa: u16 = ((data[2] as u16 & 0x0F) << 10)
            | ((data[3] as u16) << 2)
            | ((data[4] as u16 >> 6) & 0x03);
        let event = ((data[4] as u16 & 0x3F) << 4) | ((data[5] as u16 >> 4) & 0x0F);
        // let hit = data[5] & 0x0F;
        let eoc = (address >> 9) & 0x7F;
        let sp = (address >> 3) & 0x3F;
        let pix = address & 0x07;
        let x = eoc * 2 + (pix / 4);
        let y = sp * 4 + (pix % 4);
        let idx = y * 256 + x;

        let itot = if toa >= 1 && toa < MAX_LUT_ITOT as u16 {
            LUT_ITOT[toa as usize]
        } else {
            WRONG_LUT_ITOT
        };

        let event = if event >= 1 && event < MAX_LUT_TOT as u16 {
            LUT_TOT[event as usize]
        } else {
            WRONG_LUT_TOT
        };

        (idx, itot, event)
    }

    pub fn extract_frame(&self) -> Frame {
        let mut fr_itot = vec![0; 256 * 256];
        let mut fr_event = vec![0; 256 * 256];
        let mut bad_data: Vec<u8> = Vec::new();
        let mut bad_data_offset: usize = 0;

        let mut offset = 0;
        while offset < self.frame_data.len() {
            if self.frame_data.len() - offset < 6 {
                // not enough data for a pixel packet
                break;
            }

            if self.frame_data[offset] == 0x71 && self.frame_data[offset + 1] == 0xAF {
                offset += 6;
                continue;
            }

            if self.frame_data[offset] == 0x71 && self.frame_data[offset + 1] == 0xA0 {
                // end of readout
                break;
            }

            if self.frame_data[offset] == 0x14 && self.frame_data[offset + 5] == 0x02 {
                // skip extra header
                // println!(
                //     "skip extra header: {:02X}, offset: {}",
                //     self.frame_data[offset], offset
                // );
                offset += 8;
                continue;
            }

            while offset + 6 < self.frame_data.len()
                && self.frame_data[offset] & 0xF0 != 0xA0
                && self.frame_data[offset + 5] != 0xEE
            {
                bad_data.push(self.frame_data[offset]);
                if bad_data_offset == 0 {
                    bad_data_offset = offset;
                }
                offset += 1;
            }

            if bad_data.len() > 0 {
                print!("unexpected data [{}]: ", bad_data_offset);
                print_buff_hex(&bad_data);
                bad_data.clear();
                bad_data_offset = 0;
                continue;
            }

            let (idx, itot, event) = Self::parse_pixel_packet(&self.frame_data[offset..]);
            // println!("idx: {}, itot: {}, event: {}", idx, itot, event);
            fr_itot[idx as usize] = itot;
            fr_event[idx as usize] = event;

            offset += 6;
        }

        Frame {
            itot: fr_itot,
            event: fr_event,
            clusters: Vec::new(),
            timestamp: self.timestamp,
        }
    }

    pub fn clusterize_frame(&self, frame: &mut Frame) {
        let clusterer = Clusterer::new();
        frame.clusters = clusterer.search_frame(&frame.itot, 256, 256);
    }

    pub fn get_next_frame<R>(&mut self, reader: &mut io::BufReader<R>) -> Result<Frame>
    where
        R: io::Read,
    {
        for line in reader.lines() {
            let line = line?;
            let line = line.trim();
            if line.starts_with("TIMESTAMP") {
                continue; // Skip header line
            }

            let res = self.process_next_line(&line)?;
            if res {
                let mut frame = self.extract_frame();
                self.clusterize_frame(&mut frame);
                self.clear_data();
                return Ok(frame);
            }
        }
        bail!("No more data available");
    }
}

#[cfg(test)]
mod tests {
    use std::io::{BufReader, Cursor};

    use super::*;

    #[test]
    fn test_parse_line() {
        let line = "2023-10-01 12:34:56.789,1234567890abcdef";
        let (timestamp, data) = DataProcessor::parse_line(line).unwrap();
        assert_eq!(timestamp, 1696163696.789);
        assert_eq!(data, &[0x12, 0x34, 0x56, 0x78, 0x90, 0xab, 0xcd, 0xef]);
    }

    #[test]
    fn test_find_sequence_in_data() {
        let seq = &[0x71, 0xAF, 0x00, 0x00];
        let data = &[0x00, 0x71, 0xAF, 0x00, 0x00, 0x01];
        let mut seq_offset = 0;
        let result = DataProcessor::find_sequence_in_data(seq, data, &mut seq_offset);
        assert_eq!(result, Some(4));
    }

    #[test]
    fn test_find_sequence_in_data_not_found() {
        let seq = &[0x71, 0xAF, 0x00, 0x00];
        let data = &[0x00, 0x71, 0xA0, 0x00, 0x00, 0x01];
        let mut seq_offset = 0;
        let result = DataProcessor::find_sequence_in_data(seq, data, &mut seq_offset);
        assert_eq!(result, None);
    }

    #[test]
    fn test_clean_data() {
        let mut processor = DataProcessor::new();
        processor.frame_data = vec![1, 2, 3];
        processor.skipped_lines = vec!["line1".to_string(), "line2".to_string()];
        processor.timestamp = 1234567890.0;
        processor.seq_offset = 5;

        processor.clear_data();

        assert_eq!(processor.frame_data.len(), 0);
        assert_eq!(processor.skipped_lines.len(), 0);
        assert_eq!(processor.timestamp, 0.0);
        assert_eq!(processor.seq_offset, 0);
    }

    #[test]
    fn test_process_next_line() {
        let mut processor = DataProcessor::new();
        let line = "2023-10-01 12:34:56.789,1234567890abcdef";
        let result = processor.process_next_line(line).unwrap();
        assert_eq!(result, false);
        assert_eq!(processor.frame_data.len(), 0);
        assert_eq!(processor.skipped_lines, vec![line.to_string()]);

        let line2 = "2023-10-01 12:34:56.790,ABCD71AF000001020304";
        let result2 = processor.process_next_line(line2).unwrap();
        assert_eq!(result2, false);
        assert_eq!(processor.frame_data, vec![0x71, 0xAF, 0, 0, 1, 2, 3, 4]);
        assert_eq!(processor.skipped_lines, vec![line.to_string()]);

        let line2 = "2023-10-01 12:34:56.790,1234";
        let result2 = processor.process_next_line(line2).unwrap();
        assert_eq!(result2, false);
        assert_eq!(
            processor.frame_data,
            vec![0x71, 0xAF, 0, 0, 1, 2, 3, 4, 0x12, 0x34]
        );

        let line2 = "2023-10-01 12:34:56.790,ABCD71A0000000000000";
        let result2 = processor.process_next_line(line2).unwrap();
        assert_eq!(result2, true);
        assert_eq!(
            processor.frame_data,
            vec![
                0x71, 0xAF, 0, 0, 1, 2, 3, 4, 0x12, 0x34, 0xAB, 0xCD, 0x71, 0xA0, 0, 0
            ]
        );
    }

    #[test]
    fn test_parse_pixel_packet() {
        let data = vec![0xA3, 0xED, 0x79, 0xC3, 0xFF, 0xEE];
        let (idx, itot, event) = DataProcessor::parse_pixel_packet(&data);
        assert_eq!(idx, 27455);
        assert_eq!(itot, 21);
        assert_eq!(event, 1);

        let data = vec![0xA3, 0xED, 0x79, 0xC3, 0x12, 0x34];
        let (idx, itot, event) = DataProcessor::parse_pixel_packet(&data);
        assert_eq!(idx, 27455);
        assert_eq!(itot, 4357);
        assert_eq!(event, 747);
    }

    #[test]
    fn test_extract_frame() {
        let mut processor = DataProcessor::new();
        processor.timestamp = 1696163696.789;
        processor.frame_data = vec![
            0x71, 0xAF, 0, 0, 0, 0, // pixel packet
            0xA3, 0xED, 0x79, 0xC3, 0xFF, 0xEE, // packet1
            0xA3, 0xE9, 0xF3, 0x33, 0xBF, 0xEE, // packet 2
            0x14, 0x00, 0x00, 0x00, 0x00, 0x02, 0x29, 0x01, // extra header
            0x71, 0xA0, 0, 0, 0, 0, // end of readout
        ];
        let frame = processor.extract_frame();
        assert_eq!(frame.itot.len(), 256 * 256);
        assert_eq!(frame.event.len(), 256 * 256);
        assert_eq!(frame.itot[27455], 21);
        assert_eq!(frame.event[27455], 1);
        assert_eq!(frame.itot[20287], 14);
        assert_eq!(frame.event[20287], 1);
        assert_eq!(frame.timestamp, 1696163696.789);
    }

    #[test]
    fn test_clusterize_frame() {
        let mut processor = DataProcessor::new();
        processor.timestamp = 1696163696.789;
        processor.frame_data = vec![
            0x71, 0xAF, 0, 0, 0, 0, // pixel packet
            0xA3, 0xED, 0x79, 0xC3, 0xFF, 0xEE, // packet1
            0xA3, 0xE9, 0xF3, 0x33, 0xBF, 0xEE, // packet2
            0x71, 0xA0, 0, 0, 0, 0,
        ];
        let mut frame = processor.extract_frame();
        processor.clusterize_frame(&mut frame);
        assert_eq!(frame.clusters.len(), 2);
    }

    #[test]
    fn test_get_next_frame() {
        let lines = vec![
            "TIMESTAMP,DATA",
            "2024-03-01 00:01:56.419,14584E000002290171AF00006974A4485FF33FEEA4486F10BFEEA470B35F3897A46EC999FFEEA46ED999FFEEA48ECF333F88A48E1FCCFFEEA48DFFE67FEEA48B91E081E7A48E2CCCFFEEA48E36673FEEA48E4CCE3FEEA48E5333BFEEA4AD9F333FEEA4AD7333BFEEA4ADA6673F",
            "2024-03-01 00:01:56.519,EEA4ADBF333FEEA4ADC6673FEEA4ADDF333FEEA4CD1999FFEEA4CCF999FFEEA4CD23387FEEA4CD3FCCFFEEA4CD4999FFEEA5AD1EAEFE37A6CAB48AB6E7A78B13387FEEA78B2906BFEEA78B36993FEEA7C7F6673FEEA7F78E667FEEA7F72F333FEEA7E8CCCCFFEEA7E803387FEE",
            "2024-03-01 00:01:56.619,A7E7BE667FEEA7F7CE667FEEA7F733C43FEEA7E81878BFEEA7F7D333BFEEA7E82333BFEE14584E0100020C01A7F7EF99BFEEA7E86878BFEEA7E872123FEEA817A091BFEEA817BC427FEEA87496673F01A8741FFE7FEEA991E7CA3897AA75D3C43FEEAA959E667FEEABB157E0A7",
            "2024-03-01 00:01:56.719,B771A00000FFFF000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
        ];
        let input_data = lines.join("\n");
        let cursor = Cursor::new(input_data);
        let mut reader = BufReader::new(cursor);

        let mut processor = DataProcessor::new();
        let frame = processor.get_next_frame(&mut reader).unwrap();
        assert_eq!(frame.itot.len(), 256 * 256);
        assert_eq!(frame.event.len(), 256 * 256);
        assert_eq!(frame.clusters.len(), 14);
        assert_eq!(frame.timestamp, 1696163696.789);
    }
}
