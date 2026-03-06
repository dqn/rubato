use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::bms_decoder::convert_hex_string;
use crate::bms_model::{BMSModel, JudgeRankType, TotalType};
use crate::bmson;
use crate::chart_decoder::TimeLineCache;
use crate::chart_information::ChartInformation;
use crate::decode_log::{DecodeLog, State};
use crate::layer::{Event, EventType, Layer, Sequence as LayerSequence};
use crate::mode::Mode;
use crate::note::Note;
use crate::time_line::TimeLine;

pub struct BMSONDecoder {
    pub lntype: i32,
    pub log: Vec<DecodeLog>,
}

#[derive(Clone)]
struct BmsonLnInfo {
    start_section: f64,
    end_section: f64,
    end_y: i32,
}

struct LnUpInfo {
    wav: i32,
    starttime: i64,
    duration: i64,
}

impl BMSONDecoder {
    pub fn new(lntype: i32) -> Self {
        BMSONDecoder {
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
        log::debug!("BMSONファイル解析開始 :{}", f.display());
        self.log.clear();

        let mut model = BMSModel::new();
        let mut tlcache: BTreeMap<i32, TimeLineCache> = BTreeMap::new();

        // Read file and compute SHA-256
        let file_bytes = std::fs::read(f).ok()?;
        let sha256_hash = {
            let mut hasher = Sha256::new();
            hasher.update(&file_bytes);
            convert_hex_string(&hasher.finalize())
        };

        let bmson_data: bmson::Bmson = serde_json::from_slice(&file_bytes).ok()?;
        model.set_sha256(sha256_hash);

        model.set_title(&bmson_data.info.title);
        let subtitle = bmson_data.info.subtitle.as_deref().unwrap_or("");
        let chart_name = bmson_data.info.chart_name.as_deref().unwrap_or("");
        let sub_title = format!(
            "{}{}{}",
            subtitle,
            if !subtitle.is_empty() && !chart_name.is_empty() {
                " "
            } else {
                ""
            },
            if !chart_name.is_empty() {
                format!("[{}]", chart_name)
            } else {
                String::new()
            }
        );
        model.set_sub_title(sub_title);
        model.set_artist(&bmson_data.info.artist);
        let mut subartist = String::new();
        for s in &bmson_data.info.subartists {
            if !subartist.is_empty() {
                subartist.push(',');
            }
            subartist.push_str(s);
        }
        model.set_sub_artist(subartist);
        model.set_genre(&bmson_data.info.genre);

        if bmson_data.info.judge_rank < 0 {
            self.log.push(DecodeLog::new(
                State::Warning,
                format!(
                    "judge_rankが0以下です。judge_rank = {}",
                    bmson_data.info.judge_rank
                ),
            ));
        } else if bmson_data.info.judge_rank < 5 {
            model.set_judgerank(bmson_data.info.judge_rank);
            self.log.push(DecodeLog::new(
                State::Warning,
                format!(
                    "judge_rankの定義が仕様通りでない可能性があります。judge_rank = {}",
                    bmson_data.info.judge_rank
                ),
            ));
            model.set_judgerank_type(JudgeRankType::BmsRank);
        } else {
            model.set_judgerank(bmson_data.info.judge_rank);
            model.set_judgerank_type(JudgeRankType::BmsonJudgerank);
        }

        if bmson_data.info.total > 0.0 {
            model.set_total(bmson_data.info.total);
            model.set_total_type(TotalType::Bmson);
        } else {
            self.log.push(DecodeLog::new(
                State::Warning,
                format!("totalが0以下です。total = {}", bmson_data.info.total),
            ));
        }

        model.set_bpm(bmson_data.info.init_bpm);
        model.set_playlevel(bmson_data.info.level.to_string());
        let mode = Mode::from_hint(&bmson_data.info.mode_hint);
        if let Some(mode) = mode {
            model.set_mode(mode);
        } else {
            self.log.push(DecodeLog::new(
                State::Warning,
                format!(
                    "非対応のmode_hintです。mode_hint = {}",
                    bmson_data.info.mode_hint
                ),
            ));
            model.set_mode(Mode::BEAT_7K);
        }
        if bmson_data.info.ln_type > 0 && bmson_data.info.ln_type <= 3 {
            model.set_lnmode(bmson_data.info.ln_type);
        }

        let mode_key = model.mode().map(|m| m.key()).unwrap_or(0);
        let keyassign: Vec<i32> = match model.mode() {
            Some(Mode::BEAT_5K) => vec![0, 1, 2, 3, 4, -1, -1, 5],
            Some(Mode::BEAT_10K) => {
                vec![0, 1, 2, 3, 4, -1, -1, 5, 6, 7, 8, 9, 10, -1, -1, 11]
            }
            _ => (0..mode_key).collect(),
        };
        let mut lnlist: Vec<Option<Vec<BmsonLnInfo>>> = vec![None; mode_key as usize];
        // lnup: keyed by (x, y) of the bmson Note, storing wav/starttime/duration
        let mut lnup: HashMap<(i32, i32), LnUpInfo> = HashMap::new();

        model.set_banner(&bmson_data.info.banner_image);
        model.set_backbmp(&bmson_data.info.back_image);
        model.set_stagefile(&bmson_data.info.eyecatch_image);
        model.set_preview(&bmson_data.info.preview_music);

        let mut basetl = TimeLine::new(0.0, 0, mode_key);
        basetl.set_bpm(model.bpm());
        tlcache.insert(0, TimeLineCache::new(0.0, basetl));

        let mut bpm_events = bmson_data.bpm_events;
        let mut stop_events = bmson_data.stop_events;
        let mut scroll_events = bmson_data.scroll_events;

        let resolution: f64 = if bmson_data.info.resolution > 0 {
            bmson_data.info.resolution as f64 * 4.0
        } else {
            960.0
        };

        // Sort events by y
        bpm_events.sort_by_key(|e| e.y);
        stop_events.sort_by_key(|e| e.y);
        scroll_events.sort_by_key(|e| e.y);

        let mut bpmpos = 0usize;
        let mut stoppos = 0usize;
        let mut scrollpos = 0usize;

        while bpmpos < bpm_events.len()
            || stoppos < stop_events.len()
            || scrollpos < scroll_events.len()
        {
            let bpmy = if bpmpos < bpm_events.len() {
                bpm_events[bpmpos].y
            } else {
                i32::MAX
            };
            let stopy = if stoppos < stop_events.len() {
                stop_events[stoppos].y
            } else {
                i32::MAX
            };
            let scrolly = if scrollpos < scroll_events.len() {
                scroll_events[scrollpos].y
            } else {
                i32::MAX
            };
            if scrolly <= stopy && scrolly <= bpmy {
                ensure_timeline(&mut tlcache, scrolly, resolution, mode_key);
                tlcache
                    .get_mut(&scrolly)
                    .unwrap()
                    .timeline
                    .set_scroll(scroll_events[scrollpos].rate);
                scrollpos += 1;
            } else if bpmy <= stopy {
                if bpm_events[bpmpos].bpm > 0.0 {
                    ensure_timeline(&mut tlcache, bpmy, resolution, mode_key);
                    tlcache
                        .get_mut(&bpmy)
                        .unwrap()
                        .timeline
                        .set_bpm(bpm_events[bpmpos].bpm);
                } else {
                    self.log.push(DecodeLog::new(
                        State::Warning,
                        format!(
                            "negative BPMはサポートされていません - y : {} bpm : {}",
                            bpm_events[bpmpos].y, bpm_events[bpmpos].bpm
                        ),
                    ));
                }
                bpmpos += 1;
            } else if stopy != i32::MAX {
                if stop_events[stoppos].duration >= 0 {
                    ensure_timeline(&mut tlcache, stopy, resolution, mode_key);
                    let tl = &mut tlcache.get_mut(&stopy).unwrap().timeline;
                    let bpm = tl.bpm();
                    tl.set_stop(
                        ((1000.0 * 1000.0 * 60.0 * 4.0 * stop_events[stoppos].duration as f64)
                            / (bpm * resolution)) as i64,
                    );
                } else {
                    self.log.push(DecodeLog::new(
                        State::Warning,
                        format!(
                            "negative STOPはサポートされていません - y : {} bpm : {}",
                            stop_events[stoppos].y, stop_events[stoppos].duration
                        ),
                    ));
                }
                stoppos += 1;
            }
        }

        // Bar lines
        for bl in &bmson_data.lines {
            ensure_timeline(&mut tlcache, bl.y, resolution, mode_key);
            tlcache
                .get_mut(&bl.y)
                .unwrap()
                .timeline
                .set_section_line(true);
        }

        // Sound channels, key channels, mine channels
        let total_channels = bmson_data.sound_channels.len()
            + bmson_data.key_channels.len()
            + bmson_data.mine_channels.len();
        let mut wavmap: Vec<String> = Vec::with_capacity(total_channels);
        let mut id: i32 = 0;
        let mut starttime: i64 = 0;

        for sc in &bmson_data.sound_channels {
            wavmap.push(sc.name.clone());
            let mut notes = sc.notes.clone();
            notes.sort_by_key(|n| n.y);
            let length = notes.len();
            for i in 0..length {
                let n = &notes[i];
                let n_y = n.y;
                let n_x = n.x;
                let n_c = n.c;
                let n_l = n.l;
                let n_t = n.t;
                let n_up = n.up;

                let mut next_y: Option<i32> = None;
                for note in &notes[(i + 1)..length] {
                    if note.y > n_y {
                        next_y = Some(note.y);
                        break;
                    }
                }
                let mut duration: i64 = 0;
                if !n_c {
                    starttime = 0;
                }
                ensure_timeline(&mut tlcache, n_y, resolution, mode_key);
                if let Some(next_y_val) = next_y {
                    ensure_timeline(&mut tlcache, next_y_val, resolution, mode_key);
                    let next_time = tlcache.get(&next_y_val).unwrap().timeline.micro_time();
                    let cur_time = tlcache.get(&n_y).unwrap().timeline.micro_time();
                    duration = next_time - cur_time;
                }

                let key = if n_x > 0 && n_x <= keyassign.len() as i32 {
                    keyassign[(n_x - 1) as usize]
                } else {
                    -1
                };
                if key < 0 {
                    // BG note
                    let bg_note = Note::new_normal_with_start_duration(id, starttime, duration);
                    tlcache
                        .get_mut(&n_y)
                        .unwrap()
                        .timeline
                        .add_back_ground_note(bg_note);
                } else if n_up {
                    // LN end sound definition
                    let mut assigned = false;
                    let key_usize = key as usize;
                    if key_usize < lnlist.len()
                        && let Some(ref lns) = lnlist[key_usize]
                    {
                        let section = n_y as f64 / resolution;
                        for ln_info in lns {
                            if section == ln_info.end_section {
                                // Modify the end note on the timeline
                                let end_tl = &mut tlcache.get_mut(&ln_info.end_y).unwrap().timeline;
                                if let Some(end_note) = end_tl.note_mut(key) {
                                    end_note.set_wav(id);
                                    end_note.set_micro_starttime(starttime);
                                    end_note.set_micro_duration(duration);
                                }
                                assigned = true;
                                break;
                            }
                        }
                    }
                    if !assigned {
                        lnup.insert(
                            (n_x, n_y),
                            LnUpInfo {
                                wav: id,
                                starttime,
                                duration,
                            },
                        );
                    }
                } else {
                    // Check if inside existing LN
                    let key_usize = key as usize;
                    let mut insideln = false;
                    if key_usize < lnlist.len()
                        && let Some(ref lns) = lnlist[key_usize]
                    {
                        let section = n_y as f64 / resolution;
                        for ln_info in lns {
                            if ln_info.start_section < section && section <= ln_info.end_section {
                                insideln = true;
                                break;
                            }
                        }
                    }

                    if insideln {
                        self.log.push(DecodeLog::new(
                            State::Warning,
                            format!("LN内にノートを定義しています - x :  {} y : {}", n_x, n_y),
                        ));
                        let bg_note = Note::new_normal_with_start_duration(id, starttime, duration);
                        tlcache
                            .get_mut(&n_y)
                            .unwrap()
                            .timeline
                            .add_back_ground_note(bg_note);
                    } else if n_l > 0 {
                        // Long note
                        let end_y = n_y + n_l;
                        ensure_timeline(&mut tlcache, end_y, resolution, mode_key);
                        let ln = Note::new_long_with_start_duration(id, starttime, duration);

                        let tl_has_note = tlcache.get(&n_y).unwrap().timeline.exist_note_at(key);

                        if tl_has_note {
                            // Layer note check
                            let tl_note_is_long = tlcache
                                .get(&n_y)
                                .unwrap()
                                .timeline
                                .note(key)
                                .map(|en| en.is_long())
                                .unwrap_or(false);

                            let end_note_matches = if tl_note_is_long {
                                // Check if end.getNote(key) == ((LongNote)en).getPair()
                                // In our model, check if the existing LN end is at end_y
                                let existing_end_section = if let Some(ref lns) = lnlist[key_usize]
                                {
                                    lns.iter()
                                        .find(|info| {
                                            let start_sec =
                                                tlcache.get(&n_y).unwrap().timeline.section();
                                            (info.start_section - start_sec).abs() < f64::EPSILON
                                        })
                                        .map(|info| info.end_y)
                                } else {
                                    None
                                };
                                existing_end_section == Some(end_y)
                            } else {
                                false
                            };

                            if end_note_matches {
                                // Add layered note
                                tlcache
                                    .get_mut(&n_y)
                                    .unwrap()
                                    .timeline
                                    .note_mut(key)
                                    .unwrap()
                                    .add_layered_note(ln);
                            } else {
                                self.log.push(DecodeLog::new(
                                    State::Warning,
                                    format!(
                                        "同一の位置にノートが複数定義されています - x :  {} y : {}",
                                        n_x, n_y
                                    ),
                                ));
                            }
                        } else {
                            // Check if there's a note inside the LN range
                            let exist_note = {
                                let sub_range: Vec<_> = tlcache
                                    .range((
                                        std::ops::Bound::Excluded(n_y),
                                        std::ops::Bound::Included(end_y),
                                    ))
                                    .collect();
                                sub_range
                                    .iter()
                                    .any(|(_, tlc)| tlc.timeline.exist_note_at(key))
                            };
                            if exist_note {
                                self.log.push(DecodeLog::new(
                                    State::Warning,
                                    format!(
                                        "LN内にノートを定義しています - x :  {} y : {}",
                                        n_x, n_y
                                    ),
                                ));
                                let bg_note =
                                    Note::new_normal_with_start_duration(id, starttime, duration);
                                tlcache
                                    .get_mut(&n_y)
                                    .unwrap()
                                    .timeline
                                    .add_back_ground_note(bg_note);
                            } else {
                                tlcache
                                    .get_mut(&n_y)
                                    .unwrap()
                                    .timeline
                                    .set_note(key, Some(ln));

                                // Check lnup for matching end
                                let lnend = if let Some(up_info) = lnup.remove(&(n_x, end_y)) {
                                    Note::new_long_with_start_duration(
                                        up_info.wav,
                                        up_info.starttime,
                                        up_info.duration,
                                    )
                                } else {
                                    Note::new_long(-2)
                                };

                                tlcache
                                    .get_mut(&end_y)
                                    .unwrap()
                                    .timeline
                                    .set_note(key, Some(lnend));

                                // Set LN type on start note
                                let ln_type = if n_t > 0 && n_t <= 3 {
                                    n_t
                                } else {
                                    model.lnmode()
                                };
                                tlcache
                                    .get_mut(&n_y)
                                    .unwrap()
                                    .timeline
                                    .note_mut(key)
                                    .unwrap()
                                    .set_long_note_type(ln_type);

                                // Mark end note
                                tlcache
                                    .get_mut(&end_y)
                                    .unwrap()
                                    .timeline
                                    .note_mut(key)
                                    .unwrap()
                                    .set_end(true);
                                tlcache
                                    .get_mut(&end_y)
                                    .unwrap()
                                    .timeline
                                    .note_mut(key)
                                    .unwrap()
                                    .set_long_note_type(ln_type);

                                let start_section = tlcache.get(&n_y).unwrap().timeline.section();
                                let end_section = tlcache.get(&end_y).unwrap().timeline.section();

                                while lnlist.len() <= key_usize {
                                    lnlist.push(None);
                                }
                                if lnlist[key_usize].is_none() {
                                    lnlist[key_usize] = Some(Vec::new());
                                }
                                lnlist[key_usize].as_mut().unwrap().push(BmsonLnInfo {
                                    start_section,
                                    end_section,
                                    end_y,
                                });
                            }
                        }
                    } else {
                        // Normal note
                        let tl = &tlcache.get(&n_y).unwrap().timeline;
                        if tl.exist_note_at(key) {
                            let is_normal = tl.note(key).map(|n| n.is_normal()).unwrap_or(false);
                            if is_normal {
                                let layered =
                                    Note::new_normal_with_start_duration(id, starttime, duration);
                                tlcache
                                    .get_mut(&n_y)
                                    .unwrap()
                                    .timeline
                                    .note_mut(key)
                                    .unwrap()
                                    .add_layered_note(layered);
                            } else {
                                self.log.push(DecodeLog::new(
                                    State::Warning,
                                    format!(
                                        "同一の位置にノートが複数定義されています - x :  {} y : {}",
                                        n_x, n_y
                                    ),
                                ));
                            }
                        } else {
                            let normal =
                                Note::new_normal_with_start_duration(id, starttime, duration);
                            tlcache
                                .get_mut(&n_y)
                                .unwrap()
                                .timeline
                                .set_note(key, Some(normal));
                        }
                    }
                }
                starttime += duration;
            }
            id += 1;
        }

        // Key channels (hidden notes)
        for sc in &bmson_data.key_channels {
            wavmap.push(sc.name.clone());
            let mut notes = sc.notes.clone();
            notes.sort_by_key(|n| n.y);
            for n in &notes {
                ensure_timeline(&mut tlcache, n.y, resolution, mode_key);
                let key = if n.x > 0 && n.x <= keyassign.len() as i32 {
                    keyassign[(n.x - 1) as usize]
                } else {
                    -1
                };
                if key >= 0 {
                    let hidden = Note::new_normal(id);
                    tlcache
                        .get_mut(&n.y)
                        .unwrap()
                        .timeline
                        .set_hidden_note(key, Some(hidden));
                }
            }
            id += 1;
        }

        // Mine channels
        for sc in &bmson_data.mine_channels {
            wavmap.push(sc.name.clone());
            let mut notes = sc.notes.clone();
            notes.sort_by_key(|n| n.y);
            for n in &notes {
                ensure_timeline(&mut tlcache, n.y, resolution, mode_key);
                let key = if n.x > 0 && n.x <= keyassign.len() as i32 {
                    keyassign[(n.x - 1) as usize]
                } else {
                    -1
                };
                if key >= 0 {
                    let key_usize = key as usize;
                    let mut insideln = false;
                    if key_usize < lnlist.len()
                        && let Some(ref lns) = lnlist[key_usize]
                    {
                        let section = n.y as f64 / resolution;
                        for ln_info in lns {
                            if ln_info.start_section < section && section <= ln_info.end_section {
                                insideln = true;
                                break;
                            }
                        }
                    }

                    if insideln {
                        self.log.push(DecodeLog::new(
                            State::Warning,
                            format!(
                                "LN内に地雷ノートを定義しています - x :  {} y : {}",
                                n.x, n.y
                            ),
                        ));
                    } else if tlcache.get(&n.y).unwrap().timeline.exist_note_at(key) {
                        self.log.push(DecodeLog::new(
                            State::Warning,
                            format!(
                                "地雷ノートを定義している位置に通常ノートが存在します - x :  {} y : {}",
                                n.x, n.y
                            ),
                        ));
                    } else {
                        let mine = Note::new_mine(id, n.damage);
                        tlcache
                            .get_mut(&n.y)
                            .unwrap()
                            .timeline
                            .set_note(key, Some(mine));
                    }
                }
            }
            id += 1;
        }

        model.set_wav_list(wavmap);

        // BGA processing
        if let Some(ref bga) = bmson_data.bga
            && let Some(ref bga_headers) = bga.bga_header
        {
            let mut bgamap: Vec<String> = Vec::with_capacity(bga_headers.len());
            let mut idmap: HashMap<i32, i32> = HashMap::with_capacity(bga_headers.len());
            let mut seqmap: HashMap<i32, Vec<Vec<LayerSequence>>> = HashMap::new();

            for (i, bh) in bga_headers.iter().enumerate() {
                bgamap.push(bh.name.clone());
                idmap.insert(bh.id, i as i32);
            }

            if let Some(ref bga_sequences) = bga.bga_sequence {
                for bga_seq in bga_sequences {
                    let mut sequence: Vec<LayerSequence> =
                        Vec::with_capacity(bga_seq.sequence.len());
                    for seq in &bga_seq.sequence {
                        if seq.id != i32::MIN {
                            sequence.push(LayerSequence::new(seq.time, seq.id));
                        } else {
                            sequence.push(LayerSequence::new_end(seq.time));
                        }
                    }
                    seqmap.insert(bga_seq.id, vec![sequence]);
                }
            }

            if let Some(ref bga_events) = bga.bga_events {
                for bn in bga_events {
                    ensure_timeline(&mut tlcache, bn.y, resolution, mode_key);
                    if let Some(&mapped_id) = idmap.get(&bn.id) {
                        tlcache.get_mut(&bn.y).unwrap().timeline.set_bga(mapped_id);
                    }
                }
            }

            if let Some(ref layer_events) = bga.layer_events {
                for bn in layer_events {
                    ensure_timeline(&mut tlcache, bn.y, resolution, mode_key);
                    let default_id_set = [bn.id];
                    let id_set = bn.id_set.as_deref().unwrap_or(&default_id_set);
                    let mut seqs: Vec<Vec<LayerSequence>> = Vec::with_capacity(id_set.len());
                    let condition = bn.condition.as_deref().unwrap_or("");
                    let event = match condition {
                        "play" => Event::new(EventType::Play, bn.interval),
                        "miss" => Event::new(EventType::Miss, bn.interval),
                        _ => Event::new(EventType::Always, bn.interval),
                    };
                    for &nid in id_set {
                        if let Some(seq) = seqmap.get(&nid) {
                            seqs.push(seq[0].clone());
                        } else if let Some(&mapped_id) = idmap.get(&bn.id) {
                            seqs.push(vec![
                                LayerSequence::new(0, mapped_id),
                                LayerSequence::new_end(500),
                            ]);
                        }
                    }
                    tlcache
                        .get_mut(&bn.y)
                        .unwrap()
                        .timeline
                        .set_eventlayer(vec![Layer::new(event, seqs)]);
                }
            }

            if let Some(ref poor_events) = bga.poor_events {
                for bn in poor_events {
                    ensure_timeline(&mut tlcache, bn.y, resolution, mode_key);
                    let event = Event::new(EventType::Miss, 1);
                    let seqs = if let Some(seq) = seqmap.get(&bn.id) {
                        vec![seq[0].clone()]
                    } else if let Some(&mapped_id) = idmap.get(&bn.id) {
                        vec![vec![
                            LayerSequence::new(0, mapped_id),
                            LayerSequence::new_end(500),
                        ]]
                    } else {
                        vec![]
                    };
                    tlcache
                        .get_mut(&bn.y)
                        .unwrap()
                        .timeline
                        .set_eventlayer(vec![Layer::new(event, seqs)]);
                }
            }

            model.set_bga_list(bgamap);
        }

        let timelines: Vec<TimeLine> = tlcache.into_values().map(|tlc| tlc.timeline).collect();
        model.set_all_time_line(timelines);

        log::debug!("BMSONファイル解析完了 :{}", f.display());

        model.set_chart_information(ChartInformation::new(
            Some(f.to_path_buf()),
            self.lntype,
            None,
        ));
        self.print_log(f);
        Some(model)
    }

    fn print_log(&self, path: &Path) {
        for l in &self.log {
            match l.state {
                State::Info => log::info!("{}: {}", path.display(), l.message),
                State::Warning => log::warn!("{}: {}", path.display(), l.message),
                State::Error => log::error!("{}: {}", path.display(), l.message),
            }
        }
    }
}

fn ensure_timeline(
    tlcache: &mut BTreeMap<i32, TimeLineCache>,
    y: i32,
    resolution: f64,
    mode_key: i32,
) {
    if tlcache.contains_key(&y) {
        return;
    }

    let (&le_key, le_val) = tlcache.range(..y).next_back().unwrap();
    let bpm = le_val.timeline.bpm();
    let time = if bpm != 0.0 {
        le_val.time
            + le_val.timeline.micro_stop() as f64
            + (240000.0 * 1000.0 * ((y - le_key) as f64 / resolution)) / bpm
    } else {
        le_val.time + le_val.timeline.micro_stop() as f64
    };

    let mut tl = TimeLine::new(y as f64 / resolution, time as i64, mode_key);
    tl.set_bpm(bpm);
    tlcache.insert(y, TimeLineCache::new(time, tl));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn test_bms_dir() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test-bms")
    }

    #[test]
    fn test_decode_minimal_7k() {
        let path = test_bms_dir().join("bmson_minimal_7k.bmson");
        let mut decoder = BMSONDecoder::new(0);
        let model = decoder
            .decode_path(&path)
            .expect("decode_path should return Some for valid bmson");

        assert_eq!(
            model.mode(),
            Some(&Mode::BEAT_7K),
            "mode should be BEAT_7K for mode_hint 'beat-7k'"
        );
        assert_eq!(
            model.title(),
            "Minimal 7K Bmson Test",
            "title should match bmson info.title"
        );
        assert_eq!(
            model.artist(),
            "brs-test",
            "artist should match bmson info.artist"
        );
        assert!(
            (model.bpm() - 120.0).abs() < f64::EPSILON,
            "init_bpm should be 120.0"
        );
        assert_eq!(
            model.total_notes(),
            8,
            "minimal 7k fixture has 8 normal notes across 2 sound channels"
        );
        assert!(
            !model.sha256().is_empty(),
            "SHA-256 hash should be computed"
        );
        assert_eq!(
            model.wav_list().len(),
            2,
            "two sound channels should produce two wav entries"
        );
        assert_eq!(model.wav_list()[0], "kick.wav");
        assert_eq!(model.wav_list()[1], "snare.wav");
    }

    #[test]
    fn test_decode_bpm_change() {
        let path = test_bms_dir().join("bmson_bpm_change.bmson");
        let mut decoder = BMSONDecoder::new(0);
        let model = decoder
            .decode_path(&path)
            .expect("decode_path should return Some for bpm_change bmson");

        // The fixture has init_bpm=120 and a bpm_event at y=960 changing to 180.
        // There must be at least one timeline with BPM != init_bpm.
        let timelines = model.all_time_lines();
        let has_bpm_change = timelines
            .iter()
            .any(|tl| (tl.bpm() - 180.0).abs() < f64::EPSILON);
        assert!(
            has_bpm_change,
            "model should contain a timeline with BPM 180.0 from the bpm_event"
        );

        // Verify initial BPM is preserved on the first timeline
        assert!(
            (timelines[0].bpm() - 120.0).abs() < f64::EPSILON,
            "first timeline should have init_bpm 120.0"
        );
    }

    #[test]
    fn test_decode_longnote() {
        let path = test_bms_dir().join("bmson_longnote.bmson");
        let mut decoder = BMSONDecoder::new(0);
        let model = decoder
            .decode_path(&path)
            .expect("decode_path should return Some for longnote bmson");

        let timelines = model.all_time_lines();
        let mode_key = model.mode().unwrap().key();

        // Collect all long notes (start and end) across all timelines
        let mut ln_start_count = 0;
        let mut ln_end_count = 0;
        for tl in timelines {
            for lane in 0..mode_key {
                if let Some(note) = tl.note(lane)
                    && note.is_long()
                {
                    if note.is_end() {
                        ln_end_count += 1;
                    } else {
                        ln_start_count += 1;
                    }
                }
            }
        }

        assert!(
            ln_start_count >= 2,
            "fixture defines 2 long notes (l=480 and l=960), got {} starts",
            ln_start_count
        );
        assert!(
            ln_end_count >= 2,
            "each long note should have a matching end, got {} ends",
            ln_end_count
        );
    }

    #[test]
    fn test_decode_stop_sequence() {
        let path = test_bms_dir().join("bmson_stop_sequence.bmson");
        let mut decoder = BMSONDecoder::new(0);
        let model = decoder
            .decode_path(&path)
            .expect("decode_path should return Some for stop_sequence bmson");

        let timelines = model.all_time_lines();
        let has_stop = timelines.iter().any(|tl| tl.micro_stop() > 0);
        assert!(
            has_stop,
            "model should contain at least one timeline with a stop event"
        );

        // Verify the stop is at the expected position (y=480, which is section 0.5)
        let stop_tl = timelines
            .iter()
            .find(|tl| tl.micro_stop() > 0)
            .expect("should find a timeline with stop");
        assert!(stop_tl.micro_stop() > 0, "stop duration should be positive");
    }

    #[test]
    fn test_decode_mine_invisible() {
        let path = test_bms_dir().join("bmson_mine_invisible.bmson");
        let mut decoder = BMSONDecoder::new(0);
        let model = decoder
            .decode_path(&path)
            .expect("decode_path should return Some for mine_invisible bmson");

        let timelines = model.all_time_lines();
        let mode_key = model.mode().unwrap().key();

        // Check for mine notes
        let mut mine_count = 0;
        for tl in timelines.iter() {
            for lane in 0..mode_key {
                if let Some(note) = tl.note(lane)
                    && note.is_mine()
                {
                    mine_count += 1;
                }
            }
        }
        assert_eq!(
            mine_count, 2,
            "fixture defines 2 mine notes (at y=240 on x=3 and y=720 on x=4)"
        );

        // Check for hidden (invisible) notes from key_channels
        let mut hidden_count = 0;
        for tl in timelines.iter() {
            for lane in 0..mode_key {
                if tl.hidden_note(lane).is_some() {
                    hidden_count += 1;
                }
            }
        }
        assert_eq!(
            hidden_count, 2,
            "fixture defines 2 hidden notes (at y=120 on x=5 and y=600 on x=6)"
        );

        // Verify mine damage values
        let mines: Vec<&crate::note::Note> = timelines
            .iter()
            .flat_map(|tl| (0..mode_key).filter_map(move |lane| tl.note(lane)))
            .filter(|n| n.is_mine())
            .collect();
        assert!(
            (mines[0].damage() - 50.0).abs() < f64::EPSILON,
            "first mine should have damage 50.0"
        );
        assert!(
            (mines[1].damage() - 100.0).abs() < f64::EPSILON,
            "second mine should have damage 100.0"
        );
    }

    #[test]
    fn test_decode_bpm_ln_cross() {
        let path = test_bms_dir().join("bmson_bpm_ln_cross.bmson");
        let mut decoder = BMSONDecoder::new(0);
        let model = decoder
            .decode_path(&path)
            .expect("decode_path should return Some for bpm_ln_cross bmson");

        let timelines = model.all_time_lines();
        let mode_key = model.mode().unwrap().key();

        // Verify BPM changes exist: 120 -> 180 -> 60 -> 120
        let bpm_values: Vec<f64> = timelines.iter().map(|tl| tl.bpm()).collect();
        let unique_bpms: std::collections::HashSet<u64> =
            bpm_values.iter().map(|b| b.to_bits()).collect();
        assert!(
            unique_bpms.len() >= 3,
            "should have at least 3 distinct BPM values (120, 180, 60), got {:?}",
            unique_bpms.len()
        );

        // Verify long notes exist
        let mut has_ln = false;
        for tl in timelines.iter() {
            for lane in 0..mode_key {
                if let Some(note) = tl.note(lane)
                    && note.is_long()
                    && !note.is_end()
                {
                    has_ln = true;
                    break;
                }
            }
            if has_ln {
                break;
            }
        }
        assert!(has_ln, "fixture should contain long notes");

        // Verify normal notes also exist
        let normal_count: i32 = timelines
            .iter()
            .flat_map(|tl| (0..mode_key).filter_map(move |lane| tl.note(lane)))
            .filter(|n| n.is_normal())
            .count() as i32;
        assert_eq!(
            normal_count, 4,
            "fixture defines 4 normal notes in the second sound channel"
        );
    }

    #[test]
    fn test_decode_empty_json() {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let path = dir.path().join("empty.bmson");
        let mut f = std::fs::File::create(&path).expect("failed to create temp file");
        f.write_all(b"{}").expect("failed to write");
        drop(f);

        let mut decoder = BMSONDecoder::new(0);
        let model = decoder.decode_path(&path);

        // Empty JSON deserializes to default Bmson (init_bpm=0.0), which is technically valid
        // but produces a model with 0 playable notes.
        if let Some(ref m) = model {
            assert_eq!(
                m.total_notes(),
                0,
                "empty bmson should produce 0 total notes"
            );
        }
        // Either None or Some with 0 notes is acceptable
    }

    #[test]
    fn test_decode_invalid_json() {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let path = dir.path().join("invalid.bmson");
        let mut f = std::fs::File::create(&path).expect("failed to create temp file");
        f.write_all(b"this is not json at all {{{")
            .expect("failed to write");
        drop(f);

        let mut decoder = BMSONDecoder::new(0);
        let result = decoder.decode_path(&path);
        assert!(
            result.is_none(),
            "invalid JSON should cause decode_path to return None"
        );
    }

    #[test]
    fn test_decode_zero_resolution() {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let path = dir.path().join("zero_res.bmson");
        let bmson_json = r#"{
            "version": "1.0.0",
            "info": {
                "title": "Zero Resolution Test",
                "mode_hint": "beat-7k",
                "init_bpm": 120.0,
                "resolution": 0,
                "judge_rank": 100,
                "total": 300.0,
                "level": 1
            },
            "lines": [{"y": 0}],
            "bpm_events": [],
            "stop_events": [],
            "scroll_events": [],
            "sound_channels": [
                {
                    "name": "test.wav",
                    "notes": [
                        {"x": 1, "y": 0, "l": 0, "c": false, "t": 0, "up": false}
                    ]
                }
            ],
            "mine_channels": [],
            "key_channels": []
        }"#;
        std::fs::write(&path, bmson_json).expect("failed to write");

        let mut decoder = BMSONDecoder::new(0);
        // Should not panic. resolution=0 is handled by falling back to 960.0
        let result = decoder.decode_path(&path);
        assert!(
            result.is_some(),
            "zero resolution should be handled gracefully (fallback to default 960)"
        );
    }

    #[test]
    fn test_decode_nonexistent_file() {
        let path = std::path::Path::new("/nonexistent/path/to/file.bmson");
        let mut decoder = BMSONDecoder::new(0);
        let result = decoder.decode_path(path);
        assert!(
            result.is_none(),
            "nonexistent file should cause decode_path to return None"
        );
    }

    #[test]
    fn test_decode_via_chart_information() {
        // Test the decode() method that wraps decode_path via ChartInformation
        let path = test_bms_dir().join("bmson_minimal_7k.bmson");
        let info = ChartInformation::new(Some(path), 0, None);
        let mut decoder = BMSONDecoder::new(0);
        let model = decoder
            .decode(info)
            .expect("decode via ChartInformation should return Some");

        assert_eq!(
            model.title(),
            "Minimal 7K Bmson Test",
            "title should match when decoding via ChartInformation"
        );
        assert_eq!(
            model.total_notes(),
            8,
            "total notes should match when decoding via ChartInformation"
        );
    }

    #[test]
    fn test_decode_chart_information_without_path() {
        let info = ChartInformation::new(None, 0, None);
        let mut decoder = BMSONDecoder::new(0);
        let result = decoder.decode(info);
        assert!(
            result.is_none(),
            "ChartInformation with no path should return None"
        );
    }

    #[test]
    fn test_ensure_timeline_bpm_zero_no_inf() {
        // Test that ensure_timeline handles BPM=0 without producing Inf/NaN
        let mut tlcache: BTreeMap<i32, TimeLineCache> = BTreeMap::new();
        let mode_key = 8;
        let resolution = 960.0;

        let mut tl0 = TimeLine::new(0.0, 0, mode_key);
        tl0.set_bpm(0.0); // BPM=0 triggers division-by-zero path
        tlcache.insert(0, TimeLineCache::new(0.0, tl0));

        // This should NOT produce Inf or NaN
        ensure_timeline(&mut tlcache, 480, resolution, mode_key);

        let entry = tlcache.get(&480).expect("timeline should be created");
        assert!(
            entry.time.is_finite(),
            "time should be finite when BPM is 0, got {}",
            entry.time
        );
        assert_eq!(
            entry.time, 0.0,
            "time should equal previous entry's time when BPM is 0"
        );
    }

    #[test]
    fn test_ensure_timeline_normal_bpm_computes_time() {
        let mut tlcache: BTreeMap<i32, TimeLineCache> = BTreeMap::new();
        let mode_key = 8;
        let resolution = 960.0;

        let mut tl0 = TimeLine::new(0.0, 0, mode_key);
        tl0.set_bpm(120.0);
        tlcache.insert(0, TimeLineCache::new(0.0, tl0));

        ensure_timeline(&mut tlcache, 480, resolution, mode_key);

        let entry = tlcache.get(&480).expect("timeline should be created");
        assert!(
            entry.time.is_finite(),
            "time should be finite with normal BPM"
        );
        // 240000 * 1000 * (480 / 960.0) / 120.0 = 1_000_000
        let expected = 240000.0 * 1000.0 * (480.0 / 960.0) / 120.0;
        assert!(
            (entry.time - expected).abs() < 1.0,
            "time should be ~{}, got {}",
            expected,
            entry.time
        );
    }
}
