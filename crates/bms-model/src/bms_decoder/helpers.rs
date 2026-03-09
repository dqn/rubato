use std::borrow::Cow;

use crate::bms_model::{BMSModel, JudgeRankType, TotalType};
use crate::chart_decoder;
use crate::decode_log::{DecodeLog, State};

pub(super) fn normalize_path_separators(s: &str) -> Cow<'_, str> {
    if s.contains('\\') {
        Cow::Owned(s.replace('\\', "/"))
    } else {
        Cow::Borrowed(s)
    }
}

pub(super) fn matches_reserve_word(line: &str, s: &str) -> bool {
    let len = s.len();
    if line.len() <= len {
        return false;
    }
    let line_bytes = line.as_bytes();
    let s_bytes = s.as_bytes();
    for i in 0..len {
        let c = line_bytes[i + 1];
        let c2 = s_bytes[i];
        if c != c2 && c != c2 + 32 {
            return false;
        }
    }
    true
}

pub fn convert_hex_string(data: &[u8]) -> String {
    let mut sb = String::with_capacity(data.len() * 2);
    for &b in data {
        sb.push(char::from_digit(((b >> 4) & 0xf) as u32, 16).expect("valid hex digit"));
        sb.push(char::from_digit((b & 0xf) as u32, 16).expect("valid hex digit"));
    }
    sb
}

/// Thread-local LCG for BMS `#RANDOM` directives.
/// Uses a simple linear congruential generator seeded once per thread from system time,
/// so successive calls within the same decode pass produce independent values.
pub(super) fn rand_f64() -> f64 {
    use std::cell::Cell;
    use std::time::SystemTime;

    thread_local! {
        static STATE: Cell<u64> = Cell::new({
            let nanos = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .subsec_nanos();
            nanos as u64 ^ 0x5DEECE66D
        });
    }

    STATE.with(|s| {
        // Java-style LCG: next = (state * 0x5DEECE66D + 0xB) & 0xFFFF_FFFF_FFFF
        let state = s.get();
        let next = state.wrapping_mul(0x5DEECE66D).wrapping_add(0xB) & 0xFFFF_FFFF_FFFF;
        s.set(next);
        (next as f64) / (0x1_0000_0000_0000u64 as f64)
    })
}

pub(super) fn process_command_word(
    line: &str,
    model: &mut BMSModel,
    log: &mut Vec<DecodeLog>,
) -> bool {
    struct CmdDef {
        name: &'static str,
        handler: fn(&mut BMSModel, &str) -> Option<DecodeLog>,
    }

    let commands: &[CmdDef] = &[
        CmdDef {
            name: "PLAYER",
            handler: |model, arg| {
                match arg.parse::<i32>() {
                    Ok(player) => {
                        if (1..3).contains(&player) {
                            model.player = player;
                        } else {
                            return Some(DecodeLog::new(
                                State::Warning,
                                format!("#PLAYERに規定外の数字が定義されています : {}", player),
                            ));
                        }
                    }
                    Err(_) => {
                        return Some(DecodeLog::new(
                            State::Warning,
                            "#PLAYERに数字が定義されていません",
                        ));
                    }
                }
                None
            },
        },
        CmdDef {
            name: "GENRE",
            handler: |model, arg| {
                model.genre = arg.to_string();
                None
            },
        },
        CmdDef {
            name: "TITLE",
            handler: |model, arg| {
                model.title = arg.to_string();
                None
            },
        },
        CmdDef {
            name: "SUBTITLE",
            handler: |model, arg| {
                model.sub_title = arg.to_string();
                None
            },
        },
        CmdDef {
            name: "ARTIST",
            handler: |model, arg| {
                model.artist = arg.to_string();
                None
            },
        },
        CmdDef {
            name: "SUBARTIST",
            handler: |model, arg| {
                model.subartist = arg.to_string();
                None
            },
        },
        CmdDef {
            name: "PLAYLEVEL",
            handler: |model, arg| {
                model.playlevel = arg.to_string();
                None
            },
        },
        CmdDef {
            name: "RANK",
            handler: |model, arg| {
                match arg.parse::<i32>() {
                    Ok(rank) => {
                        if (0..5).contains(&rank) {
                            model.judgerank = rank;
                            model.judgerank_type = JudgeRankType::BmsRank;
                        } else {
                            return Some(DecodeLog::new(
                                State::Warning,
                                format!("#RANKに規定外の数字が定義されています : {}", rank),
                            ));
                        }
                    }
                    Err(_) => {
                        return Some(DecodeLog::new(
                            State::Warning,
                            "#RANKに数字が定義されていません",
                        ));
                    }
                }
                None
            },
        },
        CmdDef {
            name: "DEFEXRANK",
            handler: |model, arg| {
                match arg.parse::<i32>() {
                    Ok(rank) => {
                        if rank >= 1 {
                            model.judgerank = rank;
                            model.judgerank_type = JudgeRankType::BmsDefexrank;
                        } else {
                            return Some(DecodeLog::new(
                                State::Warning,
                                format!("#DEFEXRANK 1以下はサポートしていません{}", rank),
                            ));
                        }
                    }
                    Err(_) => {
                        return Some(DecodeLog::new(
                            State::Warning,
                            "#DEFEXRANKに数字が定義されていません",
                        ));
                    }
                }
                None
            },
        },
        CmdDef {
            name: "TOTAL",
            handler: |model, arg| {
                match arg.parse::<f64>() {
                    Ok(total) => {
                        if total > 0.0 {
                            model.total = total;
                            model.total_type = TotalType::Bms;
                        } else {
                            return Some(DecodeLog::new(State::Warning, "#TOTALが0以下です"));
                        }
                    }
                    Err(_) => {
                        return Some(DecodeLog::new(
                            State::Warning,
                            "#TOTALに数字が定義されていません",
                        ));
                    }
                }
                None
            },
        },
        CmdDef {
            name: "VOLWAV",
            handler: |model, arg| {
                match arg.parse::<i32>() {
                    Ok(v) => {
                        model.volwav = v;
                    }
                    Err(_) => {
                        return Some(DecodeLog::new(
                            State::Warning,
                            "#VOLWAVに数字が定義されていません",
                        ));
                    }
                }
                None
            },
        },
        CmdDef {
            name: "STAGEFILE",
            handler: |model, arg| {
                model.stagefile = normalize_path_separators(arg).into_owned();
                None
            },
        },
        CmdDef {
            name: "BACKBMP",
            handler: |model, arg| {
                model.backbmp = normalize_path_separators(arg).into_owned();
                None
            },
        },
        CmdDef {
            name: "PREVIEW",
            handler: |model, arg| {
                model.preview = normalize_path_separators(arg).into_owned();
                None
            },
        },
        CmdDef {
            name: "LNOBJ",
            handler: |model, arg| {
                if model.base() == 62 {
                    match chart_decoder::parse_int62_str(arg, 0) {
                        Ok(v) => model.lnobj = v,
                        Err(_) => {
                            return Some(DecodeLog::new(
                                State::Warning,
                                "#LNOBJに数字が定義されていません",
                            ));
                        }
                    }
                } else {
                    match i32::from_str_radix(&arg.to_uppercase(), 36) {
                        Ok(v) => model.lnobj = v,
                        Err(_) => {
                            return Some(DecodeLog::new(
                                State::Warning,
                                "#LNOBJに数字が定義されていません",
                            ));
                        }
                    }
                }
                None
            },
        },
        CmdDef {
            name: "LNMODE",
            handler: |model, arg| {
                match arg.parse::<i32>() {
                    Ok(mut lnmode) => {
                        if !(0..=3).contains(&lnmode) {
                            return Some(DecodeLog::new(
                                State::Warning,
                                "#LNMODEに無効な数字が定義されています",
                            ));
                        }
                        // LR2oraja Endless Dream: LR2 does not support LNMODE, suppress modes 1 or 2
                        lnmode = 0;
                        model.lnmode = lnmode;
                    }
                    Err(_) => {
                        return Some(DecodeLog::new(
                            State::Warning,
                            "#LNMODEに数字が定義されていません",
                        ));
                    }
                }
                None
            },
        },
        CmdDef {
            name: "DIFFICULTY",
            handler: |model, arg| {
                match arg.parse::<i32>() {
                    Ok(v) => model.difficulty = v,
                    Err(_) => {
                        return Some(DecodeLog::new(
                            State::Warning,
                            "#DIFFICULTYに数字が定義されていません",
                        ));
                    }
                }
                None
            },
        },
        CmdDef {
            name: "BANNER",
            handler: |model, arg| {
                model.banner = normalize_path_separators(arg).into_owned();
                None
            },
        },
        CmdDef {
            name: "COMMENT",
            handler: |_model, _arg| {
                // #COMMENT: metadata-only, no behavioral effect
                None
            },
        },
        CmdDef {
            name: "BASE",
            handler: |model, arg| {
                match arg.parse::<i32>() {
                    Ok(base) => {
                        if base != 62 {
                            return Some(DecodeLog::new(
                                State::Warning,
                                "#BASEに無効な数字が定義されています",
                            ));
                        }
                        model.set_base(base);
                    }
                    Err(_) => {
                        return Some(DecodeLog::new(
                            State::Warning,
                            "#BASEに数字が定義されていません",
                        ));
                    }
                }
                None
            },
        },
    ];

    for cmd in commands {
        if line.len() > cmd.name.len() + 2 && matches_reserve_word(line, cmd.name) {
            let Some(arg) = line.get(cmd.name.len() + 2..) else {
                continue;
            };
            let arg = arg.trim();
            let result = (cmd.handler)(model, arg);
            if let Some(dl) = result {
                log.push(dl);
            }
            return true;
        }
    }
    false
}
