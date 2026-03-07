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
        // wgpu NDC has y=-1 at bottom and y=1 at top, but skin coordinates
        // use y=0 at the top of the screen. Swap bottom/top so that y=0 maps
        // to NDC y=+1 (top) and y=height maps to NDC y=-1 (bottom).
        ortho.set_to_ortho(
            0.0,
            self.config.display.window_width as f32,
            self.config.display.window_height as f32,
            0.0,
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
            let bmsfile = self.bmsfile.clone().expect("bmsfile is Some");
            let mode = self.auto.unwrap_or(BMSPlayerMode::PLAY);
            let load_ok = self
                .resource
                .as_mut()
                .map(|r| r.set_bms_file(&bmsfile, mode))
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

        // current.render()
        if let Some(ref mut current) = self.current {
            current.render();
        }

        if let Some(ref mut current) = self.current
            && let Some(ref mut audio) = self.audio
        {
            current.sync_audio(audio.as_mut());
        }

        // Take sprite batch to avoid borrow conflict with self.current
        let mut sprite = self.sprite.take();
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

        if let Some(ref mut current) = self.current {
            pending_sounds = current.drain_pending_sounds();
            pending_pitch = current.take_pending_global_pitch();
            pending_handoff = current.take_score_handoff();
            pending_reload = current.take_pending_reload_bms();
            pending_change = current.take_pending_state_change();
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

        // Apply global pitch
        if let Some(pitch) = pending_pitch
            && let Some(ref mut audio) = self.audio
        {
            audio.set_global_pitch(pitch);
        }

        // Apply score handoff to PlayerResource
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
        if let Some(state_type) = pending_change {
            self.change_state(state_type);
        }

        self.process_queued_controller_commands();

        self.periodic_config_save();

        PerformanceMetrics::get().commit();

        // ImGui rendering is handled by egui in main.rs

        // Poll input (Java: done in a separate thread, Rust: done synchronously)
        if let Some(ref mut input) = self.input {
            input.poll();
        }

        // Input gating by time delta
        // Java: final long time = System.currentTimeMillis();
        //       if(time > prevtime) { prevtime = time; current.input(); ... }
        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        if time > self.lifecycle.prevtime {
            self.lifecycle.prevtime = time;
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
                    input.set_mouse_pressed();
                }
                if mouse_dragged {
                    if let Some(ref mut current) = self.current {
                        current.handle_skin_mouse_dragged(mouse_button, mouse_x, mouse_y);
                    }
                    input.set_mouse_dragged();
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

                // Twitter post (permanent stub — API deprecated)
                if input.is_activated(KeyCommand::PostTwitter) {
                    log::info!("Twitter post requested (API deprecated, no-op)");
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
            .store(true, std::sync::atomic::Ordering::Relaxed);

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
        // ShaderManager::dispose();

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
            ortho.set_to_ortho(0.0, width as f32, height as f32, 0.0, -1.0, 1.0);
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
}
