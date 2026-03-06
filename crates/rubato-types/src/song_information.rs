use std::collections::HashMap;

use crate::validatable::Validatable;
use bms_model::bms_model::{BMSModel, LNTYPE_LONGNOTE};
use bms_model::bms_model_utils::{
    TOTALNOTES_KEY, TOTALNOTES_LONG_KEY, TOTALNOTES_LONG_SCRATCH, TOTALNOTES_SCRATCH,
    total_notes_with_type,
};

/// Song detailed information
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SongInformation {
    /// Chart hash (SHA-256)
    pub sha256: String,
    /// Normal note count
    pub n: i32,
    /// Long note count
    pub ln: i32,
    /// Scratch note count
    pub s: i32,
    /// Long scratch note count
    pub ls: i32,
    /// Average density
    pub density: f64,
    /// Peak density
    pub peakdensity: f64,
    /// End density
    pub enddensity: f64,
    /// TOTAL
    pub total: f64,
    /// Main BPM
    pub mainbpm: f64,
    /// Distribution (encoded string)
    pub distribution: String,
    #[serde(skip)]
    pub distribution_values: Vec<[i32; 7]>,
    /// Speed change (encoded string)
    pub speedchange: String,
    #[serde(skip)]
    pub speedchange_values: Vec<[f64; 2]>,
    /// Lane notes (encoded string)
    pub lanenotes: String,
    #[serde(skip)]
    pub lanenotes_values: Vec<[i32; 3]>,
}

impl SongInformation {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_model(model: &BMSModel) -> Self {
        let mut info = SongInformation::new();
        info.sha256 = model.sha256().to_string();
        info.n = total_notes_with_type(model, TOTALNOTES_KEY);
        info.ln = total_notes_with_type(model, TOTALNOTES_LONG_KEY);
        info.s = total_notes_with_type(model, TOTALNOTES_SCRATCH);
        info.ls = total_notes_with_type(model, TOTALNOTES_LONG_SCRATCH);
        info.total = model.total();

        let mode = match model.mode() {
            Some(m) => m,
            None => return info,
        };

        let mode_key = mode.key();
        let mut lanenotes_arr = vec![[0i32; 3]; mode_key as usize];
        let last_time = model.last_time();
        let data_len = (last_time / 1000 + 2) as usize;
        let mut data = vec![[0i32; 7]; data_len];
        let mut pos: i32 = 0;
        let total_notes = model.total_notes();
        let model_total = model.total();
        let mut border = if model_total != 0.0 {
            (total_notes as f64 * (1.0 - 100.0 / model_total)) as i32
        } else {
            0
        };
        let mut borderpos: i32 = 0;

        let lnmode = model.lnmode();
        let lntype = model.lntype();

        let all_tls = model.all_time_lines();
        for (tl_idx, tl) in all_tls.iter().enumerate() {
            if tl.time() / 1000 != pos {
                pos = tl.time() / 1000;
            }
            for i in 0..mode_key {
                let note = match tl.note(i) {
                    Some(n) => n,
                    None => continue,
                };

                if note.is_long() && !note.is_end() {
                    // Find the paired LN end note by scanning forward
                    let mut end_time = tl.time();
                    for future_tl in &all_tls[(tl_idx + 1)..] {
                        if let Some(end_note) = future_tl.note(i)
                            && end_note.is_long()
                            && end_note.is_end()
                        {
                            end_time = future_tl.time();
                            break;
                        }
                    }
                    let start_idx = tl.time() / 1000;
                    let end_idx = end_time / 1000;
                    let col = if mode.is_scratch_key(i) { 1 } else { 4 };
                    for index in start_idx..=end_idx {
                        if (index as usize) < data.len() {
                            data[index as usize][col] += 1;
                        }
                    }
                }

                let is_ln_end_skip = (lnmode == 1 || (lnmode == 0 && lntype == LNTYPE_LONGNOTE))
                    && note.is_long()
                    && note.is_end();

                if !is_ln_end_skip {
                    let time_idx = (tl.time() / 1000) as usize;
                    if note.is_normal() {
                        if time_idx < data.len() {
                            data[time_idx][if mode.is_scratch_key(i) { 2 } else { 5 }] += 1;
                        }
                        lanenotes_arr[i as usize][0] += 1;
                    }
                    if note.is_long() {
                        if time_idx < data.len() {
                            data[time_idx][if mode.is_scratch_key(i) { 0 } else { 3 }] += 1;
                            data[time_idx][if mode.is_scratch_key(i) { 1 } else { 4 }] -= 1;
                        }
                        lanenotes_arr[i as usize][1] += 1;
                    }
                    if note.is_mine() {
                        if time_idx < data.len() {
                            data[time_idx][6] += 1;
                        }
                        lanenotes_arr[i as usize][2] += 1;
                    }

                    border -= 1;
                    if border == 0 {
                        borderpos = pos;
                    }
                }
            }
        }

        let bd = if data.is_empty() {
            0
        } else {
            total_notes / data.len() as i32 / 4
        };
        info.density = 0.0;
        info.peakdensity = 0.0;
        let mut count = 0;
        for row in &data {
            let notes = row[0] + row[1] + row[2] + row[3] + row[4] + row[5];
            if notes as f64 > info.peakdensity {
                info.peakdensity = notes as f64;
            }
            if notes >= bd {
                info.density += notes as f64;
                count += 1;
            }
        }
        if count > 0 {
            info.density /= count as f64;
        }

        let d = 5i32.min(data.len() as i32 - borderpos - 1);
        info.enddensity = 0.0;
        for i in borderpos..(data.len() as i32 - d) {
            let mut notes = 0;
            for j in 0..d {
                let idx = (i + j) as usize;
                if idx < data.len() {
                    notes += data[idx][0]
                        + data[idx][1]
                        + data[idx][2]
                        + data[idx][3]
                        + data[idx][4]
                        + data[idx][5];
                }
            }
            let density = if d > 0 {
                (notes as f64) / (d as f64)
            } else {
                0.0
            };
            if density > info.enddensity {
                info.enddensity = density;
            }
        }

        info.set_distribution_values(&data);

        // Speed change tracking
        let mut speed_list: Vec<[f64; 2]> = Vec::new();
        let mut bpm_note_count_map: HashMap<u64, i32> = HashMap::new();
        let mut now_speed = model.bpm();
        speed_list.push([now_speed, 0.0]);

        let tls = model.all_time_lines();
        for tl in tls {
            let bpm_key = tl.bpm().to_bits();
            let notecount = *bpm_note_count_map.get(&bpm_key).unwrap_or(&0);
            bpm_note_count_map.insert(bpm_key, notecount + tl.total_notes());

            if tl.stop() > 0 {
                if now_speed != 0.0 {
                    now_speed = 0.0;
                    speed_list.push([now_speed, tl.time() as f64]);
                }
            } else if now_speed != tl.bpm() * tl.scroll() {
                now_speed = tl.bpm() * tl.scroll();
                speed_list.push([now_speed, tl.time() as f64]);
            }
        }

        let mut maxcount = 0;
        // Sort by BPM ascending so that on tie, highest BPM wins (matches Java HashMap bucket order)
        let mut bpm_entries: Vec<_> = bpm_note_count_map.iter().collect();
        bpm_entries.sort_by(|a, b| {
            f64::from_bits(*a.0)
                .partial_cmp(&f64::from_bits(*b.0))
                .unwrap()
        });
        for (&bpm_bits, &count) in bpm_entries {
            if count >= maxcount {
                maxcount = count;
                info.mainbpm = f64::from_bits(bpm_bits);
            }
        }

        if !tls.is_empty() {
            let last_tl_time = tls[tls.len() - 1].time() as f64;
            if speed_list.last().map(|s| s[1]) != Some(last_tl_time) {
                speed_list.push([now_speed, last_tl_time]);
            }
        }

        info.set_speedchange_values(&speed_list);
        info.set_lanenotes_values(&lanenotes_arr);

        info
    }

    pub fn set_distribution(&mut self, distribution: String) {
        self.distribution = distribution.clone();
        let mut index = vec![0usize, 2, 3, 5, 6];
        let mut dist = distribution.as_str();
        if dist.starts_with('#') {
            index = vec![0, 1, 2, 3, 4, 5, 6];
            dist = &dist[1..];
        }
        let count = if dist.len().is_multiple_of(index.len() * 2) {
            dist.len() / (index.len() * 2)
        } else {
            0
        };
        if count == 0 {
            log::warn!("distribution string is invalid");
        }
        self.distribution_values = vec![[0i32; 7]; count];
        for i in 0..count {
            for (j, &idx) in index.iter().enumerate() {
                match parse_int36(dist, i * (index.len() * 2) + j * 2) {
                    Ok(val) => self.distribution_values[i][idx] = val,
                    Err(_) => {
                        log::warn!("exception while parsing distribution");
                    }
                }
            }
        }
    }

    pub fn distribution_values(&self) -> &[[i32; 7]] {
        &self.distribution_values
    }

    pub fn set_distribution_values(&mut self, values: &[[i32; 7]]) {
        self.distribution_values = values.to_vec();
        let mut sb = String::with_capacity(values.len() * 14 + 1);
        sb.push('#');
        for row in values {
            for &val in row.iter().take(7) {
                let value = val.clamp(0, 36 * 36 - 1);
                let val1 = value / 36;
                sb.push(if val1 >= 10 {
                    (b'a' + (val1 as u8 - 10)) as char
                } else {
                    (b'0' + val1 as u8) as char
                });
                let val2 = value % 36;
                sb.push(if val2 >= 10 {
                    (b'a' + (val2 as u8 - 10)) as char
                } else {
                    (b'0' + val2 as u8) as char
                });
            }
        }
        self.distribution = sb;
    }

    pub fn set_speedchange(&mut self, speedchange: String) {
        self.speedchange = speedchange.clone();
        let mut result: Vec<[f64; 2]> = Vec::new();
        let mut index = 0;
        let mut values = [0.0f64; 2];

        let parts: Vec<&str> = speedchange.split(',').collect();
        let mut ok = true;
        for s in &parts {
            match s.parse::<f64>() {
                Ok(val) => {
                    values[index] = val;
                    index += 1;
                    if index == values.len() {
                        index = 0;
                        result.push(values);
                        values = [0.0f64; 2];
                    }
                }
                Err(_) => {
                    ok = false;
                    break;
                }
            }
        }
        if !ok {
            result.clear();
        }
        self.speedchange_values = result;
    }

    pub fn speedchange_values(&self) -> &[[f64; 2]] {
        &self.speedchange_values
    }

    pub fn set_speedchange_values(&mut self, values: &[[f64; 2]]) {
        self.speedchange_values = values.to_vec();
        let mut sb = String::with_capacity(values.len() * 14 + 1);
        for (i, row) in values.iter().enumerate() {
            sb.push_str(&format!("{},{}", row[0], row[1]));
            if i < values.len() - 1 {
                sb.push(',');
            }
        }
        self.speedchange = sb;
    }

    pub fn set_lanenotes(&mut self, lanenotes: String) {
        self.lanenotes = lanenotes.clone();
        let mut result: Vec<[i32; 3]> = Vec::new();
        let mut index = 0;
        let mut values = [0i32; 3];

        let parts: Vec<&str> = lanenotes.split(',').collect();
        let mut ok = true;
        for s in &parts {
            match s.parse::<i32>() {
                Ok(val) => {
                    values[index] = val;
                    index += 1;
                    if index == values.len() {
                        index = 0;
                        result.push(values);
                        values = [0i32; 3];
                    }
                }
                Err(_) => {
                    ok = false;
                    break;
                }
            }
        }
        if !ok {
            result.clear();
        }
        self.lanenotes_values = result;
    }

    pub fn lanenotes_values(&self) -> &[[i32; 3]] {
        &self.lanenotes_values
    }

    pub fn set_lanenotes_values(&mut self, values: &[[i32; 3]]) {
        self.lanenotes_values = values.to_vec();
        let mut sb = String::with_capacity(values.len() * 14 + 1);
        for (i, row) in values.iter().enumerate() {
            sb.push_str(&format!("{},{},{}", row[0], row[1], row[2]));
            if i < values.len() - 1 {
                sb.push(',');
            }
        }
        self.lanenotes = sb;
    }
}

fn parse_int36(s: &str, index: usize) -> Result<i32, ()> {
    let bytes = s.as_bytes();
    if index + 1 >= bytes.len() {
        return Err(());
    }

    let c1 = bytes[index] as char;
    let result_high = if c1.is_ascii_digit() {
        ((c1 as i32) - ('0' as i32)) * 36
    } else if c1.is_ascii_lowercase() {
        ((c1 as i32) - ('a' as i32) + 10) * 36
    } else if c1.is_ascii_uppercase() {
        ((c1 as i32) - ('A' as i32) + 10) * 36
    } else {
        return Err(());
    };

    let c2 = bytes[index + 1] as char;
    let result_low = if c2.is_ascii_digit() {
        (c2 as i32) - ('0' as i32)
    } else if c2.is_ascii_lowercase() {
        (c2 as i32) - ('a' as i32) + 10
    } else if c2.is_ascii_uppercase() {
        (c2 as i32) - ('A' as i32) + 10
    } else {
        return Err(());
    };

    Ok(result_high + result_low)
}

impl Validatable for SongInformation {
    fn validate(&mut self) -> bool {
        if self.sha256.len() != 64 {
            return false;
        }
        if self.n < 0 || self.ln < 0 || self.s < 0 || self.ls < 0 {
            return false;
        }
        if self.density < 0.0
            || self.peakdensity < 0.0
            || self.enddensity < 0.0
            || self.density > self.peakdensity
            || self.enddensity > self.peakdensity
        {
            return false;
        }
        true
    }
}
