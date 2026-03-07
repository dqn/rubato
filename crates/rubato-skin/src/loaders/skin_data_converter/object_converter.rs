fn set_click_event_from_type(obj: &mut SkinObject, obj_type: &SkinObjectType) {
    match obj_type {
        SkinObjectType::Image {
            act: Some(act_id),
            click,
            ..
        } => {
            obj.data_mut().set_clickevent_by_id(*act_id);
            obj.data_mut().clickevent_type = *click;
        }
        SkinObjectType::ImageSet {
            act: Some(act_id),
            click,
            ..
        } => {
            obj.data_mut().set_clickevent_by_id(*act_id);
            obj.data_mut().clickevent_type = *click;
        }
        _ => {}
    }
}

/// Converts a SkinObjectType into a SkinObject.
fn convert_skin_object(
    obj_type: &SkinObjectType,
    source_map: &mut HashMap<String, SourceData>,
    skin_path: &Path,
    usecim: bool,
    scale_y: f32,
) -> Option<SkinObject> {
    match obj_type {
        SkinObjectType::Unknown => None,

        SkinObjectType::ImageById(id) => Some(SkinObject::Image(SkinImage::new_with_image_id(*id))),

        SkinObjectType::Image {
            src,
            x,
            y,
            w,
            h,
            divx,
            divy,
            timer,
            cycle,
            len,
            ref_id,
            act: _,
            click: _,
            is_movie,
        } => {
            if *is_movie {
                // Movie sources: create SkinImage with SkinSourceMovie
                let movie_source = crate::skin_source_movie::SkinSourceMovie::new("");
                return Some(SkinObject::Image(SkinImage::new_with_movie(movie_source)));
            }

            let tex = get_texture_for_src(src.as_deref(), source_map, skin_path, usecim)?;
            let srcimg = source_image(&tex, *x, *y, *w, *h, *divx, *divy);

            if *len > 1 {
                // Multiple reference images
                let imgs_per_ref = srcimg.len() / (*len as usize);
                let mut tr: Vec<Vec<TextureRegion>> = Vec::with_capacity(*len as usize);
                for i in 0..(*len as usize) {
                    let mut row: Vec<TextureRegion> = Vec::with_capacity(imgs_per_ref);
                    for j in 0..imgs_per_ref {
                        row.push(srcimg[i * imgs_per_ref + j].clone());
                    }
                    tr.push(row);
                }
                let timer_val = timer.unwrap_or(0);
                Some(SkinObject::Image(SkinImage::new_with_int_timer_ref_id(
                    tr, timer_val, *cycle, *ref_id,
                )))
            } else {
                let timer_val = timer.unwrap_or(0);
                Some(SkinObject::Image(SkinImage::new_with_int_timer(
                    srcimg, timer_val, *cycle,
                )))
            }
        }

        SkinObjectType::ImageSet {
            images,
            ref_id,
            value,
            act: _,
            click: _,
        } => {
            // ImageSet: each image ID in `images` references an entry in sk.image[].
            // The converter doesn't have access to sk, so we create a SkinImage
            // bound to the value/ref property. The actual image sources will be empty
            // (rendering deferred until sk is threaded through the converter).
            if images.is_empty() {
                warn!("ImageSet has no image entries");
                return None;
            }
            let binding_id = value.unwrap_or(*ref_id);
            debug!(
                "ImageSet: creating placeholder with {} image refs, binding={}",
                images.len(),
                binding_id
            );
            Some(SkinObject::Image(SkinImage::new_with_image_id(binding_id)))
        }

        SkinObjectType::ResolvedImageSet { images, ref_id } => {
            resolve_image_set(images, *ref_id, source_map, skin_path, usecim)
        }

        SkinObjectType::Number {
            src,
            x,
            y,
            w,
            h,
            divx,
            divy,
            timer,
            cycle,
            digit,
            padding,
            zeropadding,
            space,
            ref_id,
            value,
            align,
            offsets,
        } => {
            let tex = get_texture_for_src(src.as_deref(), source_map, skin_path, usecim)?;
            let images = source_image(&tex, *x, *y, *w, *h, *divx, *divy);
            let timer_val = timer.unwrap_or(0);

            let num = if images.len().is_multiple_of(24) {
                // +-12 digit images
                let set_count = images.len() / 24;
                let mut pn: Vec<Vec<TextureRegion>> = Vec::with_capacity(set_count);
                let mut mn: Vec<Vec<TextureRegion>> = Vec::with_capacity(set_count);
                for j in 0..set_count {
                    let mut p_row = Vec::with_capacity(12);
                    let mut m_row = Vec::with_capacity(12);
                    for i in 0..12 {
                        p_row.push(images[j * 24 + i].clone());
                        m_row.push(images[j * 24 + i + 12].clone());
                    }
                    pn.push(p_row);
                    mn.push(m_row);
                }
                if let Some(val) = value {
                    SkinNumber::new_with_int_timer(
                        pn,
                        Some(mn),
                        timer_val,
                        *cycle,
                        *digit,
                        *zeropadding,
                        *space,
                        *val,
                        *align,
                    )
                } else {
                    SkinNumber::new_with_int_timer(
                        pn,
                        Some(mn),
                        timer_val,
                        *cycle,
                        *digit,
                        *zeropadding,
                        *space,
                        *ref_id,
                        *align,
                    )
                }
            } else {
                // 10 or 11 digit images
                let d = if images.len().is_multiple_of(10) {
                    10
                } else {
                    11
                };
                let set_count = images.len() / d;
                let mut nimages: Vec<Vec<TextureRegion>> = Vec::with_capacity(set_count);
                for j in 0..set_count {
                    let mut row = Vec::with_capacity(d);
                    for i in 0..d {
                        row.push(images[j * d + i].clone());
                    }
                    nimages.push(row);
                }
                let actual_padding = if d > 10 { 2 } else { *padding };
                if let Some(val) = value {
                    SkinNumber::new_with_int_timer(
                        nimages,
                        None,
                        timer_val,
                        *cycle,
                        *digit,
                        actual_padding,
                        *space,
                        *val,
                        *align,
                    )
                } else {
                    SkinNumber::new_with_int_timer(
                        nimages,
                        None,
                        timer_val,
                        *cycle,
                        *digit,
                        actual_padding,
                        *space,
                        *ref_id,
                        *align,
                    )
                }
            };

            // Apply per-digit offsets if present
            let mut num = num;
            if let Some(ofs) = offsets {
                let skin_offsets: Vec<SkinOffset> = ofs
                    .iter()
                    .map(|o| SkinOffset {
                        x: o.x as f32,
                        y: o.y as f32,
                        w: o.w as f32,
                        h: o.h as f32,
                        r: 0.0,
                        a: 0.0,
                    })
                    .collect();
                num.set_offsets(skin_offsets);
            }

            Some(SkinObject::Number(num))
        }

        SkinObjectType::Float {
            src,
            x,
            y,
            w,
            h,
            divx,
            divy,
            timer,
            cycle,
            iketa,
            fketa,
            is_signvisible,
            align,
            zeropadding,
            space,
            ref_id,
            value,
            gain,
            offsets: _,
        } => {
            // SkinFloat construction requires complex image splitting.
            // For now, create a stub that won't crash but won't render either.
            warn!("Float conversion creates placeholder (full SkinFloat image splitting deferred)");
            let tex = get_texture_for_src(src.as_deref(), source_map, skin_path, usecim);
            tex.as_ref()?;
            let tex = tex.expect("tex");
            let images = source_image(&tex, *x, *y, *w, *h, *divx, *divy);
            let timer_val = timer.unwrap_or(0);

            // Create as SkinFloat using the available constructor
            let image_opts: Vec<Vec<Option<TextureRegion>>> = if images.len().is_multiple_of(12) {
                let set_count = images.len() / 12;
                let mut result = Vec::with_capacity(set_count);
                for j in 0..set_count {
                    let mut row = Vec::with_capacity(12);
                    for i in 0..12 {
                        row.push(Some(images[j * 12 + i].clone()));
                    }
                    result.push(row);
                }
                result
            } else {
                vec![images.into_iter().map(Some).collect()]
            };

            // Use `value` if present (explicit ID), otherwise fall back to `ref_id`
            let prop_id = value.unwrap_or(*ref_id);
            let sf = crate::skin_float::SkinFloat::new_with_int_timer_int_id(
                image_opts,
                timer_val,
                *cycle,
                *iketa,
                *fketa,
                *is_signvisible,
                *align,
                *zeropadding,
                *space,
                prop_id,
                *gain,
            );
            Some(SkinObject::Float(sf))
        }

        SkinObjectType::Text {
            font,
            size,
            align: _,
            ref_id,
            value,
            constant_text: _,
            wrapping: _,
            overflow: _,
            outline_color: _,
            outline_width: _,
            shadow_color: _,
            shadow_offset_x: _,
            shadow_offset_y: _,
            shadow_smoothness: _,
        } => {
            if let Some(font_path) = font {
                let text_id = value.unwrap_or(*ref_id);
                let property = if text_id >= 0 {
                    string_property_factory::string_property_by_id(text_id)
                } else {
                    None
                };
                let stf = SkinTextFont::new_with_property(font_path, 0, *size, 0, property);
                Some(SkinObject::TextFont(stf))
            } else {
                warn!("Text object without font path, skipping");
                None
            }
        }

        SkinObjectType::Slider {
            src,
            x,
            y,
            w,
            h,
            divx,
            divy,
            timer,
            cycle,
            angle,
            range,
            slider_type,
            changeable,
            value,
            event: _,
            is_ref_num: _,
            min: _,
            max: _,
        } => {
            let tex = get_texture_for_src(src.as_deref(), source_map, skin_path, usecim)?;
            let images = source_image(&tex, *x, *y, *w, *h, *divx, *divy);
            let timer_val = timer.unwrap_or(0);
            let type_id = value.unwrap_or(*slider_type);
            let slider = SkinSlider::new_with_int_timer(
                images,
                timer_val,
                *cycle,
                *angle,
                *range,
                type_id,
                *changeable,
            );
            Some(SkinObject::Slider(slider))
        }

        SkinObjectType::Graph {
            src,
            x,
            y,
            w,
            h,
            divx,
            divy,
            timer,
            cycle,
            angle,
            graph_type,
            value,
            is_ref_num,
            min,
            max,
        } => {
            let tex = get_texture_for_src(src.as_deref(), source_map, skin_path, usecim)?;
            let images = source_image(&tex, *x, *y, *w, *h, *divx, *divy);
            let timer_val = timer.unwrap_or(0);
            if let Some(val) = value {
                Some(SkinObject::Graph(SkinGraph::new_with_int_timer(
                    images, timer_val, *cycle, *val, *angle,
                )))
            } else if *is_ref_num {
                Some(SkinObject::Graph(SkinGraph::new_with_int_timer_minmax(
                    images,
                    timer_val,
                    *cycle,
                    *graph_type,
                    *min,
                    *max,
                    *angle,
                )))
            } else {
                Some(SkinObject::Graph(SkinGraph::new_with_int_timer(
                    images,
                    timer_val,
                    *cycle,
                    *graph_type,
                    *angle,
                )))
            }
        }

        SkinObjectType::DistributionGraph { graph_type, .. } => {
            // SkinNoteDistributionGraph with TYPE_NORMAL
            let graph = SkinNoteDistributionGraph::new(*graph_type, 0, 0, 0, 0, 0);
            Some(SkinObject::NoteDistributionGraph(graph))
        }

        SkinObjectType::GaugeGraph {
            color,
            assist_clear_bg_color,
            assist_and_easy_fail_bg_color,
            groove_fail_bg_color,
            groove_clear_and_hard_bg_color,
            ex_hard_bg_color,
            hazard_bg_color,
            assist_clear_line_color,
            assist_and_easy_fail_line_color,
            groove_fail_line_color,
            groove_clear_and_hard_line_color,
            ex_hard_line_color,
            hazard_line_color,
            borderline_color,
            border_color,
        } => {
            let gg = if let Some(colors) = color {
                SkinGaugeGraphObject::new_from_colors(colors)
            } else {
                SkinGaugeGraphObject::new_from_color_strings(
                    assist_clear_bg_color,
                    assist_and_easy_fail_bg_color,
                    groove_fail_bg_color,
                    groove_clear_and_hard_bg_color,
                    ex_hard_bg_color,
                    hazard_bg_color,
                    assist_clear_line_color,
                    assist_and_easy_fail_line_color,
                    groove_fail_line_color,
                    groove_clear_and_hard_line_color,
                    ex_hard_line_color,
                    hazard_line_color,
                    borderline_color,
                    border_color,
                )
            };
            Some(SkinObject::GaugeGraph(gg))
        }

        SkinObjectType::JudgeGraph {
            graph_type,
            delay,
            back_tex_off,
            order_reverse,
            no_gap,
            no_gap_x,
        } => {
            let graph = SkinNoteDistributionGraph::new(
                *graph_type,
                *delay,
                *back_tex_off,
                *order_reverse,
                *no_gap,
                *no_gap_x,
            );
            Some(SkinObject::NoteDistributionGraph(graph))
        }

        SkinObjectType::BpmGraph {
            delay,
            line_width,
            main_bpm_color,
            min_bpm_color,
            max_bpm_color,
            other_bpm_color,
            stop_line_color,
            transition_line_color,
        } => {
            let graph = SkinBPMGraph::new(
                *delay,
                *line_width,
                main_bpm_color,
                min_bpm_color,
                max_bpm_color,
                other_bpm_color,
                stop_line_color,
                transition_line_color,
            );
            Some(SkinObject::BpmGraph(graph))
        }

        SkinObjectType::HitErrorVisualizer {
            width,
            judge_width_millis,
            line_width,
            color_mode,
            hiterror_mode,
            ema_mode,
            line_color,
            center_color,
            pg_color,
            gr_color,
            gd_color,
            bd_color,
            pr_color,
            ema_color,
            alpha,
            window_length,
            transparent,
            draw_decay,
        } => {
            let viz = SkinHitErrorVisualizer::new(
                *width,
                *judge_width_millis,
                *line_width,
                *color_mode,
                *hiterror_mode,
                *ema_mode,
                line_color,
                center_color,
                pg_color,
                gr_color,
                gd_color,
                bd_color,
                pr_color,
                ema_color,
                *alpha,
                *window_length,
                *transparent,
                *draw_decay,
            );
            Some(SkinObject::HitErrorVisualizer(viz))
        }

        SkinObjectType::TimingVisualizer {
            width,
            judge_width_millis,
            line_width,
            line_color,
            center_color,
            pg_color,
            gr_color,
            gd_color,
            bd_color,
            pr_color,
            transparent,
            draw_decay,
        } => {
            let viz = SkinTimingVisualizer::new(
                *width,
                *judge_width_millis,
                *line_width,
                line_color,
                center_color,
                pg_color,
                gr_color,
                gd_color,
                bd_color,
                pr_color,
                *transparent,
                *draw_decay,
            );
            Some(SkinObject::TimingVisualizer(viz))
        }

        SkinObjectType::TimingDistributionGraph {
            width,
            line_width,
            graph_color,
            average_color,
            dev_color,
            pg_color,
            gr_color,
            gd_color,
            bd_color,
            pr_color,
            draw_average,
            draw_dev,
        } => {
            let graph = SkinTimingDistributionGraph::new(
                *width,
                *line_width,
                graph_color,
                average_color,
                dev_color,
                pg_color,
                gr_color,
                gd_color,
                bd_color,
                pr_color,
                *draw_average,
                *draw_dev,
            );
            Some(SkinObject::TimingDistributionGraph(graph))
        }

        SkinObjectType::Gauge {
            nodes,
            parts,
            gauge_type,
            range,
            cycle,
            starttime,
            endtime,
        } => {
            // Gauge conversion: creates a SkinGauge with gauge image tiles.
            // Node IDs reference sk.image[] entries, which aren't available here.
            // We create the gauge structure with empty images; the node textures
            // require threading sk through the converter (deferred).
            //
            // Java indexmap logic maps 4/8/12 node configs to 36 gauge slots.
            // With 36 nodes, each maps 1:1 to a slot.
            let gauge_images: Vec<Vec<Option<TextureRegion>>> = Vec::new();
            debug!(
                "Gauge: creating with {} nodes, parts={}, type={} (images deferred)",
                nodes.len(),
                parts,
                gauge_type
            );
            let mut gauge = SkinGauge::new(
                gauge_images,
                0,
                *cycle,
                *parts,
                *gauge_type,
                *range,
                *cycle as i64,
            );
            gauge.starttime = *starttime;
            gauge.endtime = *endtime;
            Some(SkinObject::Gauge(gauge))
        }
        SkinObjectType::Note => {
            // Default lane count; lanes are configured later via set_lane_region
            let note = SkinNoteObject::new(0);
            Some(SkinObject::Note(note))
        }
        SkinObjectType::HiddenCover {
            src,
            x,
            y,
            w,
            h,
            divx,
            divy,
            timer,
            cycle,
            disapear_line,
            is_disapear_line_link_lift,
        } => {
            // HiddenCover: create SkinHidden with texture and disappear line.
            // Java: new SkinHidden(getSourceImage(tex,...), timer, cycle)
            //       setDisapearLine(disapearLine * scaleY)
            //       offsets += [OFFSET_LIFT, OFFSET_HIDDEN_COVER]
            let tex = get_texture_for_src(src.as_deref(), source_map, skin_path, usecim);
            if let Some(tex) = tex {
                let srcimg = source_image(&tex, *x, *y, *w, *h, *divx, *divy);
                let timer_val = timer.unwrap_or(0);
                let mut hidden = SkinHidden::new_with_int_timer(srcimg, timer_val, *cycle);
                hidden.set_disapear_line(*disapear_line as f32 * scale_y);
                hidden.is_disapear_line_link_lift = *is_disapear_line_link_lift;
                Some(SkinObject::Hidden(hidden))
            } else {
                warn!("HiddenCover: texture source {:?} not found", src);
                None
            }
        }
        SkinObjectType::LiftCover {
            src,
            x,
            y,
            w,
            h,
            divx,
            divy,
            timer,
            cycle,
            disapear_line,
            is_disapear_line_link_lift,
        } => {
            // LiftCover: same as HiddenCover but offset list only adds OFFSET_LIFT.
            let tex = get_texture_for_src(src.as_deref(), source_map, skin_path, usecim);
            if let Some(tex) = tex {
                let srcimg = source_image(&tex, *x, *y, *w, *h, *divx, *divy);
                let timer_val = timer.unwrap_or(0);
                let mut hidden = SkinHidden::new_with_int_timer(srcimg, timer_val, *cycle);
                hidden.set_disapear_line(*disapear_line as f32 * scale_y);
                hidden.is_disapear_line_link_lift = *is_disapear_line_link_lift;
                Some(SkinObject::Hidden(hidden))
            } else {
                warn!("LiftCover: texture source {:?} not found", src);
                None
            }
        }
        SkinObjectType::Bga { bga_expand } => {
            let bga = SkinBgaObject::new(*bga_expand);
            Some(SkinObject::Bga(bga))
        }
        SkinObjectType::Judge { index, shift } => {
            let judge = SkinJudgeObject::new(*index, *shift);
            Some(SkinObject::Judge(judge))
        }
        SkinObjectType::PmChara {
            src,
            color,
            chara_type,
            side: _,
        } => {
            // PmChara: Pomyu character rendering.
            // In Java, this uses PomyuCharaLoader to load character sprite sheets.
            // The loader needs file system access via getSrcIdPath and dst coordinates.
            // We create a placeholder SkinImage since PomyuCharaLoader produces SkinImage.
            debug!(
                "PmChara: type={}, color={}, src={:?} (image loading deferred)",
                chara_type, color, src
            );
            Some(SkinObject::Image(SkinImage::new_with_image_id(0)))
        }
        SkinObjectType::SongList { center, .. } => {
            let bar = SkinBarObject::new(*center);
            Some(SkinObject::Bar(bar))
        }
        SkinObjectType::SearchTextRegion { x, y, w, h } => {
            // SearchTextRegion: In Java, this sets a Rectangle on MusicSelectSkin.
            // It's not a SkinObject itself but a property of the select skin.
            // Since we don't have MusicSelectSkin in the converter, we log and skip.
            debug!(
                "SearchTextRegion: ({}, {}, {}, {}) -- stored as skin property, not a SkinObject",
                x, y, w, h
            );
            None
        }
    }
}

/// Loads a texture from the source map, resolving the source ID path.
fn get_texture_for_src(
    src_id: Option<&str>,
    source_map: &mut HashMap<String, SourceData>,
    skin_path: &Path,
    _usecim: bool,
) -> Option<crate::stubs::Texture> {
    let src_id = src_id?;

    // Check if already loaded
    if let Some(data) = source_map.get(src_id) {
        if data.loaded {
            return match &data.data {
                Some(SourceDataType::Texture(tex)) => Some(tex.clone()),
                _ => None,
            };
        }
    } else {
        return None;
    }

    // Load the texture
    let data_path = source_map.get(src_id)?.path.clone();
    let parent = skin_path
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    let image_path = format!("{}/{}", parent, data_path);

    let result = if std::path::Path::new(&image_path).exists() {
        Some(SourceDataType::Texture(crate::stubs::Texture::new(
            &image_path,
        )))
    } else {
        None
    };

    let tex_result = match &result {
        Some(SourceDataType::Texture(tex)) => Some(tex.clone()),
        _ => None,
    };

    // Cache the result
    if let Some(data) = source_map.get_mut(src_id) {
        data.data = result;
        data.loaded = true;
    }

    tex_result
}

/// Resolve an ImageSet into a multi-source SkinImage with actual textures.
/// Each entry in the set is looked up and its texture resolved from source_map.
fn resolve_image_set(
    entries: &[ResolvedImageEntry],
    ref_id: i32,
    source_map: &mut HashMap<String, SourceData>,
    skin_path: &Path,
    usecim: bool,
) -> Option<SkinObject> {
    if entries.is_empty() {
        return None;
    }
    let images: Vec<Vec<TextureRegion>> = entries
        .iter()
        .filter_map(|entry| {
            let tex = get_texture_for_src(entry.src.as_deref(), source_map, skin_path, usecim)?;
            Some(source_image(
                &tex, entry.x, entry.y, entry.w, entry.h, entry.divx, entry.divy,
            ))
        })
        .collect();
    if images.is_empty() {
        return None;
    }
    Some(SkinObject::Image(SkinImage::new_with_int_timer_ref_id(
        images, 0, 0, ref_id,
    )))
}

/// Build SelectBarData from resolved JSON SongList bar sub-objects.
/// Each sub-SkinObjectData is converted to the appropriate skin type
/// (SkinImage, SkinNumber, SkinTextFont) and stored in SelectBarData.
fn build_select_bar_data(
    bar_data: &SongListBarData,
    center: i32,
    clickable: &[i32],
    source_map: &mut HashMap<String, SourceData>,
    skin_path: &Path,
    usecim: bool,
    scale_y: f32,
) -> crate::select_bar_data::SelectBarData {
    crate::select_bar_data::SelectBarData {
        barimageon: convert_bar_sub_images(
            &bar_data.liston,
            source_map,
            skin_path,
            usecim,
            scale_y,
        ),
        barimageoff: convert_bar_sub_images(
            &bar_data.listoff,
            source_map,
            skin_path,
            usecim,
            scale_y,
        ),
        center_bar: center,
        clickable_bar: clickable.to_vec(),
        barlevel: convert_bar_sub_numbers(&bar_data.level, source_map, skin_path, usecim, scale_y),
        bartext: convert_bar_sub_text(&bar_data.text, source_map, skin_path, usecim, scale_y),
        barlamp: convert_bar_sub_images(&bar_data.lamp, source_map, skin_path, usecim, scale_y),
        barmylamp: convert_bar_sub_images(
            &bar_data.playerlamp,
            source_map,
            skin_path,
            usecim,
            scale_y,
        ),
        barrivallamp: convert_bar_sub_images(
            &bar_data.rivallamp,
            source_map,
            skin_path,
            usecim,
            scale_y,
        ),
        bartrophy: convert_bar_sub_images(&bar_data.trophy, source_map, skin_path, usecim, scale_y),
        barlabel: convert_bar_sub_images(&bar_data.label, source_map, skin_path, usecim, scale_y),
        graph_type: None,
        graph_images: None,
        graph_region: crate::stubs::Rectangle::default(),
    }
}

fn convert_bar_sub_images(
    objs: &[Option<LoaderSkinObjectData>],
    source_map: &mut HashMap<String, SourceData>,
    skin_path: &Path,
    usecim: bool,
    scale_y: f32,
) -> Vec<Option<SkinImage>> {
    objs.iter()
        .map(|opt_obj| {
            let obj_data = opt_obj.as_ref()?;
            let skin_obj = convert_skin_object(
                &obj_data.object_type,
                source_map,
                skin_path,
                usecim,
                scale_y,
            )?;
            if let SkinObject::Image(mut img) = skin_obj {
                apply_destinations(&mut img.data, &obj_data.destinations);
                Some(img)
            } else {
                None
            }
        })
        .collect()
}

fn convert_bar_sub_text(
    objs: &[Option<LoaderSkinObjectData>],
    source_map: &mut HashMap<String, SourceData>,
    skin_path: &Path,
    usecim: bool,
    scale_y: f32,
) -> Vec<Option<Box<dyn crate::skin_text::SkinText>>> {
    objs.iter()
        .map(|opt_obj| {
            let obj_data = opt_obj.as_ref()?;
            let skin_obj = convert_skin_object(
                &obj_data.object_type,
                source_map,
                skin_path,
                usecim,
                scale_y,
            )?;
            if let SkinObject::TextFont(mut stf) = skin_obj {
                apply_destinations(&mut stf.text_data.data, &obj_data.destinations);
                Some(Box::new(stf) as Box<dyn crate::skin_text::SkinText>)
            } else {
                None
            }
        })
        .collect()
}

fn convert_bar_sub_numbers(
    objs: &[Option<LoaderSkinObjectData>],
    source_map: &mut HashMap<String, SourceData>,
    skin_path: &Path,
    usecim: bool,
    scale_y: f32,
) -> Vec<Option<SkinNumber>> {
    objs.iter()
        .map(|opt_obj| {
            let obj_data = opt_obj.as_ref()?;
            let skin_obj = convert_skin_object(
                &obj_data.object_type,
                source_map,
                skin_path,
                usecim,
                scale_y,
            )?;
            if let SkinObject::Number(mut num) = skin_obj {
                apply_destinations(&mut num.data, &obj_data.destinations);
                Some(num)
            } else {
                None
            }
        })
        .collect()
}

/// Apply destination data from loader DestinationData to a runtime SkinObjectData.
/// Sets the initial position/size from the destination keyframes.
fn apply_destinations(
    data: &mut crate::skin_object::SkinObjectData,
    destinations: &[crate::json::json_skin_loader::DestinationData],
) {
    for dst in destinations {
        data.set_destination_with_int_timer_ops(
            dst.time as i64,
            dst.x as f32,
            dst.y as f32,
            dst.w as f32,
            dst.h as f32,
            dst.acc,
            dst.a,
            dst.r,
            dst.g,
            dst.b,
            dst.blend,
            dst.filter,
            dst.angle,
            dst.center,
            dst.loop_val,
            dst.timer.unwrap_or(0),
            &dst.op,
        );
    }
}

