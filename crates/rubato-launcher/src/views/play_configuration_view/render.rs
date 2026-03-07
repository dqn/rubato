use super::*;

impl PlayConfigurationView {
    /// Render the UI using egui widgets.
    ///
    /// Replaces the JavaFX FXML layout. Groups config fields into collapsible
    /// sections so the long list of options remains navigable.
    pub fn render(&mut self, ui: &mut egui::Ui) {
        // ---- Player selector ----
        ui.heading("Player");
        egui::Grid::new("pcv_player_grid")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Player:");
                let selected_text = self
                    .players_selected
                    .clone()
                    .unwrap_or_else(|| "(none)".to_string());
                egui::ComboBox::from_id_salt("pcv_player_select")
                    .selected_text(&selected_text)
                    .show_ui(ui, |ui| {
                        for p in &self.players {
                            let is_selected = self.players_selected.as_deref() == Some(p.as_str());
                            if ui.selectable_label(is_selected, p).clicked() && !is_selected {
                                self.players_selected = Some(p.clone());
                                // Trigger player change on next frame
                            }
                        }
                    });
                ui.end_row();

                ui.label("Player Name:");
                ui.text_edit_singleline(&mut self.playername);
                ui.end_row();
            });

        if ui.button("Add Player").clicked() {
            self.add_player();
        }

        ui.separator();

        // ---- Play mode / Hi-speed ----
        ui.heading("Play Config");
        egui::Grid::new("pcv_playconfig_grid")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Play Mode:");
                {
                    let selected_text = self
                        .playconfig
                        .as_ref()
                        .map(|m| m.display_name().to_string())
                        .unwrap_or_else(|| "(none)".to_string());
                    egui::ComboBox::from_id_salt("pcv_playmode")
                        .selected_text(&selected_text)
                        .show_ui(ui, |ui| {
                            for mode in PlayMode::values() {
                                let is_selected = self.playconfig.as_ref() == Some(&mode);
                                if ui
                                    .selectable_label(is_selected, mode.display_name())
                                    .clicked()
                                {
                                    self.playconfig = Some(mode);
                                }
                            }
                        });
                }
                ui.end_row();

                ui.label("Hi-Speed:");
                ui.add(
                    egui::DragValue::new(&mut self.hispeed)
                        .range(0.01..=20.0)
                        .speed(0.01),
                );
                ui.end_row();

                ui.label("Hi-Speed Auto Adjust:");
                ui.checkbox(&mut self.hispeedautoadjust, "");
                ui.end_row();

                ui.label("Hi-Speed Margin:");
                ui.add(
                    egui::DragValue::new(&mut self.hispeedmargin)
                        .range(0.0..=10.0)
                        .speed(0.01),
                );
                ui.end_row();

                ui.label("Fix Hi-Speed:");
                Self::render_combo_i32(
                    ui,
                    "pcv_fixhispeed",
                    &mut self.fixhispeed,
                    &self.fixhispeed_labels,
                );
                ui.end_row();

                ui.label("Green Value:");
                ui.add(egui::DragValue::new(&mut self.gvalue).range(0..=9999));
                ui.end_row();

                ui.label("Constant Mode:");
                ui.checkbox(&mut self.enable_constant, "");
                ui.end_row();

                if self.enable_constant {
                    ui.label("Constant Fade-in (ms):");
                    ui.add(egui::DragValue::new(&mut self.const_fadein_time).range(0..=10000));
                    ui.end_row();
                }
            });

        ui.separator();

        // ---- Score options ----
        ui.heading("Score Options");
        egui::Grid::new("pcv_scoreoptions_grid")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("1P Random:");
                Self::render_combo_i32(
                    ui,
                    "pcv_scoreop",
                    &mut self.scoreop,
                    &self.score_options_labels,
                );
                ui.end_row();

                ui.label("2P Random:");
                Self::render_combo_i32(
                    ui,
                    "pcv_scoreop2",
                    &mut self.scoreop2,
                    &self.score_options_labels,
                );
                ui.end_row();

                ui.label("Double Option:");
                Self::render_combo_i32(
                    ui,
                    "pcv_doubleop",
                    &mut self.doubleop,
                    &self.double_options_labels,
                );
                ui.end_row();

                ui.label("Gauge:");
                Self::render_combo_i32(
                    ui,
                    "pcv_gaugeop",
                    &mut self.gaugeop,
                    &self.gauge_options_labels,
                );
                ui.end_row();

                ui.label("LN Type:");
                Self::render_combo_i32(ui, "pcv_lntype", &mut self.lntype, &self.lntype_labels);
                ui.end_row();
            });

        ui.separator();

        // ---- Lane cover / Lift / Hidden ----
        ui.heading("Lane Cover / Lift / Hidden");
        egui::Grid::new("pcv_lanecover_grid")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Enable Lane Cover:");
                ui.checkbox(&mut self.enable_lanecover, "");
                ui.end_row();

                if self.enable_lanecover {
                    ui.label("Lane Cover:");
                    ui.add(egui::DragValue::new(&mut self.lanecover).range(0..=1000));
                    ui.end_row();

                    ui.label("Margin Low:");
                    ui.add(egui::DragValue::new(&mut self.lanecovermarginlow).range(0..=1000));
                    ui.end_row();

                    ui.label("Margin High:");
                    ui.add(egui::DragValue::new(&mut self.lanecovermarginhigh).range(0..=1000));
                    ui.end_row();

                    ui.label("Switch Duration:");
                    ui.add(
                        egui::DragValue::new(&mut self.lanecoverswitchduration).range(0..=10000),
                    );
                    ui.end_row();
                }

                ui.label("Enable Lift:");
                ui.checkbox(&mut self.enable_lift, "");
                ui.end_row();

                if self.enable_lift {
                    ui.label("Lift:");
                    ui.add(egui::DragValue::new(&mut self.lift).range(0..=1000));
                    ui.end_row();
                }

                ui.label("Enable Hidden:");
                ui.checkbox(&mut self.enable_hidden, "");
                ui.end_row();

                if self.enable_hidden {
                    ui.label("Hidden:");
                    ui.add(egui::DragValue::new(&mut self.hidden).range(0..=1000));
                    ui.end_row();
                }
            });

        ui.separator();

        // ---- Timing ----
        ui.heading("Timing");
        egui::Grid::new("pcv_timing_grid")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Notes Display Timing:");
                ui.add(egui::DragValue::new(&mut self.notesdisplaytiming).range(-999..=999));
                ui.end_row();

                ui.label("Auto Adjust:");
                ui.checkbox(&mut self.notesdisplaytimingautoadjust, "");
                ui.end_row();

                ui.label("BPM Guide:");
                ui.checkbox(&mut self.bpmguide, "");
                ui.end_row();

                ui.label("Gauge Auto Shift:");
                Self::render_combo_i32(
                    ui,
                    "pcv_gaugeautoshift",
                    &mut self.gaugeautoshift,
                    &self.gaugeautoshift_labels,
                );
                ui.end_row();

                ui.label("Bottom Shiftable Gauge:");
                Self::render_combo_i32(
                    ui,
                    "pcv_bottomshiftablegauge",
                    &mut self.bottomshiftablegauge,
                    &self.bottomshiftablegauge_labels,
                );
                ui.end_row();

                ui.label("Judge Algorithm:");
                Self::render_combo_i32(
                    ui,
                    "pcv_judgealgorithm",
                    &mut self.judgealgorithm,
                    &self.judgealgorithm_labels,
                );
                ui.end_row();
            });

        ui.separator();

        // ---- Custom Judge ----
        ui.heading("Custom Judge");
        egui::Grid::new("pcv_customjudge_grid")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Enable Custom Judge:");
                ui.checkbox(&mut self.customjudge, "");
                ui.end_row();

                if self.customjudge {
                    ui.label("Normal PG:");
                    ui.add(egui::DragValue::new(&mut self.njudgepg).range(0..=9999));
                    ui.end_row();

                    ui.label("Normal GR:");
                    ui.add(egui::DragValue::new(&mut self.njudgegr).range(0..=9999));
                    ui.end_row();

                    ui.label("Normal GD:");
                    ui.add(egui::DragValue::new(&mut self.njudgegd).range(0..=9999));
                    ui.end_row();

                    ui.label("Scratch PG:");
                    ui.add(egui::DragValue::new(&mut self.sjudgepg).range(0..=9999));
                    ui.end_row();

                    ui.label("Scratch GR:");
                    ui.add(egui::DragValue::new(&mut self.sjudgegr).range(0..=9999));
                    ui.end_row();

                    ui.label("Scratch GD:");
                    ui.add(egui::DragValue::new(&mut self.sjudgegd).range(0..=9999));
                    ui.end_row();
                }
            });

        ui.separator();

        // ---- Mine / Scroll / LN modes ----
        ui.heading("Note Modifiers");
        egui::Grid::new("pcv_notemod_grid")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Mine Mode:");
                Self::render_combo_i32(
                    ui,
                    "pcv_minemode",
                    &mut self.minemode,
                    &self.minemode_labels,
                );
                ui.end_row();

                ui.label("Scroll Mode:");
                Self::render_combo_i32(
                    ui,
                    "pcv_scrollmode",
                    &mut self.scrollmode,
                    &self.scrollmode_labels,
                );
                ui.end_row();

                ui.label("LN Mode:");
                Self::render_combo_i32(
                    ui,
                    "pcv_longnotemode",
                    &mut self.longnotemode,
                    &self.longnotemode_labels,
                );
                ui.end_row();

                ui.label("Forced CN Endings:");
                ui.checkbox(&mut self.forcedcnendings, "");
                ui.end_row();

                ui.label("LN Rate:");
                ui.add(
                    egui::DragValue::new(&mut self.longnoterate)
                        .range(0.0..=10.0)
                        .speed(0.01),
                );
                ui.end_row();

                ui.label("H-RAN Threshold BPM:");
                ui.add(egui::DragValue::new(&mut self.hranthresholdbpm).range(0..=999));
                ui.end_row();

                ui.label("7 to 9 Pattern:");
                Self::render_combo_i32(
                    ui,
                    "pcv_seventoninepattern",
                    &mut self.seventoninepattern,
                    &self.seven_to_nine_pattern_labels,
                );
                ui.end_row();

                ui.label("7 to 9 Type:");
                Self::render_combo_i32(
                    ui,
                    "pcv_seventoninetype",
                    &mut self.seventoninetype,
                    &self.seven_to_nine_type_labels,
                );
                ui.end_row();

                ui.label("Extra Note Depth:");
                ui.add(egui::DragValue::new(&mut self.extranotedepth).range(0..=100));
                ui.end_row();
            });

        ui.separator();

        // ---- Visual options ----
        ui.heading("Visual");
        egui::Grid::new("pcv_visual_grid")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Judge Region:");
                ui.checkbox(&mut self.judgeregion, "");
                ui.end_row();

                ui.label("Mark Processed Note:");
                ui.checkbox(&mut self.markprocessednote, "");
                ui.end_row();

                ui.label("Show Hidden Note:");
                ui.checkbox(&mut self.showhiddennote, "");
                ui.end_row();

                ui.label("Show Past Note:");
                ui.checkbox(&mut self.showpastnote, "");
                ui.end_row();

                ui.label("Target:");
                {
                    let selected_text = self
                        .target_selected
                        .clone()
                        .unwrap_or_else(|| "(none)".to_string());
                    egui::ComboBox::from_id_salt("pcv_target")
                        .selected_text(&selected_text)
                        .show_ui(ui, |ui| {
                            for t in &self.target {
                                let is_selected =
                                    self.target_selected.as_deref() == Some(t.as_str());
                                if ui.selectable_label(is_selected, t).clicked() {
                                    self.target_selected = Some(t.clone());
                                }
                            }
                        });
                }
                ui.end_row();
            });

        ui.separator();

        // ---- Auto-save replays ----
        ui.heading("Auto Save Replay");
        egui::Grid::new("pcv_autosave_grid")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Replay 1:");
                Self::render_combo_i32(
                    ui,
                    "pcv_autosave1",
                    &mut self.autosavereplay1,
                    &self.autosave_labels,
                );
                ui.end_row();

                ui.label("Replay 2:");
                Self::render_combo_i32(
                    ui,
                    "pcv_autosave2",
                    &mut self.autosavereplay2,
                    &self.autosave_labels,
                );
                ui.end_row();

                ui.label("Replay 3:");
                Self::render_combo_i32(
                    ui,
                    "pcv_autosave3",
                    &mut self.autosavereplay3,
                    &self.autosave_labels,
                );
                ui.end_row();

                ui.label("Replay 4:");
                Self::render_combo_i32(
                    ui,
                    "pcv_autosave4",
                    &mut self.autosavereplay4,
                    &self.autosave_labels,
                );
                ui.end_row();
            });

        ui.separator();

        // ---- Misc ----
        ui.heading("Miscellaneous");
        egui::Grid::new("pcv_misc_grid")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Exit Press Duration (ms):");
                ui.add(egui::DragValue::new(&mut self.exitpressduration).range(0..=10000));
                ui.end_row();

                ui.label("Chart Preview:");
                ui.checkbox(&mut self.chartpreview, "");
                ui.end_row();

                ui.label("Guide SE:");
                ui.checkbox(&mut self.guidese, "");
                ui.end_row();

                ui.label("Window Hold:");
                ui.checkbox(&mut self.windowhold, "");
                ui.end_row();

                ui.label("Cache Skin Image:");
                ui.checkbox(&mut self.usecim, "");
                ui.end_row();

                ui.label("Clipboard Screenshot:");
                ui.checkbox(&mut self.clipboard_screenshot, "");
                ui.end_row();
            });

        ui.separator();

        // ---- Paths ----
        ui.heading("Paths");
        egui::Grid::new("pcv_paths_grid")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("BGM Path:");
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut self.bgmpath);
                    if ui.button("Browse...").clicked() {
                        self.add_bgm_path();
                    }
                });
                ui.end_row();

                ui.label("Sound Path:");
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut self.soundpath);
                    if ui.button("Browse...").clicked() {
                        self.add_sound_path();
                    }
                });
                ui.end_row();
            });

        ui.separator();

        // ---- IPFS ----
        ui.heading("IPFS");
        egui::Grid::new("pcv_ipfs_grid")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Enable IPFS:");
                ui.checkbox(&mut self.enable_ipfs, "");
                ui.end_row();

                if self.enable_ipfs {
                    ui.label("IPFS URL:");
                    ui.text_edit_singleline(&mut self.ipfsurl);
                    ui.end_row();
                }
            });

        ui.separator();

        // ---- HTTP Download ----
        ui.heading("HTTP Download");
        egui::Grid::new("pcv_http_grid")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Enable HTTP:");
                ui.checkbox(&mut self.enable_http, "");
                ui.end_row();

                if self.enable_http {
                    ui.label("Download Source:");
                    {
                        let selected_text = self
                            .http_download_source_selected
                            .clone()
                            .unwrap_or_else(|| "(none)".to_string());
                        egui::ComboBox::from_id_salt("pcv_http_source")
                            .selected_text(&selected_text)
                            .show_ui(ui, |ui| {
                                for src in &self.http_download_source {
                                    let is_selected = self.http_download_source_selected.as_deref()
                                        == Some(src.as_str());
                                    if ui.selectable_label(is_selected, src).clicked() {
                                        self.http_download_source_selected = Some(src.clone());
                                    }
                                }
                            });
                    }
                    ui.end_row();

                    ui.label("Override URL:");
                    ui.text_edit_singleline(&mut self.override_download_url);
                    ui.end_row();
                }
            });

        ui.separator();

        // ---- Twitter (deprecated) ----
        egui::CollapsingHeader::new("Twitter (deprecated)")
            .default_open(false)
            .show(ui, |ui| {
                egui::Grid::new("pcv_twitter_grid")
                    .num_columns(2)
                    .show(ui, |ui| {
                        ui.label("Consumer Key:");
                        ui.text_edit_singleline(&mut self.txt_twitter_consumer_key);
                        ui.end_row();

                        ui.label("Consumer Secret:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.txt_twitter_consumer_secret)
                                .password(true),
                        );
                        ui.end_row();

                        if self.txt_twitter_authenticated_visible {
                            ui.label("Status:");
                            ui.label("Authenticated");
                            ui.end_row();
                        }

                        ui.label("PIN:");
                        ui.add_enabled(
                            self.twitter_pin_enabled,
                            egui::TextEdit::singleline(&mut self.txt_twitter_pin),
                        );
                        ui.end_row();
                    });

                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            self.twitter_auth_button_enabled,
                            egui::Button::new("Start Auth"),
                        )
                        .clicked()
                    {
                        self.start_twitter_auth();
                    }
                    if ui
                        .add_enabled(self.twitter_pin_enabled, egui::Button::new("Submit PIN"))
                        .clicked()
                    {
                        self.start_pin_auth();
                    }
                });
            });

        ui.separator();

        // ---- New version banner ----
        if !self.newversion_text.is_empty() {
            ui.horizontal(|ui| {
                ui.label(&self.newversion_text);
                if self.newversion_url.is_some() && ui.button("Download").clicked() {
                    // URL open handled externally
                }
            });
            ui.separator();
        }

        // ---- Control buttons ----
        ui.horizontal(|ui| {
            let disabled = self.control_panel_disabled;
            if ui
                .add_enabled(!disabled, egui::Button::new("Start"))
                .clicked()
            {
                self.start();
            }
            if ui
                .add_enabled(!disabled, egui::Button::new("Load All BMS"))
                .clicked()
            {
                self.load_all_bms();
            }
            if ui
                .add_enabled(!disabled, egui::Button::new("Load Diff BMS"))
                .clicked()
            {
                self.load_diff_bms();
            }
            if ui
                .add_enabled(!disabled, egui::Button::new("Import LR2 Scores"))
                .clicked()
            {
                self.import_score_data_from_lr2();
            }
            if ui.button("Exit").clicked() {
                self.exit();
            }
        });

        // ---- BMS loading progress ----
        match self.bms_loading_state() {
            BmsLoadingState::Idle => {}
            BmsLoadingState::Loading {
                bms_files,
                processed_files,
                new_files,
            } => {
                ui.separator();
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(format!(
                        "Loading BMS: {processed_files}/{bms_files} processed, {new_files} new"
                    ));
                });
            }
            BmsLoadingState::Completed => {
                ui.separator();
                ui.label("BMS loading completed.");
                if ui.button("Dismiss").clicked() {
                    self.reset_bms_loading();
                }
            }
            BmsLoadingState::Failed(ref msg) => {
                ui.separator();
                ui.colored_label(egui::Color32::RED, format!("BMS loading failed: {msg}"));
                if ui.button("Dismiss").clicked() {
                    self.reset_bms_loading();
                }
            }
        }
    }

    /// Render a ComboBox for `Option<i32>` backed by a label list.
    ///
    /// Shared helper used by all the indexed combo box fields.
    fn render_combo_i32(ui: &mut egui::Ui, id: &str, value: &mut Option<i32>, labels: &[String]) {
        let selected_text = value
            .and_then(|v| labels.get(v as usize))
            .cloned()
            .unwrap_or_else(|| "(none)".to_string());
        egui::ComboBox::from_id_salt(id)
            .selected_text(&selected_text)
            .show_ui(ui, |ui| {
                for (i, label) in labels.iter().enumerate() {
                    let is_selected = *value == Some(i as i32);
                    if ui.selectable_label(is_selected, label).clicked() {
                        *value = Some(i as i32);
                    }
                }
            });
    }
}
