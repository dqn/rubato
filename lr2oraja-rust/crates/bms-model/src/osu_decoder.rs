use std::collections::BTreeMap;
use std::io::{BufReader, Cursor};
use std::path::Path;

use md5::Md5;
use sha2::{Digest, Sha256};

use crate::bms_decoder::convert_hex_string;
use crate::bms_model::{BMSModel, JudgeRankType, TotalType};
use crate::chart_information::ChartInformation;
use crate::decode_log::DecodeLog;
use crate::mode::Mode;
use crate::note::Note;
use crate::osu::{Osu, TimingPoints};
use crate::time_line::TimeLine;

pub struct OSUDecoder {
    pub lntype: i32,
    pub log: Vec<DecodeLog>,
}

impl OSUDecoder {
    pub fn new(lntype: i32) -> Self {
        OSUDecoder {
            lntype,
            log: Vec::new(),
        }
    }

    pub fn decode(&mut self, info: ChartInformation) -> Option<BMSModel> {
        self.lntype = info.lntype;
        let path = info.path.clone()?;
        self.decode_path(&path)
    }

    pub fn decode_path(&mut self, f: &Path) -> Option<BMSModel> {
        self.log.clear();

        // Read file and compute hashes
        let file_bytes = std::fs::read(f).ok()?;
        let md5_hash = {
            let mut hasher = Md5::new();
            hasher.update(&file_bytes);
            convert_hex_string(&hasher.finalize())
        };
        let sha256_hash = {
            let mut hasher = Sha256::new();
            hasher.update(&file_bytes);
            convert_hex_string(&hasher.finalize())
        };

        // Decode as MS932 (Shift_JIS superset)
        let (decoded, _, _) = encoding_rs::SHIFT_JIS.decode(&file_bytes);
        let mut reader = BufReader::new(Cursor::new(decoded.as_bytes().to_vec()));
        let osu = Osu::parse(&mut reader);

        if osu.timing_points.is_empty() || osu.hit_objects.is_empty() {
            return None;
        }

        let mut model = BMSModel::new();
        model.set_md5(md5_hash);
        model.set_sha256(sha256_hash);

        if osu.general.mode != 3 {
            return None;
        }

        let keymode = osu.difficulty.circle_size as i32;
        model.set_title(&osu.metadata.title);
        model.set_sub_title(format!("[{}]", osu.metadata.version));
        model.set_artist(&osu.metadata.artist);
        model.set_sub_artist(&osu.metadata.creator);
        model.set_genre(format!("{}K", keymode));
        model.set_judgerank(3);
        model.set_judgerank_type(JudgeRankType::BmsRank);
        model.set_total(0.0);
        model.set_total_type(TotalType::Bms);
        model.set_playlevel("");

        let mapping: Vec<i32> = match keymode {
            4 => vec![0, 2, 4, 6, -1, -1, -1, -1],
            5 => vec![0, 1, 2, 3, 4, -1],
            6 => vec![0, 1, 2, 4, 5, 6, -1, -1],
            7 => vec![0, 1, 2, 3, 4, 5, 6, -1],
            8 => vec![7, 0, 1, 2, 3, 4, 5, 6],
            9 => vec![0, 1, 2, 3, 4, 5, 6, 7, 8],
            10 => vec![0, 1, 2, 3, 4, 6, 7, 8, 9, 10, -1, -1],
            12 => vec![5, 0, 1, 2, 3, 4, 6, 7, 8, 9, 10, 11],
            14 => vec![0, 1, 2, 3, 4, 5, 6, 8, 9, 10, 11, 12, 13, 14, -1, -1],
            16 => vec![7, 0, 1, 2, 3, 4, 5, 6, 8, 9, 10, 11, 12, 13, 14, 15],
            _ => return None,
        };

        match keymode {
            4 | 6 | 7 | 8 => model.set_mode(Mode::BEAT_7K),
            5 => model.set_mode(Mode::BEAT_5K),
            9 => model.set_mode(Mode::POPN_9K),
            10 | 12 => model.set_mode(Mode::BEAT_10K),
            14 | 16 => model.set_mode(Mode::BEAT_14K),
            _ => return None,
        }

        model.set_banner("");

        let offset: i32 = 38;
        let mut bga_list: Vec<String> = Vec::new();
        let mut videos: Vec<(i32, String)> = Vec::new(); // (adjusted start_time, name)
        let mut bg_sounds: Vec<(i32, String)> = Vec::new(); // (start_time, name)
        let mut wavmap: Vec<String> = Vec::new();
        wavmap.push(osu.general.audio_filename.clone());

        for event in &osu.events {
            match event.event_type.as_str() {
                "0" => {
                    if !event.event_params.is_empty() {
                        model.set_backbmp(&event.event_params[0]);
                        model.set_stagefile(&event.event_params[0]);
                    }
                }
                "1" | "Video" => {
                    let adjusted_time = event.start_time + offset;
                    if !event.event_params.is_empty() {
                        let name = event.event_params[0].replace('"', "");
                        bga_list.push(name.clone());
                        videos.push((adjusted_time, name));
                    }
                }
                "5" | "Sample" => {
                    if event.event_params.len() > 1 {
                        let name = event.event_params[1].replace('"', "");
                        wavmap.push(name.clone());
                        bg_sounds.push((event.start_time, name));
                    }
                }
                _ => continue,
            }
        }
        model.set_preview(&osu.general.audio_filename);

        let mode_key = model.get_mode().map(|m| m.key()).unwrap_or(0);
        let mut timelines: BTreeMap<i32, TimeLine> = BTreeMap::new();
        let mut timing_points: Vec<TimingPoints> = Vec::new();
        let mut svs: Vec<TimingPoints> = Vec::new();

        for i in 0..osu.timing_points.len() {
            let mut point = osu.timing_points[i].clone();
            point.time += offset as f32;
            if point.uninherited {
                timing_points.push(point.clone());

                let sv = TimingPoints {
                    time: point.time,
                    beat_length: -100.0,
                    meter: point.meter,
                    sample_set: point.sample_set,
                    sample_index: point.sample_index,
                    volume: point.volume,
                    uninherited: false,
                    effects: point.effects,
                };
                if i != osu.timing_points.len() - 1 {
                    let next_time = osu.timing_points[i + 1].time + offset as f32;
                    if (next_time - point.time).abs() > f32::EPSILON {
                        svs.push(sv);
                    }
                } else {
                    svs.push(sv);
                }
            } else {
                if !svs.is_empty() {
                    let last_sv = svs.last().unwrap();
                    if (last_sv.time - point.time).abs() < f32::EPSILON {
                        svs.last_mut().unwrap().beat_length = point.beat_length;
                        continue;
                    }
                }
                svs.push(point);
            }
        }

        model.set_bpm(get_bpm(&timing_points, 0));

        bga_list.push(model.get_backbmp().to_string());
        let bgm_tl = get_timeline(&mut timelines, 0, 0.0, mode_key);
        let bgm = Note::new_normal_with_start_duration(0, 0, 0);
        bgm_tl.add_back_ground_note(bgm);
        bgm_tl.set_bpm(get_bpm(&timing_points, bgm_tl.get_time()));
        bgm_tl.set_scroll(get_sv(&svs, bgm_tl.get_time()));
        bgm_tl.set_bga(bga_list.len() as i32 - 1);

        for (i, &(start_time, _)) in videos.iter().enumerate() {
            let section = get_section(&timing_points, start_time);
            let tl = get_timeline(&mut timelines, start_time, section, mode_key);
            tl.set_bga(i as i32);
            tl.set_bpm(get_bpm(&timing_points, start_time));
            tl.set_scroll(get_sv(&svs, start_time));
        }
        for (i, &(start_time, _)) in bg_sounds.iter().enumerate() {
            let section = get_section(&timing_points, start_time);
            let tl = get_timeline(&mut timelines, start_time, section, mode_key);
            let note = Note::new_normal_with_start_duration((i + 1) as i32, start_time as i64, 0);
            tl.add_back_ground_note(note);
            tl.set_bpm(get_bpm(&timing_points, start_time));
            tl.set_scroll(get_sv(&svs, start_time));
        }

        for point in &timing_points {
            let time = point.time as i32;
            let section = get_section(&timing_points, time);
            let tl = get_timeline(&mut timelines, time, section, mode_key);
            tl.set_bpm(safe_bpm_from_beat_length(point.beat_length as f64));
            tl.set_scroll(get_sv(&svs, time));
        }
        for sv in &svs {
            let time = sv.time as i32;
            let section = get_section(&timing_points, time);
            let tl = get_timeline(&mut timelines, time, section, mode_key);
            if sv.beat_length != 0.0 {
                tl.set_scroll(100.0 / (-sv.beat_length as f64));
            }
            tl.set_bpm(get_bpm(&timing_points, time));
        }

        // Generate section lines
        for i in 0..timing_points.len() {
            let last_note_time = osu.hit_objects.last().map(|h| h.time).unwrap_or(0);
            let point = &timing_points[i];
            let begin_time = point.time as i32;
            let end_time = if i < timing_points.len() - 1 {
                timing_points[i + 1].time as i32
            } else {
                last_note_time
            };
            let begin_section = get_section(&timing_points, begin_time);
            let duration = end_time - begin_time;
            let total_sections = if point.beat_length != 0.0 {
                duration as f32 / (point.beat_length * 4.0)
            } else {
                0.0
            };
            if total_sections > 10000.0 {
                let first_line = get_timeline(&mut timelines, begin_time, begin_section, mode_key);
                first_line.set_bpm(safe_bpm_from_beat_length(point.beat_length as f64));
                first_line.set_scroll(get_sv(&svs, begin_time));
                first_line.set_section_line(true);

                let end_sec = begin_section + total_sections as f64;
                let last_line = get_timeline(&mut timelines, end_time, end_sec, mode_key);
                let first_bpm = safe_bpm_from_beat_length(point.beat_length as f64);
                last_line.set_bpm(first_bpm);
                last_line.set_scroll(get_sv(&svs, end_time));
                last_line.set_section_line(true);
                continue;
            }
            for section_idx in 0..=(total_sections as i32) {
                let time = begin_time + (section_idx as f32 * point.beat_length * 4.0) as i32;
                let section = begin_section + section_idx as f64;
                let line = get_timeline(&mut timelines, time, section, mode_key);
                line.set_bpm(safe_bpm_from_beat_length(point.beat_length as f64));
                line.set_scroll(get_sv(&svs, time));
                line.set_section_line(true);
            }
        }

        // Hit objects
        for hit_object in &osu.hit_objects {
            if hit_object.time < 0 {
                continue;
            }
            let adjusted_time = hit_object.time + offset;

            let column_idx = ((hit_object.x as f32 * keymode as f32 / 512.0).floor() as i32)
                .max(0)
                .min(keymode - 1);
            let section = get_section(&timing_points, adjusted_time);

            let tl = get_timeline(&mut timelines, adjusted_time, section, mode_key);
            tl.set_bpm(get_bpm(&timing_points, tl.get_time()));
            tl.set_scroll(get_sv(&svs, tl.get_time()));
            let is_mania_hold = (hit_object.hit_type & 0x80) > 0;
            let wav_idx: i32 = -2;

            if is_mania_hold {
                let tail_time_ms = hit_object
                    .object_params
                    .first()
                    .and_then(|s| s.parse::<i32>().ok())
                    .unwrap_or(0)
                    + offset;
                let tail_time_us = tail_time_ms as i64 * 1000;
                if tail_time_ms <= adjusted_time {
                    let note = Note::new_normal_with_start_duration(
                        wav_idx,
                        adjusted_time as i64 * 1000,
                        0,
                    );
                    tl.set_note(mapping[column_idx as usize], Some(note));
                    continue;
                }
                let mut head =
                    Note::new_long_with_start_duration(wav_idx, adjusted_time as i64 * 1000, 0);
                head.set_long_note_type(model.get_lntype());
                tl.set_note(mapping[column_idx as usize], Some(head));

                let tail_section = get_section(&timing_points, tail_time_ms);
                let mut tail = Note::new_long_with_start_duration(wav_idx, tail_time_us, 0);
                tail.set_long_note_type(model.get_lntype());
                tail.set_end(true);
                let tail_tl = get_timeline(&mut timelines, tail_time_ms, tail_section, mode_key);
                tail_tl.set_bpm(get_bpm(&timing_points, tail_time_ms));
                tail_tl.set_scroll(get_sv(&svs, tail_time_ms));
                tail_tl.set_note(mapping[column_idx as usize], Some(tail));
            } else {
                let note =
                    Note::new_normal_with_start_duration(wav_idx, adjusted_time as i64 * 1000, 0);
                tl.set_note(mapping[column_idx as usize], Some(note));
            }
        }

        model.set_wav_list(wavmap);
        let tl_vec: Vec<TimeLine> = timelines.into_values().collect();
        model.set_all_time_line(tl_vec);
        model.set_bga_list(bga_list);
        model.set_chart_information(ChartInformation::new(
            Some(f.to_path_buf()),
            self.lntype,
            None,
        ));
        Some(model)
    }
}

fn get_timing_point(timing_points: &[TimingPoints], time: i32) -> Option<&TimingPoints> {
    let mut entry = timing_points.first()?;
    let mut last_idx = 0usize;
    while (entry.time as i32) < time {
        last_idx += 1;
        if last_idx >= timing_points.len() {
            break;
        }
        let next_entry = &timing_points[last_idx];
        // Skip entries with same time as current
        if next_entry.time as i32 <= entry.time as i32 {
            continue;
        }
        if next_entry.time as i32 <= time {
            entry = next_entry;
        } else {
            break;
        }
    }
    Some(entry)
}

/// Convert beat_length (ms per beat) to BPM. Returns 120.0 as fallback if beat_length is 0.
fn safe_bpm_from_beat_length(beat_length: f64) -> f64 {
    if beat_length == 0.0 {
        120.0
    } else {
        1.0 / beat_length * 1000.0 * 60.0
    }
}

fn get_bpm(timing_points: &[TimingPoints], time: i32) -> f64 {
    match get_timing_point(timing_points, time) {
        Some(point) => safe_bpm_from_beat_length(point.beat_length as f64),
        None => 120.0, // fallback BPM
    }
}

fn get_sv(svs: &[TimingPoints], time: i32) -> f64 {
    if svs.is_empty() {
        return 1.0;
    }
    let entry = &svs[0];
    if entry.time as i32 > time {
        return 1.0;
    }
    let mut current = entry;
    let mut last_idx = 0usize;
    while (current.time as i32) < time {
        last_idx += 1;
        if last_idx >= svs.len() {
            break;
        }
        let next_entry = &svs[last_idx];
        if next_entry.time as i32 <= current.time as i32 {
            continue;
        }
        if next_entry.time as i32 <= time {
            current = next_entry;
        } else {
            break;
        }
    }
    if current.beat_length == 0.0 {
        1.0
    } else {
        100.0 / (-current.beat_length as f64)
    }
}

fn get_timeline(
    timelines: &mut BTreeMap<i32, TimeLine>,
    time: i32,
    section: f64,
    mode_key: i32,
) -> &mut TimeLine {
    timelines
        .entry(time)
        .or_insert_with(|| TimeLine::new(section, time as i64 * 1000, mode_key))
}

/// Safely divide by beat_length * 4.0. Returns 0.0 if beat_length is 0.
fn safe_section_delta(time_delta: f64, beat_length: f64) -> f64 {
    let divisor = beat_length * 4.0;
    if divisor == 0.0 {
        0.0
    } else {
        time_delta / divisor
    }
}

fn get_section(timing_points: &[TimingPoints], time: i32) -> f64 {
    let entry = match timing_points.first() {
        Some(e) => e,
        None => return 0.0,
    };
    if time <= entry.time as i32 {
        return safe_section_delta(time as f64, entry.beat_length as f64);
    }
    let mut section = safe_section_delta(entry.time as f64, entry.beat_length as f64);
    let mut current = entry;
    let mut last_idx = 0usize;
    while (current.time as i32) < time {
        last_idx += 1;
        if last_idx >= timing_points.len() {
            section += safe_section_delta(
                (time - current.time as i32) as f64,
                current.beat_length as f64,
            );
            break;
        }
        let next_entry = &timing_points[last_idx];
        if next_entry.time as i32 <= current.time as i32 {
            continue;
        }
        if next_entry.time as i32 > time {
            section += safe_section_delta(
                (time - current.time as i32) as f64,
                current.beat_length as f64,
            );
            break;
        }
        section += safe_section_delta(
            (next_entry.time as i32 - current.time as i32) as f64,
            current.beat_length as f64,
        );
        current = next_entry;
    }
    section
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_timing_point(time: f32, beat_length: f32, uninherited: bool) -> TimingPoints {
        TimingPoints {
            time,
            beat_length,
            meter: 4,
            sample_set: 0,
            sample_index: 0,
            volume: 100,
            uninherited,
            effects: 0,
        }
    }

    #[test]
    fn test_safe_bpm_from_beat_length_normal() {
        let bpm = safe_bpm_from_beat_length(500.0); // 500ms per beat = 120 BPM
        assert!((bpm - 120.0).abs() < 0.001, "expected 120 BPM, got {}", bpm);
    }

    #[test]
    fn test_safe_bpm_from_beat_length_zero_returns_fallback() {
        let bpm = safe_bpm_from_beat_length(0.0);
        assert_eq!(
            bpm, 120.0,
            "BPM should fall back to 120.0 when beat_length is 0"
        );
    }

    #[test]
    fn test_safe_section_delta_normal() {
        let delta = safe_section_delta(2000.0, 500.0);
        // 2000 / (500 * 4) = 1.0
        assert!((delta - 1.0).abs() < 0.001, "expected 1.0, got {}", delta);
    }

    #[test]
    fn test_safe_section_delta_zero_beat_length() {
        let delta = safe_section_delta(2000.0, 0.0);
        assert_eq!(delta, 0.0, "should return 0.0 when beat_length is 0");
    }

    #[test]
    fn test_get_bpm_zero_beat_length() {
        let points = vec![make_timing_point(0.0, 0.0, true)];
        let bpm = get_bpm(&points, 0);
        assert_eq!(
            bpm, 120.0,
            "should return fallback BPM when beat_length is 0"
        );
    }

    #[test]
    fn test_get_bpm_normal_beat_length() {
        let points = vec![make_timing_point(0.0, 500.0, true)];
        let bpm = get_bpm(&points, 0);
        assert!(
            (bpm - 120.0).abs() < 0.001,
            "expected 120 BPM for 500ms beat_length"
        );
    }

    #[test]
    fn test_get_sv_zero_beat_length() {
        let svs = vec![make_timing_point(0.0, 0.0, false)];
        let sv = get_sv(&svs, 0);
        assert_eq!(sv, 1.0, "should return 1.0 when beat_length is 0");
    }

    #[test]
    fn test_get_sv_normal() {
        let svs = vec![make_timing_point(0.0, -100.0, false)];
        let sv = get_sv(&svs, 0);
        assert!(
            (sv - 1.0).abs() < 0.001,
            "sv should be 1.0 for beat_length -100"
        );
    }

    #[test]
    fn test_get_section_zero_beat_length() {
        let points = vec![make_timing_point(0.0, 0.0, true)];
        let section = get_section(&points, 1000);
        assert!(
            section.is_finite(),
            "section should be finite, got {}",
            section
        );
        assert_eq!(section, 0.0, "should return 0.0 when beat_length is 0");
    }

    #[test]
    fn test_get_section_normal() {
        let points = vec![make_timing_point(0.0, 500.0, true)];
        // time=2000, beat_length=500 => section = 2000 / (500*4) = 1.0
        let section = get_section(&points, 2000);
        assert!(
            (section - 1.0).abs() < 0.001,
            "expected section 1.0, got {}",
            section
        );
    }

    // --- Regression tests for fuzzer-found panics ---

    #[test]
    fn test_get_timing_point_empty_slice_returns_none() {
        let empty: Vec<TimingPoints> = vec![];
        assert!(
            get_timing_point(&empty, 0).is_none(),
            "get_timing_point should return None for empty slice"
        );
    }

    #[test]
    fn test_get_bpm_empty_timing_points_returns_fallback() {
        let empty: Vec<TimingPoints> = vec![];
        let bpm = get_bpm(&empty, 0);
        assert_eq!(
            bpm, 120.0,
            "get_bpm should return fallback 120.0 for empty timing_points"
        );
    }

    #[test]
    fn test_get_section_empty_timing_points_returns_zero() {
        let empty: Vec<TimingPoints> = vec![];
        let section = get_section(&empty, 1000);
        assert_eq!(
            section, 0.0,
            "get_section should return 0.0 for empty timing_points"
        );
    }

    #[test]
    fn test_get_sv_empty_slice_returns_default() {
        let empty: Vec<TimingPoints> = vec![];
        let sv = get_sv(&empty, 0);
        assert_eq!(sv, 1.0, "get_sv should return 1.0 for empty slice");
    }

    #[test]
    fn test_osu_parser_section_header_missing_closing_bracket() {
        let input = b"osu file format v14\r\n\r\n[General\r\nMode: 3\r\n";
        let mut reader = std::io::BufReader::new(std::io::Cursor::new(input.to_vec()));
        let osu = Osu::parse(&mut reader);
        // Should not panic; section name parsed as "General" (without ']')
        assert_eq!(osu.general.mode, 3);
    }

    #[test]
    fn test_osu_parser_section_header_multibyte_chars() {
        // Section header with multi-byte UTF-8 chars and no closing bracket
        let input = "osu file format v14\r\n\r\n[\u{65E5}\u{672C}\r\n".as_bytes();
        let mut reader = std::io::BufReader::new(std::io::Cursor::new(input.to_vec()));
        // Should not panic on multi-byte char boundary slicing
        let _osu = Osu::parse(&mut reader);
    }

    #[test]
    fn test_osu_parser_section_header_only_open_bracket() {
        let input = b"osu file format v14\r\n\r\n[\r\n";
        let mut reader = std::io::BufReader::new(std::io::Cursor::new(input.to_vec()));
        // Single '[' has len < 2 so it is skipped; should not panic
        let _osu = Osu::parse(&mut reader);
    }

    #[test]
    fn test_osu_parser_section_header_bracket_with_one_char() {
        let input = b"osu file format v14\r\n\r\n[X\r\n";
        let mut reader = std::io::BufReader::new(std::io::Cursor::new(input.to_vec()));
        // "[X" without closing bracket; should not panic
        let _osu = Osu::parse(&mut reader);
    }

    #[test]
    fn test_osu_parser_empty_input() {
        let input = b"";
        let mut reader = std::io::BufReader::new(std::io::Cursor::new(input.to_vec()));
        let osu = Osu::parse(&mut reader);
        assert!(osu.timing_points.is_empty());
        assert!(osu.hit_objects.is_empty());
    }

    #[test]
    fn test_osu_parser_garbage_input() {
        let input = b"\xff\xfe\x00\x01\x02\x03garbage\nmore garbage\n[[[[\n";
        let mut reader = std::io::BufReader::new(std::io::Cursor::new(input.to_vec()));
        // Should not panic on arbitrary binary input
        let _osu = Osu::parse(&mut reader);
    }
}
