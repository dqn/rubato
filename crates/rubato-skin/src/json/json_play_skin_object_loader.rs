// Mechanical translation of JsonPlaySkinObjectLoader.java
// Play skin object loader (handles note, judge, hidden cover, BGA, etc.)

use std::path::Path;

use crate::json::json_skin;
use crate::json::json_skin_loader::{JSONSkinLoader, SkinData, SkinObjectData, SkinObjectType};
use crate::json::json_skin_object_loader::{self, JsonSkinObjectLoader};
use crate::property::boolean_property_factory;
use crate::property::timer_property_factory;
use crate::reexports::Resolution;
use crate::types::skin_object::DestinationParams;

/// Corresponds to JsonPlaySkinObjectLoader extends JsonSkinObjectLoader<PlaySkin>
pub struct JsonPlaySkinObjectLoader;

fn source_resolution(skin: &SkinData) -> Resolution {
    skin.header
        .as_ref()
        .and_then(|header| header.source_resolution.clone())
        .unwrap_or(Resolution {
            width: 1280.0,
            height: 720.0,
        })
}

fn apply_runtime_skin_object_data(
    obj: &mut crate::skin::SkinObject,
    obj_data: &SkinObjectData,
    src: &Resolution,
    dst: &Resolution,
) {
    let dw = crate::safe_div_f32(dst.width, src.width);
    let dh = crate::safe_div_f32(dst.height, src.height);

    for dst_data in &obj_data.destinations {
        let timer_id = dst_data.timer.unwrap_or(0);
        let timer_prop = if timer_id > 0 {
            timer_property_factory::timer_property(timer_id)
        } else {
            None
        };
        let params = DestinationParams {
            time: dst_data.time as i64,
            x: dst_data.x as f32 * dw,
            y: dst_data.y as f32 * dh,
            w: dst_data.w as f32 * dw,
            h: dst_data.h as f32 * dh,
            acc: dst_data.acc,
            a: dst_data.a,
            r: dst_data.r,
            g: dst_data.g,
            b: dst_data.b,
            blend: dst_data.blend,
            filter: dst_data.filter,
            angle: dst_data.angle,
            center: dst_data.center,
            loop_val: dst_data.loop_val,
        };

        if let Some(draw_id) = dst_data.draw
            && draw_id != 0
            && let Some(draw_prop) = boolean_property_factory::boolean_property(draw_id)
        {
            obj.set_destination_with_timer_draw(&params, timer_prop, draw_prop);
            continue;
        }

        if !dst_data.op.is_empty() {
            obj.set_destination_with_timer_ops(&params, timer_prop, &dst_data.op);
        } else {
            obj.set_destination(&params, timer_prop, &[0, 0, 0], &[]);
        }
    }

    if let Some(mouse_rect) = &obj_data.mouse_rect {
        obj.set_mouse_rect(
            mouse_rect.x as f32 * dw,
            mouse_rect.y as f32 * dh,
            mouse_rect.w as f32 * dw,
            mouse_rect.h as f32 * dh,
        );
    }

    if !obj_data.offset_ids.is_empty() {
        obj.data_mut().set_offset_id(&obj_data.offset_ids);
    }
    if obj_data.stretch >= 0 {
        obj.data_mut().set_stretch_by_id(obj_data.stretch);
    }
}

fn prime_named_object_texture(
    loader: &mut JSONSkinLoader,
    sk: &json_skin::Skin,
    dst_id: &str,
    p: &Path,
) {
    for img in &sk.image {
        if img.id.as_deref() == Some(dst_id) {
            let _ = crate::json::json_skin_object_loader::utilities::texture(
                loader,
                img.src.as_deref(),
                p,
            );
            return;
        }
    }
    for value in &sk.value {
        if value.id.as_deref() == Some(dst_id) {
            let _ = crate::json::json_skin_object_loader::utilities::texture(
                loader,
                value.src.as_deref(),
                p,
            );
            return;
        }
    }
    for float_value in &sk.floatvalue {
        if float_value.id.as_deref() == Some(dst_id) {
            let _ = crate::json::json_skin_object_loader::utilities::texture(
                loader,
                float_value.src.as_deref(),
                p,
            );
            return;
        }
    }
}

fn resolve_judge_child_skin_object(
    loader: &mut JSONSkinLoader,
    skin: &SkinData,
    sk: &json_skin::Skin,
    dst: &json_skin::Destination,
    p: &Path,
) -> Option<crate::skin::SkinObject> {
    let dst_id = dst.id.as_deref()?;
    prime_named_object_texture(loader, sk, dst_id, p);
    let mut obj_data = json_skin_object_loader::load_base_skin_object(loader, skin, sk, dst, p)?;
    let mut dummy_skin = SkinData::new();
    loader.set_destination(&mut dummy_skin, &mut obj_data, dst);

    let src = source_resolution(skin);
    let scale_x = crate::safe_div_f32(loader.dstr.width, src.width);
    let scale_y = crate::safe_div_f32(loader.dstr.height, src.height);
    let mut obj = crate::skin_data_converter::convert_runtime_object(
        &obj_data.object_type,
        &mut loader.source_map,
        p,
        loader.usecim,
        scale_x,
        scale_y,
    )?;
    apply_runtime_skin_object_data(&mut obj, &obj_data, &src, &loader.dstr);
    Some(obj)
}

fn build_resolved_judge(
    loader: &mut JSONSkinLoader,
    skin: &SkinData,
    sk: &json_skin::Skin,
    judge: &json_skin::Judge,
    p: &Path,
) -> crate::skin_judge_object::SkinJudgeObject {
    let mut judge_obj = crate::skin_judge_object::SkinJudgeObject::new(judge.index, judge.shift);

    for (idx, image_dst) in judge.images.iter().enumerate().take(7) {
        match resolve_judge_child_skin_object(loader, skin, sk, image_dst, p) {
            Some(crate::skin::SkinObject::Image(image)) => {
                judge_obj.inner.set_judge(idx);
                judge_obj.set_judge_image(idx, image);
            }
            Some(other) => {
                log::warn!(
                    "judge image child {:?} resolved to unexpected object type {}",
                    image_dst.id,
                    other.type_name()
                );
            }
            None => {
                log::warn!("failed to resolve judge image child {:?}", image_dst.id);
            }
        }
    }

    for (idx, number_dst) in judge.numbers.iter().enumerate().take(7) {
        match resolve_judge_child_skin_object(loader, skin, sk, number_dst, p) {
            Some(crate::skin::SkinObject::Number(number)) => {
                judge_obj.inner.set_judge_count(idx);
                judge_obj.set_judge_count(idx, number);
            }
            Some(other) => {
                log::warn!(
                    "judge number child {:?} resolved to unexpected object type {}",
                    number_dst.id,
                    other.type_name()
                );
            }
            None => {
                log::warn!("failed to resolve judge number child {:?}", number_dst.id);
            }
        }
    }

    judge_obj
}

impl JsonSkinObjectLoader for JsonPlaySkinObjectLoader {
    fn skin(&self, header: &crate::json::json_skin_loader::SkinHeaderData) -> SkinData {
        // Corresponds to Java: new PlaySkin(header)
        let skin_type = crate::skin_type::SkinType::skin_type_by_id(header.skin_type)
            .unwrap_or(crate::skin_type::SkinType::Play7Keys);
        SkinData::from_header(header, skin_type)
    }

    fn load_skin_object(
        &self,
        loader: &mut JSONSkinLoader,
        skin: &SkinData,
        sk: &json_skin::Skin,
        dst: &json_skin::Destination,
        p: &Path,
    ) -> Option<SkinObjectData> {
        // Try base loader first
        let obj = json_skin_object_loader::load_base_skin_object(loader, skin, sk, dst, p);
        if obj.is_some() {
            return obj;
        }

        let dst_id = dst.id.as_deref()?;

        // note (playskin only)
        if let Some(ref note) = sk.note
            && dst_id == note.id.as_deref().unwrap_or("")
        {
            use crate::json::json_skin_object_loader::utilities::note_texture;
            use crate::skin_note_object::SkinNoteObject;

            // Determine lane count from note.dst (per-lane regions) or note.note (image IDs)
            let lane_count = if !note.dst.is_empty() {
                note.dst.len()
            } else {
                note.note.len()
            };

            let mut note_obj = SkinNoteObject::new(lane_count);

            // Set lane regions from note.dst
            for (i, anim) in note.dst.iter().enumerate() {
                note_obj.inner.set_lane_region(
                    i,
                    &rubato_play::skin::note::LaneRegion {
                        x: anim.x as f32,
                        y: anim.y as f32,
                        width: anim.w as f32,
                        height: anim.h as f32,
                        scale: 1.0,
                        dstnote2: i32::MIN,
                    },
                );
            }

            // Resolve note textures (first frame of each lane's animation)
            let note_textures = note_texture(loader, &note.note, p);
            for (i, tex) in note_textures.iter().enumerate() {
                if let Some(regions) = tex
                    && let Some(first) = regions.first()
                    && i < note_obj.note_images.len()
                {
                    // Use note image height as scale (Java: scale = noteImage.getRegionHeight() * dy)
                    if note.size.get(i).copied().unwrap_or(0.0) > 0.0 {
                        note_obj.inner.lanes_mut()[i].scale = note.size[i];
                    } else {
                        note_obj.inner.lanes_mut()[i].scale = first.region_height as f32;
                    }
                    note_obj.note_images[i] = Some(first.clone());
                }
            }

            // Resolve mine textures
            let mine_textures = note_texture(loader, &note.mine, p);
            for (i, tex) in mine_textures.iter().enumerate() {
                if let Some(regions) = tex
                    && let Some(first) = regions.first()
                    && i < note_obj.mine_images.len()
                {
                    note_obj.mine_images[i] = Some(first.clone());
                }
            }

            log::debug!(
                "Note: lane_count={}, note_images_wired={}, mine_images_wired={}",
                lane_count,
                note_obj.note_images.iter().filter(|i| i.is_some()).count(),
                note_obj.mine_images.iter().filter(|i| i.is_some()).count(),
            );

            let obj = SkinObjectData {
                name: note.id.clone(),
                object_type: SkinObjectType::Note,
                resolved_note: Some(note_obj),
                ..Default::default()
            };
            return Some(obj);
        }

        // hidden cover (playskin only)
        for img in &sk.hidden_cover {
            if dst_id == img.id.as_deref().unwrap_or("") {
                let obj = SkinObjectData {
                    name: img.id.clone(),
                    object_type: SkinObjectType::HiddenCover {
                        src: img.src.clone(),
                        x: img.x,
                        y: img.y,
                        w: img.w,
                        h: img.h,
                        divx: img.divx,
                        divy: img.divy,
                        timer: img.timer,
                        cycle: img.cycle,
                        disapear_line: img.disapear_line,
                        is_disapear_line_link_lift: img.is_disapear_line_link_lift,
                    },
                    ..Default::default()
                };
                return Some(obj);
            }
        }

        // lift cover (playskin only)
        for img in &sk.lift_cover {
            if dst_id == img.id.as_deref().unwrap_or("") {
                let obj = SkinObjectData {
                    name: img.id.clone(),
                    object_type: SkinObjectType::LiftCover {
                        src: img.src.clone(),
                        x: img.x,
                        y: img.y,
                        w: img.w,
                        h: img.h,
                        divx: img.divx,
                        divy: img.divy,
                        timer: img.timer,
                        cycle: img.cycle,
                        disapear_line: img.disapear_line,
                        is_disapear_line_link_lift: img.is_disapear_line_link_lift,
                    },
                    ..Default::default()
                };
                return Some(obj);
            }
        }

        // bga (playskin only)
        if let Some(ref bga) = sk.bga
            && dst_id == bga.id.as_deref().unwrap_or("")
        {
            let obj = SkinObjectData {
                name: bga.id.clone(),
                object_type: SkinObjectType::Bga {
                    bga_expand: loader.bga_expand,
                },
                ..Default::default()
            };
            return Some(obj);
        }

        // judge (playskin only)
        for judge in &sk.judge {
            if dst_id == judge.id.as_deref().unwrap_or("") {
                let obj = SkinObjectData {
                    name: judge.id.clone(),
                    object_type: SkinObjectType::Judge {
                        index: judge.index,
                        shift: judge.shift,
                    },
                    resolved_judge: Some(build_resolved_judge(loader, skin, sk, judge, p)),
                    ..Default::default()
                };
                return Some(obj);
            }
        }

        // POMYU chara
        for chara in &sk.pmchara {
            if dst_id == chara.id.as_deref().unwrap_or("") {
                let obj = SkinObjectData {
                    name: chara.id.clone(),
                    object_type: SkinObjectType::PmChara {
                        src: chara.src.clone(),
                        color: chara.color,
                        chara_type: chara.chara_type,
                        side: chara.side,
                    },
                    ..Default::default()
                };
                return Some(obj);
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::json_skin_loader::SkinHeaderData;
    use crate::json::json_skin_loader::{SkinConfigProperty, parse_skin_json};
    use crate::json::json_skin_object_loader::JsonSkinObjectLoader;
    use crate::loaders::skin_data_converter::convert_skin_data;
    use crate::reexports::Resolution;
    use crate::skin_type::SkinType;
    use crate::types::skin::SkinObject;
    use std::path::PathBuf;

    const MINIMAL_JUDGE_SKIN_JSON: &str = r#"{
        "type": 0,
        "name": "Judge wiring test",
        "w": 1920,
        "h": 1080,
        "source": [
            {"id": "judge_sheet", "path": "judge/default.png"}
        ],
        "image": [
            {"id": "judgef-pg", "src": "judge_sheet", "x": 0, "y": 0, "w": 64, "h": 32, "divx": 1, "divy": 1, "timer": 46, "cycle": 500}
        ],
        "value": [
            {"id": "judgen-pg", "src": "judge_sheet", "x": 0, "y": 0, "w": 110, "h": 10, "divx": 11, "divy": 1, "digit": 4, "align": 2, "timer": 46, "cycle": 500}
        ],
        "judge": [
            {
                "id": "judge",
                "index": 0,
                "shift": true,
                "images": [
                    {
                        "id": "judgef-pg",
                        "timer": 46,
                        "dst": [
                            {"time": 0, "x": 100, "y": 100, "w": 64, "h": 32},
                            {"time": 500}
                        ]
                    }
                ],
                "numbers": [
                    {
                        "id": "judgen-pg",
                        "timer": 46,
                        "dst": [
                            {"time": 0, "x": 180, "y": 100, "w": 10, "h": 10},
                            {"time": 500}
                        ]
                    }
                ]
            }
        ],
        "destination": [
            {"id": "judge"}
        ]
    }"#;

    fn make_header(skin_type_id: i32) -> SkinHeaderData {
        SkinHeaderData {
            skin_type: skin_type_id,
            name: "Test Play Skin".to_string(),
            ..Default::default()
        }
    }

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
    }

    #[test]
    fn test_get_skin_returns_play7keys_for_7key_header() {
        let loader = JsonPlaySkinObjectLoader;
        let header = make_header(SkinType::Play7Keys.id());
        let skin = loader.skin(&header);
        assert_eq!(skin.skin_type, Some(SkinType::Play7Keys));
        assert!(skin.header.is_some());
        assert_eq!(skin.header.unwrap().name, "Test Play Skin");
    }

    #[test]
    fn test_get_skin_returns_play5keys_for_5key_header() {
        let loader = JsonPlaySkinObjectLoader;
        let header = make_header(SkinType::Play5Keys.id());
        let skin = loader.skin(&header);
        assert_eq!(skin.skin_type, Some(SkinType::Play5Keys));
    }

    #[test]
    fn test_get_skin_returns_play14keys_for_14key_header() {
        let loader = JsonPlaySkinObjectLoader;
        let header = make_header(SkinType::Play14Keys.id());
        let skin = loader.skin(&header);
        assert_eq!(skin.skin_type, Some(SkinType::Play14Keys));
    }

    #[test]
    fn test_get_skin_returns_play24keys_for_24key_header() {
        let loader = JsonPlaySkinObjectLoader;
        let header = make_header(SkinType::Play24Keys.id());
        let skin = loader.skin(&header);
        assert_eq!(skin.skin_type, Some(SkinType::Play24Keys));
    }

    #[test]
    fn test_get_skin_fallback_to_play7keys_for_unknown_id() {
        let loader = JsonPlaySkinObjectLoader;
        let header = make_header(-999);
        let skin = loader.skin(&header);
        assert_eq!(skin.skin_type, Some(SkinType::Play7Keys));
    }

    #[test]
    fn test_get_skin_default_fields_are_zero() {
        let loader = JsonPlaySkinObjectLoader;
        let header = make_header(SkinType::Play7Keys.id());
        let skin = loader.skin(&header);
        assert_eq!(skin.fadeout, 0);
        assert_eq!(skin.input, 0);
        assert_eq!(skin.scene, 0);
        assert!(skin.objects.is_empty());
    }

    #[test]
    fn test_json_play_skin_judge_wires_nested_images_and_numbers() {
        let mut loader = JSONSkinLoader::new();
        let skin_path = repo_root().join("skin/ECFN/play/test-judge.json");
        let header = SkinHeaderData {
            skin_type: SkinType::Play7Keys.id(),
            name: "Judge wiring".to_string(),
            path: skin_path.clone(),
            source_resolution: Some(Resolution {
                width: 1920.0,
                height: 1080.0,
            }),
            destination_resolution: Some(Resolution {
                width: 1920.0,
                height: 1080.0,
            }),
            ..Default::default()
        };
        let sk = parse_skin_json(MINIMAL_JUDGE_SKIN_JSON).expect("judge JSON should parse");
        let skin_data = loader
            .load_json_skin(
                &header,
                &sk,
                &SkinType::Play7Keys,
                &SkinConfigProperty,
                &skin_path,
            )
            .expect("play skin should load");
        let runtime_skin = convert_skin_data(
            &header,
            skin_data,
            &mut loader.source_map,
            &skin_path,
            false,
            &Resolution {
                width: 1920.0,
                height: 1080.0,
            },
        )
        .expect("runtime skin should convert");

        let judge = runtime_skin
            .objects()
            .iter()
            .find_map(|obj| match obj {
                SkinObject::Judge(judge) => Some(judge),
                _ => None,
            })
            .expect("converted skin should contain a judge object");
        assert!(
            judge.judge_images()[0].is_some(),
            "nested judge image should be wired into SkinJudgeObject"
        );
    }
}
