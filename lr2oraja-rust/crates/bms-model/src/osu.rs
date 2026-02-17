// osu! beatmap file format data structures
// Reference: Java bms/model/osu/*.java

use std::io::BufRead;

/// General section
#[derive(Debug, Default)]
pub struct General {
    pub audio_filename: String,
    pub audio_lead_in: i32,
    pub preview_time: i32,
    pub countdown: i32,
    pub sample_set: String,
    pub stack_leniency: f64,
    pub mode: i32,
    pub letterbox_in_breaks: bool,
    pub widescreen_storyboard: bool,
    pub special_style: bool,
}

/// Metadata section
#[derive(Debug, Default)]
pub struct Metadata {
    pub title: String,
    pub title_unicode: String,
    pub artist: String,
    pub artist_unicode: String,
    pub creator: String,
    pub version: String,
    pub source: String,
    pub tags: Vec<String>,
    pub beatmap_id: i32,
    pub beatmap_set_id: i32,
}

/// Difficulty section
#[derive(Debug, Default)]
pub struct Difficulty {
    pub hp_drain_rate: f32,
    pub circle_size: f32,
    pub overall_difficulty: f32,
    pub approach_rate: f32,
    pub slider_multiplier: f64,
    pub slider_tick_rate: f64,
}

/// Events section entry
#[derive(Debug, Default, Clone)]
pub struct Events {
    pub event_type: String,
    pub start_time: i32,
    pub event_params: Vec<String>,
}

/// TimingPoints section entry
#[derive(Debug, Clone)]
pub struct TimingPoints {
    pub time: f32,
    pub beat_length: f32,
    #[allow(dead_code)] // Parsed for completeness (osu! format field)
    pub meter: i32,
    #[allow(dead_code)] // Parsed for completeness (osu! format field)
    pub sample_set: i32,
    #[allow(dead_code)] // Parsed for completeness (osu! format field)
    pub sample_index: i32,
    #[allow(dead_code)] // Parsed for completeness (osu! format field)
    pub volume: i32,
    pub uninherited: bool,
    #[allow(dead_code)] // Parsed for completeness (osu! format field)
    pub effects: i32,
}

impl Default for TimingPoints {
    fn default() -> Self {
        Self {
            time: 0.0,
            beat_length: 0.0,
            meter: 4,
            sample_set: 0,
            sample_index: 0,
            volume: 100,
            uninherited: true,
            effects: 0,
        }
    }
}

/// HitSample data
#[derive(Debug, Default, Clone)]
pub struct HitSample {
    pub normal_set: i32,
    pub additional_set: i32,
    pub index: i32,
    pub volume: i32,
    pub filename: String,
}

/// HitObjects section entry
#[derive(Debug, Default, Clone)]
pub struct HitObjects {
    pub x: i32,
    #[allow(dead_code)] // Parsed for completeness (osu! format field)
    pub y: i32,
    pub time: i32,
    pub hit_type: i32,
    #[allow(dead_code)] // Parsed for completeness (osu! format field)
    pub hit_sound: i32,
    pub object_params: Vec<String>,
    #[allow(dead_code)] // Parsed for completeness (osu! format field)
    pub hit_sample: HitSample,
}

/// Parsed .osu file
#[derive(Debug, Default)]
pub struct Osu {
    pub general: General,
    pub metadata: Metadata,
    pub difficulty: Difficulty,
    pub events: Vec<Events>,
    pub timing_points: Vec<TimingPoints>,
    pub hit_objects: Vec<HitObjects>,
}

impl Osu {
    /// Parse an .osu file from a buffered reader.
    pub fn parse<R: BufRead>(reader: R) -> Self {
        let mut osu = Osu::default();
        osu.general.preview_time = -1;
        osu.general.countdown = 1;
        osu.general.sample_set = "Normal".to_string();
        osu.general.stack_leniency = 0.7;

        let mut section = String::new();

        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => continue,
            };
            let line = line.trim().to_string();
            if line.is_empty() || line.starts_with("//") {
                continue;
            }

            // Section header
            if line.starts_with('[') && line.ends_with(']') {
                section = line[1..line.len() - 1].to_string();
                continue;
            }

            match section.as_str() {
                "General" => Self::parse_general(&mut osu.general, &line),
                "Metadata" => Self::parse_metadata(&mut osu.metadata, &line),
                "Difficulty" => Self::parse_difficulty(&mut osu.difficulty, &line),
                "Events" => Self::parse_events(&mut osu.events, &line),
                "TimingPoints" => Self::parse_timing_point(&mut osu.timing_points, &line),
                "HitObjects" => Self::parse_hit_object(&mut osu.hit_objects, &line),
                _ => {}
            }
        }

        osu
    }

    fn parse_general(general: &mut General, line: &str) {
        let Some((key, value)) = line.split_once(':') else {
            return;
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "AudioFilename" => general.audio_filename = value.to_string(),
            "AudioLeadIn" => general.audio_lead_in = value.parse().unwrap_or(0),
            "PreviewTime" => general.preview_time = value.parse().unwrap_or(-1),
            "Countdown" => general.countdown = value.parse().unwrap_or(1),
            "SampleSet" => general.sample_set = value.to_string(),
            "StackLeniency" => general.stack_leniency = value.parse().unwrap_or(0.7),
            "Mode" => general.mode = value.parse().unwrap_or(0),
            "LetterboxInBreaks" => {
                general.letterbox_in_breaks = value.parse::<i32>().unwrap_or(0) >= 1
            }
            "WidescreenStoryboard" => {
                general.widescreen_storyboard = value.parse::<i32>().unwrap_or(0) >= 1
            }
            "SpecialStyle" => general.special_style = value.parse::<i32>().unwrap_or(0) >= 1,
            _ => {}
        }
    }

    fn parse_metadata(metadata: &mut Metadata, line: &str) {
        let Some((key, value)) = line.split_once(':') else {
            return;
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "Title" => metadata.title = value.to_string(),
            "TitleUnicode" => metadata.title_unicode = value.to_string(),
            "Artist" => metadata.artist = value.to_string(),
            "ArtistUnicode" => metadata.artist_unicode = value.to_string(),
            "Creator" => metadata.creator = value.to_string(),
            "Version" => metadata.version = value.to_string(),
            "Source" => metadata.source = value.to_string(),
            "Tags" => metadata.tags = value.split_whitespace().map(|s| s.to_string()).collect(),
            "BeatmapID" => metadata.beatmap_id = value.parse().unwrap_or(0),
            "BeatmapSetID" => metadata.beatmap_set_id = value.parse().unwrap_or(0),
            _ => {}
        }
    }

    fn parse_difficulty(difficulty: &mut Difficulty, line: &str) {
        let Some((key, value)) = line.split_once(':') else {
            return;
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "HPDrainRate" => difficulty.hp_drain_rate = value.parse().unwrap_or(0.0),
            "CircleSize" => difficulty.circle_size = value.parse().unwrap_or(0.0),
            "OverallDifficulty" => difficulty.overall_difficulty = value.parse().unwrap_or(0.0),
            "ApproachRate" => difficulty.approach_rate = value.parse().unwrap_or(0.0),
            "SliderMultiplier" => difficulty.slider_multiplier = value.parse().unwrap_or(0.0),
            "SliderTickRate" => difficulty.slider_tick_rate = value.parse().unwrap_or(0.0),
            _ => {}
        }
    }

    fn parse_events(events: &mut Vec<Events>, line: &str) {
        let parts: Vec<&str> = line.splitn(3, ',').collect();
        if parts.len() < 2 {
            return;
        }
        let event_type = parts[0].trim().to_string();
        let start_time = parts[1].trim().parse().unwrap_or(0);
        let event_params = if parts.len() > 2 {
            parts[2].split(',').map(|s| s.trim().to_string()).collect()
        } else {
            Vec::new()
        };
        events.push(Events {
            event_type,
            start_time,
            event_params,
        });
    }

    fn parse_timing_point(timing_points: &mut Vec<TimingPoints>, line: &str) {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() < 2 {
            return;
        }
        let time: f32 = parts[0].trim().parse().unwrap_or(0.0);
        let beat_length: f32 = parts[1].trim().parse().unwrap_or(0.0);
        let meter = parts
            .get(2)
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(4);
        let sample_set = parts
            .get(3)
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0);
        let sample_index = parts
            .get(4)
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0);
        let volume = parts
            .get(5)
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(100);
        let uninherited = parts
            .get(6)
            .and_then(|s| s.trim().parse::<i32>().ok())
            .unwrap_or(1)
            >= 1;
        let effects = parts
            .get(7)
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0);

        timing_points.push(TimingPoints {
            time,
            beat_length,
            meter,
            sample_set,
            sample_index,
            volume,
            uninherited,
            effects,
        });
    }

    fn parse_hit_object(hit_objects: &mut Vec<HitObjects>, line: &str) {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() < 5 {
            return;
        }
        let x: i32 = parts[0].trim().parse().unwrap_or(0);
        let y: i32 = parts[1].trim().parse().unwrap_or(0);
        let time: i32 = parts[2].trim().parse().unwrap_or(0);
        let hit_type: i32 = parts[3].trim().parse().unwrap_or(0);
        let hit_sound: i32 = parts[4].trim().parse().unwrap_or(0);

        // For mania hold notes (type & 0x80), objectParams[0] contains end time
        // Format: x,y,time,type,hitSound,endTime:hitSample
        let is_hold = (hit_type & 0x80) > 0;
        let mut object_params = Vec::new();
        let mut hit_sample = HitSample::default();

        if parts.len() > 5 {
            if is_hold {
                // endTime:sampleSet:additionalSet:index:volume:filename
                let extra = parts[5];
                let sub_parts: Vec<&str> = extra.split(':').collect();
                if !sub_parts.is_empty() {
                    object_params.push(sub_parts[0].trim().to_string());
                }
                // Parse hit sample from remaining colon-separated values
                if sub_parts.len() > 1 {
                    hit_sample.normal_set = sub_parts[1].trim().parse().unwrap_or(0);
                }
                if sub_parts.len() > 2 {
                    hit_sample.additional_set = sub_parts[2].trim().parse().unwrap_or(0);
                }
                if sub_parts.len() > 3 {
                    hit_sample.index = sub_parts[3].trim().parse().unwrap_or(0);
                }
                if sub_parts.len() > 4 {
                    hit_sample.volume = sub_parts[4].trim().parse().unwrap_or(0);
                }
                if sub_parts.len() > 5 {
                    hit_sample.filename = sub_parts[5].trim().to_string();
                }
            } else {
                // hitSample format: sampleSet:additionalSet:index:volume:filename
                let sample_str = parts[5];
                let sub_parts: Vec<&str> = sample_str.split(':').collect();
                if !sub_parts.is_empty() {
                    hit_sample.normal_set = sub_parts[0].trim().parse().unwrap_or(0);
                }
                if sub_parts.len() > 1 {
                    hit_sample.additional_set = sub_parts[1].trim().parse().unwrap_or(0);
                }
                if sub_parts.len() > 2 {
                    hit_sample.index = sub_parts[2].trim().parse().unwrap_or(0);
                }
                if sub_parts.len() > 3 {
                    hit_sample.volume = sub_parts[3].trim().parse().unwrap_or(0);
                }
                if sub_parts.len() > 4 {
                    hit_sample.filename = sub_parts[4].trim().to_string();
                }
            }
        }

        hit_objects.push(HitObjects {
            x,
            y,
            time,
            hit_type,
            hit_sound,
            object_params,
            hit_sample,
        });
    }
}
