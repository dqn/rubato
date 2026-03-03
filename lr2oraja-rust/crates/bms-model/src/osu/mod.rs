use std::io::BufRead;

#[derive(Clone, Debug, Default)]
pub struct General {
    pub audio_filename: String,
    pub audio_lead_in: i32,
    pub audio_hash: String,
    pub preview_time: i32,
    pub countdown: i32,
    pub sample_set: String,
    pub stack_leniency: f32,
    pub mode: i32,
    pub letterbox_in_breaks: bool,
    pub story_fire_in_front: bool,
    pub use_skin_sprites: bool,
    pub always_show_playfield: bool,
    pub overlay_position: String,
    pub skin_preference: String,
    pub epilepsy_warning: bool,
    pub countdown_offset: i32,
    pub special_style: bool,
    pub widescreen_storyboard: bool,
    pub samples_match_playback_rate: bool,
}

impl General {
    pub fn new() -> Self {
        General {
            audio_filename: String::new(),
            audio_lead_in: 0,
            audio_hash: String::new(),
            preview_time: -1,
            countdown: 1,
            sample_set: "Normal".to_string(),
            stack_leniency: 0.7,
            mode: 0,
            letterbox_in_breaks: false,
            story_fire_in_front: true,
            use_skin_sprites: false,
            always_show_playfield: false,
            overlay_position: "NoChange".to_string(),
            skin_preference: String::new(),
            epilepsy_warning: false,
            countdown_offset: 0,
            special_style: false,
            widescreen_storyboard: false,
            samples_match_playback_rate: false,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Editor {
    pub bookmarks: Vec<i32>,
    pub distance_spacing: f32,
    pub beat_divisor: i32,
    pub grid_size: i32,
    pub timeline_zoom: f32,
}

#[derive(Clone, Debug, Default)]
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

#[derive(Clone, Debug, Default)]
pub struct Difficulty {
    pub hp_drain_rate: f32,
    pub circle_size: f32,
    pub overall_difficulty: f32,
    pub approach_rate: f32,
    pub slider_multiplier: f32,
    pub slider_tick_rate: f32,
}

#[derive(Clone, Debug, Default)]
pub struct Events {
    pub event_type: String,
    pub start_time: i32,
    pub event_params: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct TimingPoints {
    pub time: f32,
    pub beat_length: f32,
    pub meter: i32,
    pub sample_set: i32,
    pub sample_index: i32,
    pub volume: i32,
    pub uninherited: bool,
    pub effects: i32,
}

#[derive(Clone, Debug, Default)]
pub struct HitSample {
    pub normal_set: i32,
    pub additional_set: i32,
    pub index: i32,
    pub volume: i32,
    pub filename: String,
}

#[derive(Clone, Debug, Default)]
pub struct HitObjects {
    pub x: i32,
    pub y: i32,
    pub time: i32,
    pub hit_type: i32,
    pub hit_sound: i32,
    pub object_params: Vec<String>,
    pub hit_sample: HitSample,
}

#[derive(Clone, Debug, Default)]
pub struct RGB {
    pub red: i32,
    pub green: i32,
    pub blue: i32,
}

#[derive(Clone, Debug, Default)]
pub struct Colours {
    pub combo: Vec<RGB>,
    pub slider_track_override: Option<RGB>,
    pub slider_border: Option<RGB>,
}

pub struct Osu {
    pub general: General,
    pub editor: Editor,
    pub metadata: Metadata,
    pub difficulty: Difficulty,
    pub events: Vec<Events>,
    pub timing_points: Vec<TimingPoints>,
    pub colours: Colours,
    pub hit_objects: Vec<HitObjects>,
}

impl Osu {
    pub fn parse(reader: &mut impl BufRead) -> Self {
        let mut osu = Osu {
            general: General::new(),
            editor: Editor::default(),
            metadata: Metadata::default(),
            difficulty: Difficulty::default(),
            events: Vec::new(),
            timing_points: Vec::new(),
            colours: Colours::default(),
            hit_objects: Vec::new(),
        };

        let mut section = String::new();
        let mut line_buf = String::new();
        loop {
            line_buf.clear();
            match reader.read_line(&mut line_buf) {
                Ok(0) => break,
                Err(_) => break,
                _ => {}
            }
            let line = if let Some(idx) = line_buf.find("//") {
                &line_buf[..idx]
            } else {
                &line_buf
            };
            let line = line.trim_end_matches(['\n', '\r']);
            if line.len() < 2 {
                continue;
            }
            let line_no_space: String = line.chars().filter(|c| !c.is_whitespace()).collect();
            if line_no_space.starts_with('[') {
                // Use get() to avoid panics on malformed input (e.g. missing ']', multi-byte chars)
                if let Some(inner) = line_no_space.get(1..).and_then(|s| s.strip_suffix(']')) {
                    section = inner.to_string();
                } else if let Some(inner) = line_no_space.get(1..) {
                    section = inner.to_string();
                }
                continue;
            }

            let delimiter = line.find(':');
            let (key, value) = if let Some(d) = delimiter {
                let k = if d > 1 && line.as_bytes().get(d - 1) == Some(&b' ') {
                    &line[..d - 1]
                } else if d > 0 {
                    &line[..d]
                } else {
                    ""
                };
                let v = if line.len() <= d + 1 {
                    ""
                } else if line.as_bytes().get(d + 1) == Some(&b' ') {
                    &line[d + 2..]
                } else {
                    &line[d + 1..]
                };
                (k, v)
            } else {
                ("", line)
            };

            match section.as_str() {
                "General" => {
                    match key {
                        "AudioFilename" => osu.general.audio_filename = value.to_string(),
                        "AudioLeadIn" => {
                            if let Ok(v) = value.parse::<i32>() {
                                osu.general.audio_lead_in = v;
                            }
                        }
                        "AudioHash" => osu.general.audio_hash = value.to_string(),
                        "PreviewTime" => {
                            if let Ok(v) = value.parse::<i32>() {
                                osu.general.preview_time = v;
                            }
                        }
                        "Countdown" => {
                            if let Ok(v) = value.parse::<i32>() {
                                osu.general.countdown = v;
                            }
                        }
                        "SampleSet" => osu.general.sample_set = value.to_string(),
                        "StackLeniency" => {
                            if let Ok(v) = value.parse::<f32>() {
                                osu.general.stack_leniency = v;
                            }
                        }
                        "Mode" => {
                            if let Ok(v) = value.parse::<i32>() {
                                osu.general.mode = v;
                            }
                        }
                        "LetterboxInBreaks" => {
                            if let Ok(v) = value.parse::<i32>() {
                                osu.general.letterbox_in_breaks = v >= 1;
                            }
                            // Java fallthrough: also sets StoryFireInFront
                            // (This is a bug in the Java code but we preserve it)
                        }
                        "StoryFireInFront" => {
                            if let Ok(v) = value.parse::<i32>() {
                                osu.general.story_fire_in_front = v >= 1;
                            }
                        }
                        "UseSkinSprites" => {
                            if let Ok(v) = value.parse::<i32>() {
                                osu.general.use_skin_sprites = v >= 1;
                            }
                        }
                        "AlwaysShowPlayfield" => {
                            if let Ok(v) = value.parse::<i32>() {
                                osu.general.always_show_playfield = v >= 1;
                            }
                        }
                        "OverlayPosition" => osu.general.overlay_position = value.to_string(),
                        "SkinPreference" => osu.general.skin_preference = value.to_string(),
                        "EpilepsyWarning" => {
                            if let Ok(v) = value.parse::<i32>() {
                                osu.general.epilepsy_warning = v >= 1;
                            }
                        }
                        "CountdownOffset" => {
                            if let Ok(v) = value.parse::<i32>() {
                                osu.general.countdown_offset = v;
                            }
                        }
                        "SpecialStyle" => {
                            if let Ok(v) = value.parse::<i32>() {
                                osu.general.special_style = v >= 1;
                            }
                        }
                        "WidescreenStoryboard" => {
                            if let Ok(v) = value.parse::<i32>() {
                                osu.general.widescreen_storyboard = v >= 1;
                            }
                        }
                        "SamplesMatchPlaybackRate" => {
                            if let Ok(v) = value.parse::<i32>() {
                                osu.general.samples_match_playback_rate = v >= 1;
                            }
                        }
                        _ => {}
                    }
                    // Java: General case falls through to Editor (no break)
                    // Replicate: also try to parse as Editor
                    Self::parse_editor(&mut osu.editor, key, value);
                }
                "Editor" => {
                    Self::parse_editor(&mut osu.editor, key, value);
                }
                "Metadata" => match key {
                    "Title" => osu.metadata.title = value.to_string(),
                    "TitleUnicode" => osu.metadata.title_unicode = value.to_string(),
                    "Artist" => osu.metadata.artist = value.to_string(),
                    "ArtistUnicode" => osu.metadata.artist_unicode = value.to_string(),
                    "Creator" => osu.metadata.creator = value.to_string(),
                    "Version" => osu.metadata.version = value.to_string(),
                    "Source" => osu.metadata.source = value.to_string(),
                    "Tags" => {
                        osu.metadata.tags = value.split(' ').map(|s| s.to_string()).collect();
                    }
                    "BeatmapID" => {
                        if let Ok(v) = value.parse::<i32>() {
                            osu.metadata.beatmap_id = v;
                        }
                    }
                    "BeatmapSetID" => {
                        if let Ok(v) = value.parse::<i32>() {
                            osu.metadata.beatmap_set_id = v;
                        }
                    }
                    _ => {}
                },
                "Difficulty" => match key {
                    "HPDrainRate" => {
                        if let Ok(v) = value.parse::<f32>() {
                            osu.difficulty.hp_drain_rate = v;
                        }
                    }
                    "CircleSize" => {
                        if let Ok(v) = value.parse::<f32>() {
                            osu.difficulty.circle_size = v;
                        }
                    }
                    "OverallDifficulty" => {
                        if let Ok(v) = value.parse::<f32>() {
                            osu.difficulty.overall_difficulty = v;
                        }
                    }
                    "ApproachRate" => {
                        if let Ok(v) = value.parse::<f32>() {
                            osu.difficulty.approach_rate = v;
                        }
                    }
                    "SliderMultiplier" => {
                        if let Ok(v) = value.parse::<f32>() {
                            osu.difficulty.slider_multiplier = v;
                        }
                    }
                    "SliderTickRate" => {
                        if let Ok(v) = value.parse::<f32>() {
                            osu.difficulty.slider_tick_rate = v;
                        }
                    }
                    _ => {}
                },
                "Events" => {
                    let values: Vec<&str> = value.split(',').collect();
                    if values.len() < 3 {
                        continue;
                    }
                    let event = Events {
                        event_type: values[0].to_string(),
                        start_time: values[1].parse::<i32>().unwrap_or(0),
                        event_params: values[2..].iter().map(|v| v.replace('"', "")).collect(),
                    };
                    osu.events.push(event);
                }
                "TimingPoints" => {
                    let values: Vec<&str> = value.split(',').collect();
                    if values.len() < 6 {
                        continue;
                    }
                    let time = match values[0].parse::<f32>() {
                        Ok(v) => v,
                        Err(_) => continue,
                    };
                    let beat_length = match values[1].parse::<f32>() {
                        Ok(v) => v,
                        Err(_) => continue,
                    };
                    let meter = match values[2].parse::<i32>() {
                        Ok(v) => v,
                        Err(_) => continue,
                    };
                    let sample_set = match values[3].parse::<i32>() {
                        Ok(v) => v,
                        Err(_) => continue,
                    };
                    let sample_index = match values[4].parse::<i32>() {
                        Ok(v) => v,
                        Err(_) => continue,
                    };
                    let volume_val = match values[5].parse::<i32>() {
                        Ok(v) => v,
                        Err(_) => continue,
                    };
                    let uninherited = if values.len() > 6 {
                        values[6].parse::<i32>().map(|v| v >= 1).unwrap_or(true)
                    } else {
                        true
                    };
                    let effects = if values.len() > 7 {
                        values[7].parse::<i32>().unwrap_or(0)
                    } else {
                        0
                    };
                    let tp = TimingPoints {
                        time,
                        beat_length,
                        meter,
                        sample_set,
                        sample_index,
                        volume: volume_val,
                        uninherited,
                        effects,
                    };
                    osu.timing_points.push(tp);
                }
                "Colours" => {
                    let values: Vec<&str> = value.split(',').collect();
                    if values.len() < 3 {
                        continue;
                    }
                    let red = match values[0].trim().parse::<i32>() {
                        Ok(v) => v,
                        Err(_) => continue,
                    };
                    let green = match values[1].trim().parse::<i32>() {
                        Ok(v) => v,
                        Err(_) => continue,
                    };
                    let blue = match values[2].trim().parse::<i32>() {
                        Ok(v) => v,
                        Err(_) => continue,
                    };
                    let rgb = RGB { red, green, blue };
                    match key {
                        "SliderTrackOverride" => {
                            osu.colours.slider_track_override = Some(rgb);
                        }
                        "SliderBorder" => {
                            osu.colours.slider_border = Some(rgb);
                        }
                        _ => {
                            if key.starts_with("Combo") {
                                osu.colours.combo.push(rgb);
                            }
                        }
                    }
                }
                "HitObjects" => {
                    let mut full_value = value.to_string();
                    if !key.is_empty() && !key.starts_with(' ') {
                        full_value = format!("{}:{}", key, value);
                    }

                    let values: Vec<&str> = full_value.split(',').collect();
                    if values.len() < 6 {
                        continue;
                    }
                    let x = match values[0].parse::<i32>() {
                        Ok(v) => v,
                        Err(_) => continue,
                    };
                    let y_val = match values[1].parse::<i32>() {
                        Ok(v) => v,
                        Err(_) => continue,
                    };
                    let time_val = match values[2].parse::<i32>() {
                        Ok(v) => v,
                        Err(_) => continue,
                    };
                    let hit_type = match values[3].parse::<i32>() {
                        Ok(v) => v,
                        Err(_) => continue,
                    };
                    let hit_sound = match values[4].parse::<i32>() {
                        Ok(v) => v,
                        Err(_) => continue,
                    };

                    let mut hit_object = HitObjects {
                        x,
                        y: y_val,
                        time: time_val,
                        hit_type,
                        hit_sound,
                        object_params: Vec::new(),
                        hit_sample: HitSample::default(),
                    };

                    let is_mania_hold = (hit_type & 0x80) > 0;
                    let last_value = values[values.len() - 1];
                    if is_mania_hold {
                        let hit_sample_values: Vec<&str> = last_value.split(':').collect();
                        hit_object
                            .object_params
                            .push(hit_sample_values[0].to_string());
                        if hit_sample_values.len() < 5 {
                            continue;
                        }
                        let normal_set = match hit_sample_values[1].parse::<i32>() {
                            Ok(v) => v,
                            Err(_) => continue,
                        };
                        let additional_set = match hit_sample_values[2].parse::<i32>() {
                            Ok(v) => v,
                            Err(_) => continue,
                        };
                        let index_val = match hit_sample_values[3].parse::<i32>() {
                            Ok(v) => v,
                            Err(_) => continue,
                        };
                        let volume_val = match hit_sample_values[4].parse::<i32>() {
                            Ok(v) => v,
                            Err(_) => continue,
                        };
                        hit_object.hit_sample.normal_set = normal_set;
                        hit_object.hit_sample.additional_set = additional_set;
                        hit_object.hit_sample.index = index_val;
                        hit_object.hit_sample.volume = volume_val;
                        if last_value.ends_with(':') {
                            hit_object.hit_sample.filename = String::new();
                        } else if hit_sample_values.len() > 5 {
                            hit_object.hit_sample.filename = hit_sample_values[5].to_string();
                        }
                    } else {
                        let hit_sample_values: Vec<&str> = last_value.split(':').collect();
                        if hit_sample_values.len() < 4 {
                            continue;
                        }
                        let normal_set = match hit_sample_values[0].parse::<i32>() {
                            Ok(v) => v,
                            Err(_) => continue,
                        };
                        let additional_set = match hit_sample_values[1].parse::<i32>() {
                            Ok(v) => v,
                            Err(_) => continue,
                        };
                        let index_val = match hit_sample_values[2].parse::<i32>() {
                            Ok(v) => v,
                            Err(_) => continue,
                        };
                        let volume_val = match hit_sample_values[3].parse::<i32>() {
                            Ok(v) => v,
                            Err(_) => continue,
                        };
                        hit_object.hit_sample.normal_set = normal_set;
                        hit_object.hit_sample.additional_set = additional_set;
                        hit_object.hit_sample.index = index_val;
                        hit_object.hit_sample.volume = volume_val;
                        if last_value.ends_with(':') {
                            hit_object.hit_sample.filename = String::new();
                        } else if hit_sample_values.len() > 4 {
                            hit_object.hit_sample.filename = hit_sample_values[4].to_string();
                        }
                    }
                    osu.hit_objects.push(hit_object);
                }
                _ => {
                    continue;
                }
            }
        }

        osu
    }

    fn parse_editor(editor: &mut Editor, key: &str, value: &str) {
        match key {
            "Bookmarks" => {
                editor.bookmarks = value
                    .split(',')
                    .filter_map(|v| v.parse::<i32>().ok())
                    .collect();
            }
            "DistanceSpacing" => {
                if let Ok(v) = value.parse::<f32>() {
                    editor.distance_spacing = v;
                }
            }
            "BeatDivisor" => {
                if let Ok(v) = value.parse::<i32>() {
                    editor.beat_divisor = v;
                }
            }
            "GridSize" => {
                if let Ok(v) = value.parse::<i32>() {
                    editor.grid_size = v;
                }
            }
            "TimelineZoom" => {
                if let Ok(v) = value.parse::<f32>() {
                    editor.timeline_zoom = v;
                }
            }
            _ => {}
        }
    }
}
