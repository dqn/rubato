use super::*;

impl MainController {
    /// Main create lifecycle method.
    ///
    /// Translated from: MainController.create()
    ///
    /// In Java this initializes SpriteBatch, fonts, input, audio, then calls
    /// initializeStates() and changeState() to enter the initial state.
    /// Java lines 416-552
    pub fn create(&mut self) {
        let t = Instant::now();
        let mut sprite = SpriteBatchHelper::create_sprite_batch();
        // Java: SpriteBatch constructor calls setToOrtho2D(0, 0, width, height)
        let mut ortho = rubato_render::color::Matrix4::new();
        // Match Java LibGDX setToOrtho2D(0, 0, width, height): Y-up projection
        // where y=0 is at the bottom and y=height is at the top.
        // The LR2 skin loader already flips Y coordinates from LR2 format (y-down)
        // to this Y-up system (e.g. y = dsth - (skin_y + skin_h) * scale).
        ortho.set_to_ortho(
            0.0,
            self.config.display.window_width as f32,
            0.0,
            self.config.display.window_height as f32,
            -1.0,
            1.0,
        );
        sprite.set_projection_matrix(&ortho);
        self.sprite = Some(sprite);

        // ImGui init: managed by beatoraja-bin (egui context), not here

        // Audio driver initialization
        // Java lines 439-446:
        // switch(config.getAudioConfig().getDriver()) {
        //     case OpenAL: audio = new GdxSoundDriver(config); break;
        // }
        // In Rust, the audio driver is injected via set_audio_driver() from the launcher.
        // If no driver was set in the constructor (for PortAudio), we log for OpenAL:
        if self.audio.is_none() {
            let driver_type = self
                .config
                .audio_config()
                .map(|ac| format!("{:?}", ac.driver))
                .unwrap_or_else(|| "None".to_string());
            log::info!(
                "Audio driver not set; driver type = {}. \
                 Launcher should call set_audio_driver() before create().",
                driver_type
            );
        }

        // Initialize states (creates PlayerResource)
        self.initialize_states();
        self.update_state_references();

        // Input polling: done synchronously in render().
        // Java spawns a thread that calls input.poll() once per millisecond,
        // but in Rust, poll() requires &mut self. The synchronous approach in
        // render() provides equivalent functionality for single-threaded rendering.

        // Enter initial state based on bmsfile
        if self.bmsfile.is_some() {
            // Java: if(resource.setBMSFile(bmsfile, auto)) changeState(PLAY)
            //       else { changeState(CONFIG); exit(); }
            let bmsfile = self.bmsfile.as_ref().expect("bmsfile is Some");
            let mode = self.auto.unwrap_or(BMSPlayerMode::PLAY);
            let load_ok = self
                .resource
                .as_mut()
                .map(|r| r.set_bms_file(bmsfile, mode))
                .unwrap_or(false);
            if load_ok {
                self.change_state(MainStateType::Play);
            } else {
                self.change_state(MainStateType::Config);
                self.exit();
            }
        } else {
            self.change_state(MainStateType::MusicSelect);
        }

        self.trigger_ln_warning();
        self.set_target_list();

        self.lifecycle.last_config_save = Instant::now();

        info!("Initialization time (ms): {}", t.elapsed().as_millis());
    }

    /// Main render lifecycle method — called every frame.
    ///
    /// Translated from: MainController.render()
    ///
    /// Java lines 606-780:
    /// ```java
    /// public void render() {
    ///     timer.update();
    ///     Gdx.gl.glClear(GL20.GL_COLOR_BUFFER_BIT);
    ///     current.render();
    ///     sprite.begin();
    ///     if (current.getSkin() != null) {
    ///         current.getSkin().updateCustomObjects(current);
    ///         current.getSkin().drawAllObjects(sprite, current);
    ///     }
    ///     sprite.end();
    ///     // ... stage, FPS display, ImGui ...
    ///     periodicConfigSave();
    ///     PerformanceMetrics.get().commit();
    ///     // Input gating
    ///     final long time = System.currentTimeMillis();
    ///     if(time > prevtime) { prevtime = time; current.input(); ... }
    /// }
    /// ```
    pub fn render(&mut self) {
        // timer.update()
        self.timer.update();

        // GL clear is handled by wgpu render pass in main.rs

        // Notify current state of media load status from PlayerResource.
        // Java: BMSPlayer.render() checks resource.mediaLoadFinished() directly;
        // Rust: BMSPlayer cannot access the resource, so MainController polls and pushes.
        if let Some(ref mut current) = self.current {
            let media_ready = self
                .resource
                .as_ref()
                .is_none_or(|r| r.media_load_finished());
            if media_ready {
                current.notify_media_load_finished();
            }
        }

        // Poll background keysound loading (non-blocking check each frame)
        if let Some(ref mut audio) = self.audio {
            audio.poll_loading();
        }

        // Push gradual loading progress to the current state each frame.
        // Audio progress comes from the audio driver; BGA progress is read
        // internally by BMSPlayer from its own BGAProcessor.
        if let Some(ref mut current) = self.current {
            let audio_progress = self.audio.as_ref().map_or(1.0, |a| a.get_progress());
            let bga_on = self.resource.as_ref().is_some_and(|r| r.is_bga_on());
            current.update_loading_progress(audio_progress, bga_on);
        }

        // current.render()
        if let Some(ref mut current) = self.current {
            current.render();
        }

        if let Some(ref mut current) = self.current
            && let Some(ref mut audio) = self.audio
        {
            current.sync_audio(audio);
        }

        // Take sprite batch to avoid borrow conflict with self.current.
        // Wrap in catch_unwind so that sprite is restored even if a panic
        // occurs between take and put-back (panic safety, same pattern as
        // state_factory in state_machine.rs).
        let mut sprite = self.sprite.take();
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if let Some(ref mut s) = sprite {
                s.begin();
            }

            // Skin update and draw — delegated to state via render_skin() override.
            // Default implementation does update_custom_objects + draw_all_objects.
            // MusicSelector overrides to add BarRenderer prepare/render around the cycle.
            if let Some(ref mut current) = self.current {
                // Read state type before mutable borrow
                let st = current.state_type();
                let data = current.main_state_data_mut();
                // Advance the state's timer each frame (Java shares one timer;
                // Rust has separate TimerManagers for controller and state).
                data.timer.update();
                data.timer.state_type = st;
                // Keep boot-relative time in sync so skin property IDs 27-29 show
                // time since application start, not time since state creation.
                data.timer
                    .set_boot_time_millis(self.lifecycle.boottime.elapsed().as_millis() as i64);

                if current.main_state_data().skin.is_some() {
                    if let Some(ref mut s) = sprite {
                        current.render_skin(s);
                    }
                } else {
                    use std::sync::Once;
                    static WARN_ONCE: Once = Once::new();
                    WARN_ONCE.call_once(|| {
                        log::warn!("No skin loaded for current state — screen will be blank");
                    });
                }
            }

            if let Some(ref mut s) = sprite {
                s.end();
            }
        })) {
            Ok(()) => {}
            Err(payload) => {
                // Reset drawing state and clear partial buffers so that if a
                // higher-level catch_unwind suppresses this panic, the next
                // render cycle won't call begin() on an already-begun batch.
                if let Some(ref mut s) = sprite {
                    s.end();
                    s.flush();
                }
                self.sprite = sprite;
                std::panic::resume_unwind(payload);
            }
        }
        self.sprite = sprite;

        // Stage update/draw skipped (no scene2d equivalent yet)

        // FPS display (Phase 22+: requires system font)

        // --- Outbox consumption: poll pending operations from current state ---
        // Order: sounds → pitch → score handoff → reload → state change (last, destroys current)
        let mut pending_sounds: Vec<(SoundType, bool)> = Vec::new();
        let mut pending_pitch: Option<f32> = None;
        let mut pending_handoff: Option<rubato_types::score_handoff::ScoreHandoff> = None;
        let mut pending_reload = false;
        let mut pending_change: Option<MainStateType> = None;
        let mut pending_play_config: Option<(
            bms_model::mode::Mode,
            rubato_types::play_config::PlayConfig,
        )> = None;

        let mut pending_replay_seed_reset = false;
        let mut pending_quick_retry_score: Option<rubato_types::score_data::ScoreData> = None;
        let mut pending_quick_retry_replay: Option<rubato_types::replay_data::ReplayData> = None;
        let mut pending_audio_config: Option<rubato_types::audio_config::AudioConfig> = None;
        let mut pending_audio_path_plays: Vec<(String, f32, bool)> = Vec::new();
        let mut pending_audio_path_stops: Vec<String> = Vec::new();
        let mut pending_player_config: Option<rubato_types::player_config::PlayerConfig> = None;

        if let Some(ref mut current) = self.current {
            pending_sounds = current.drain_pending_sounds();
            pending_pitch = current.take_pending_global_pitch();
            pending_handoff = current.take_score_handoff();
            pending_reload = current.take_pending_reload_bms();
            pending_replay_seed_reset = current.take_pending_replay_seed_reset();
            pending_quick_retry_score = current.take_pending_quick_retry_score();
            pending_quick_retry_replay = current.take_pending_quick_retry_replay();
            pending_play_config = current.take_pending_play_config_update();
            pending_player_config = current.take_pending_player_config_update();
            pending_audio_config = current.take_pending_audio_config();
            pending_audio_path_plays = current.drain_pending_audio_path_plays();
            pending_audio_path_stops = current.drain_pending_audio_path_stops();
            pending_change = current.take_pending_state_change();
        }

        // Capture sound count for observability event before consuming the Vec.
        let pending_sounds_count = pending_sounds.len();

        // Apply audio config (volume changes from skin sliders)
        // ORDERING: Must be applied BEFORE system sound playback so that
        // sounds use the updated volume, not the stale one.
        if let Some(audio_config) = pending_audio_config {
            self.config.audio = Some(audio_config);
        }

        // Apply sounds
        for (sound, loop_sound) in pending_sounds {
            let volume = self.config.audio.as_ref().map_or(1.0, |a| a.systemvolume);
            let path = self.sound.as_ref().and_then(|sm| sm.sound(&sound).cloned());
            if let Some(path) = path
                && let Some(ref mut audio) = self.audio
            {
                audio.play_path(&path, volume, loop_sound);
            }
        }

        // Apply skin-scripted audio path plays (from SkinRenderContext::audio_play)
        if let Some(ref mut audio) = self.audio {
            for (path, volume, is_loop) in pending_audio_path_plays {
                if !path.is_empty() {
                    audio.play_path(&path, volume, is_loop);
                }
            }
            for path in pending_audio_path_stops {
                if !path.is_empty() {
                    audio.stop_path(&path);
                }
            }
        }

        // Apply global pitch
        if let Some(pitch) = pending_pitch
            && let Some(ref mut audio) = self.audio
        {
            audio.set_global_pitch(pitch);
        }

        // Capture handoff summary for observability event before values are consumed.
        let handoff_summary = pending_handoff.as_ref().map(|h| {
            let exscore = h.score_data.as_ref().map_or(0, |s| s.exscore());
            let max_combo = h.maxcombo;
            let gauge = h.groove_gauge.as_ref().map_or(0.0, |g| g.value() as f64);
            (exscore, max_combo, gauge)
        });

        // Apply score handoff to PlayerResource.
        // ORDERING INVARIANT: The handoff must be applied BEFORE the state change
        // at the end of this function. BMSPlayer::render() populates replay_data
        // via build_replay_data() WITHOUT the keylog (it only has pattern/seed info).
        // This section appends the keylog from BMSPlayerInputProcessor below.
        // If the state change ran first, the input processor would be destroyed
        // and the keylog would be lost.
        if let Some(handoff) = pending_handoff
            && let Some(ref mut resource) = self.resource
        {
            if let Some(score) = handoff.score_data {
                resource.set_score_data(score);
            }
            resource.combo = handoff.combo;
            resource.maxcombo = handoff.maxcombo;
            resource.set_gauge(handoff.gauge);
            if let Some(gg) = handoff.groove_gauge {
                resource.set_groove_gauge(gg);
            }
            resource.assist = handoff.assist;
            // Java: resource.setUpdateScore(assist == 0)
            resource.update_score = handoff.assist == 0;
            // Java: resource.setUpdateCourseScore(resource.isUpdateCourseScore() && assist == 0)
            // Course scores must also be gated by assist status to prevent
            // assisted plays from saving course records.
            resource.update_course_score = resource.update_course_score && (handoff.assist == 0);
            resource.freq_on = handoff.freq_on;
            resource.force_no_ir_send = handoff.force_no_ir_send;

            // Apply replay data with key input log from the input processor.
            // BMSPlayer builds pattern info (random options, seeds, gauge type, etc.)
            // and MainController appends the recorded key input log from BMSPlayerInputProcessor.
            if let Some(mut rd) = handoff.replay_data {
                Self::append_keylog_to_replay(&self.input, &mut rd);
                resource.set_replay_data(rd);
            }

            // Update the model on PlayerResource with judge states synced from
            // JudgeManager. In Java, JudgeManager modifies Note objects in-place
            // via shared references; in Rust, the updated model is passed through
            // the handoff so the result screen reads correct state/play_time values
            // for timing distribution computation.
            if let Some(updated_model) = handoff.updated_model
                && let Some(sd) = resource.songdata_mut()
            {
                sd.set_bms_model(updated_model);
            }

            // Transfer recent judge offsets for result screen visualizers.
            resource.set_recent_judges(handoff.recent_judges_index, handoff.recent_judges);
        }

        // Emit ScoreHandoffApplied event if a handoff was processed.
        if let Some((exscore, max_combo, gauge)) = handoff_summary {
            self.emit_state_event(rubato_types::state_event::StateEvent::ScoreHandoffApplied {
                exscore,
                max_combo,
                gauge,
            });
        }

        // Apply play config update to MainController's PlayerConfig.
        // BMSPlayer owns a clone; save_config() writes to that clone and pushes
        // the updated PlayConfig back here so periodic_config_save() persists it.
        // Full replacement is intentional: BMSPlayer's save_config() writes back
        // authoritative live values (hispeed, lanecover, etc.). The modmenu path
        // uses apply_modmenu_fields() to avoid overwriting live-mutated fields.
        if let Some((mode, play_config)) = pending_play_config {
            self.player.play_config(mode).playconfig = play_config;
        }

        // Apply full PlayerConfig update from MusicSelector.
        // MusicSelector owns a clone of PlayerConfig; skin events modify it locally.
        // This outbox pushes the entire config back so periodic_config_save() persists changes.
        if let Some(player_config) = pending_player_config {
            self.player = player_config;
        }

        // Quick retry: reset replay seed (START/assist) or save score+replay (SELECT).
        // Applied before BMS reload so the next play gets the correct seed state.
        if let Some(ref mut resource) = self.resource {
            if pending_replay_seed_reset && let Some(rd) = resource.replay_data_mut() {
                rd.randomoptionseed = -1;
            }
            if let Some(score) = pending_quick_retry_score {
                resource.set_score_data(score);
            }
            if let Some(mut replay) = pending_quick_retry_replay {
                Self::append_keylog_to_replay(&self.input, &mut replay);
                resource.set_replay_data(replay);
            }
        }

        // Reload BMS file (before state change so new Play state gets fresh model)
        if pending_reload {
            if let Some(ref mut resource) = self.resource {
                resource.reload_bms_file();
            }
            // If no state change follows (practice mode restart), push the fresh model
            // back to the current state so it can apply modifiers on a clean copy.
            if pending_change.is_none() {
                let fresh_model = self.resource.as_ref().and_then(|r| r.bms_model().cloned());
                if let Some(model) = fresh_model
                    && let Some(ref mut current) = self.current
                {
                    current.receive_reloaded_model(model);
                }
            }
        }

        // State change (last - destroys current state)
        let has_state_change = pending_change.is_some();
        if let Some(state_type) = pending_change {
            self.change_state(state_type);
        }

        // Emit OutboxDrained event when sounds or state changes were processed.
        if pending_sounds_count > 0 || has_state_change {
            self.emit_state_event(rubato_types::state_event::StateEvent::OutboxDrained {
                sounds: pending_sounds_count,
                state_change: has_state_change,
            });
        }

        self.process_queued_controller_commands();

        // Prune finished background threads: join them to observe panics,
        // then retain only the still-running handles.
        let mut remaining = Vec::new();
        for handle in self.background_threads.drain(..) {
            if handle.is_finished() {
                if let Err(e) = handle.join() {
                    log::warn!("Background thread panicked: {:?}", e);
                }
            } else {
                remaining.push(handle);
            }
        }
        self.background_threads = remaining;

        self.periodic_config_save();

        PerformanceMetrics::get().commit();

        // ImGui rendering is handled by egui in main.rs

        // Input gating by time delta (Java parity: System.currentTimeMillis)
        // Note: SystemTime is not monotonic; NTP jumps could briefly gate input.
        // Using Instant would be more robust but changes timing semantics vs Java.
        // Java: final long time = System.currentTimeMillis();
        //       if(time > prevtime) { prevtime = time; current.input(); ... }
        let time = match self.lifecycle.override_input_gate_time.take() {
            Some(t) => t,
            None => std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64,
        };
        if time > self.lifecycle.prevtime {
            self.lifecycle.prevtime = time;
            // Poll input (Java: done in a separate thread, Rust: done synchronously).
            // Polling inside the time gate ensures no intermediate key transitions
            // are lost between poll and sync_input_from/input/sync_input_back_to.
            if let Some(ref mut input) = self.input {
                input.poll();
            }
            if let Some(ref input) = self.input
                && let Some(ref mut current) = self.current
            {
                current.sync_input_from(input);
            }
            if let Some(ref mut current) = self.current {
                current.input();
            }
            if let Some(ref mut input) = self.input
                && let Some(ref mut current) = self.current
            {
                current.sync_input_back_to(input);
            }
            // Mouse pressed/dragged → skin
            // Java: if (input.isMousePressed()) {
            //     current.getSkin().mousePressed(current, input.getMouseButton(), input.getMouseX(), input.getMouseY());
            // }
            // Java: if (input.isMouseDragged()) {
            //     current.getSkin().mouseDragged(current, input.getMouseButton(), input.getMouseX(), input.getMouseY());
            // }
            if let Some(ref mut input) = self.input {
                let mouse_pressed = input.is_mouse_pressed();
                let mouse_dragged = input.is_mouse_dragged();
                let mouse_button = input.mousebutton;
                let mouse_x = input.mousex;
                let mouse_y = input.mousey;
                if mouse_pressed {
                    if let Some(ref mut current) = self.current {
                        current.handle_skin_mouse_pressed(mouse_button, mouse_x, mouse_y);
                    }
                    input.consume_mouse_pressed();
                }
                if mouse_dragged {
                    if let Some(ref mut current) = self.current {
                        current.handle_skin_mouse_dragged(mouse_button, mouse_x, mouse_y);
                    }
                    input.consume_mouse_dragged();
                }

                // Mouse moved → cursor visibility timer
                if input.is_mouse_moved() {
                    self.lifecycle.mouse_moved_time = time;
                    input.mouse_moved = false;
                }
            }

            // KeyCommand handlers (Java: MainController.render() lines 727-819)
            if let Some(ref mut input) = self.input {
                // FPS display toggle
                if input.is_activated(KeyCommand::ShowFps) {
                    self.showfps = !self.showfps;
                    log::info!("FPS display: {}", if self.showfps { "ON" } else { "OFF" });
                }

                // Fullscreen / windowed toggle (F4 without Alt held)
                // Java: if (!ALT_LEFT && !ALT_RIGHT && SWITCH_SCREEN_MODE)
                if !input.is_alt_held() && input.is_activated(KeyCommand::SwitchScreenMode) {
                    crate::window_command::request_fullscreen_toggle();
                    log::info!("Fullscreen toggle requested");
                }

                // Screenshot
                if input.is_activated(KeyCommand::SaveScreenshot) {
                    crate::window_command::request_screenshot();
                    log::info!("Screenshot requested");
                }

                // Mod menu toggle
                if input.is_activated(KeyCommand::ToggleModMenu)
                    && let Some(ref mut imgui) = self.integration.imgui
                {
                    imgui.toggle_menu();
                }
            }
        }
    }

    /// Dispose lifecycle — called on application shutdown.
    ///
    /// Translated from: MainController.dispose()
    pub fn dispose(&mut self) {
        self.save_config();

        // Stop input polling
        self.input_poll_quit
            .store(true, std::sync::atomic::Ordering::Release);

        // Dispose input processor
        if let Some(ref mut input) = self.input {
            input.dispose();
        }

        // Dispose current state
        if let Some(ref mut current) = self.current {
            current.dispose();
        }
        self.current = None;

        // Java: if (streamController != null) { streamController.dispose(); }
        if let Some(ref mut sc) = self.integration.stream_controller {
            sc.dispose();
        }
        self.integration.stream_controller = None;

        if let Some(mut imgui) = self.integration.imgui.take() {
            imgui.dispose();
        }
        if let Some(mut resource) = self.resource.take() {
            resource.dispose();
        }
        // ShaderManager removed: LibGDX shader management not needed with wgpu.

        // Stop the IR resend background thread.
        if let Some(ref service) = self.integration.ir_resend_service {
            service.stop();
        }
        self.integration.ir_resend_service = None;

        // Dispose OBS client before audio driver so its Drop impl runs while
        // other resources are still intact. ObsAccess has no explicit close()
        // method; dropping it disconnects the WebSocket.
        if let Some(obs) = self.integration.obs_client.take() {
            drop(obs);
        }

        // Join background threads (song update, table update) to ensure clean
        // shutdown and release of DB handles.
        for handle in self.background_threads.drain(..) {
            if let Err(e) = handle.join() {
                log::warn!("Background thread panicked during shutdown: {:?}", e);
            }
        }

        // Dispose audio driver to release Kira's AudioManager and its background
        // cpal thread.
        if let Some(ref mut audio) = self.audio {
            audio.dispose();
        }
        self.audio = None;

        info!("All resources disposed");
    }

    /// Pause lifecycle — dispatches to current state.
    ///
    /// Translated from: MainController.pause()
    pub fn pause(&mut self) {
        if let Some(ref mut current) = self.current {
            current.pause();
        }
    }

    /// Resize lifecycle — dispatches to current state.
    ///
    /// Translated from: MainController.resize(int, int)
    pub fn resize(&mut self, width: i32, height: i32) {
        // Update sprite batch projection to match new window size
        // Java: SpriteBatch projection is updated via Gdx.graphics viewport
        if let Some(ref mut sprite) = self.sprite {
            let mut ortho = rubato_render::color::Matrix4::new();
            ortho.set_to_ortho(0.0, width as f32, 0.0, height as f32, -1.0, 1.0);
            sprite.set_projection_matrix(&ortho);
        }
        if let Some(ref mut current) = self.current {
            current.resize(width, height);
        }
    }

    /// Resume lifecycle — dispatches to current state.
    ///
    /// Translated from: MainController.resume()
    pub fn resume(&mut self) {
        if let Some(ref mut current) = self.current {
            current.resume();
        }
    }

    /// Save config and player config to disk.
    ///
    /// Translated from: MainController.saveConfig()
    ///
    /// Java lines 883-887:
    /// ```java
    /// public void saveConfig(){
    ///     Config.write(config);
    ///     PlayerConfig.write(config.getPlayerpath(), player);
    ///     logger.info("設定情報を保存");
    /// }
    /// ```
    pub fn save_config(&self) {
        if let Err(e) = Config::write(&self.config) {
            log::error!("Failed to write config: {}", e);
        }
        if let Err(e) = PlayerConfig::write(&self.config.paths.playerpath, &self.player) {
            log::error!("Failed to write player config: {}", e);
        }
        info!("Config saved");
    }

    /// Request application exit. Sets exit flag and saves config.
    ///
    /// Translated from: MainController.exit()
    ///
    /// Java lines 919-921:
    /// ```java
    /// public void exit() {
    ///     Gdx.app.exit();
    /// }
    /// ```
    ///
    /// In Java, Gdx.app.exit() triggers the LibGDX lifecycle (pause → dispose),
    /// and dispose() calls saveConfig(). In Rust, we set an exit flag and save
    /// config immediately, since the main loop checks is_exit_requested().
    pub fn exit(&self) {
        self.exit_requested.store(true, Ordering::Release);
        self.save_config();
        info!("Exit requested");
    }

    /// Check whether exit has been requested.
    ///
    /// The main event loop should poll this and initiate shutdown when true.
    pub fn is_exit_requested(&self) -> bool {
        self.exit_requested.load(Ordering::Acquire)
    }

    /// Append recorded key input log from BMSPlayerInputProcessor to replay data.
    ///
    /// Both the normal score handoff path and the quick retry path need this
    /// so that saved replays contain actual key events instead of being empty.
    fn append_keylog_to_replay(
        input: &Option<BMSPlayerInputProcessor>,
        replay: &mut rubato_types::replay_data::ReplayData,
    ) {
        if let Some(input) = input {
            replay.keylog = input
                .key_input_log()
                .iter()
                .map(|k| rubato_types::KeyInputLog {
                    time: k.time(),
                    keycode: k.keycode(),
                    pressed: k.is_pressed(),
                })
                .collect();
        }
    }
}
