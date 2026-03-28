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
            self.ctx.config.display.window_width as f32,
            0.0,
            self.ctx.config.display.window_height as f32,
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
        if self.ctx.audio.is_none() {
            let driver_type = self
                .ctx
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

        self.ctx.lifecycle.last_config_save = Instant::now();

        info!("Initialization time (ms): {}", t.elapsed().as_millis());
    }

    /// Main render lifecycle method -- called every frame.
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
        self.ctx.timer.update();

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
        if let Some(ref mut audio) = self.ctx.audio {
            audio.poll_loading();
        }

        // Push gradual loading progress to the current state each frame.
        // Audio progress comes from the audio driver; BGA progress is read
        // internally by BMSPlayer from its own BGAProcessor.
        if let Some(ref mut current) = self.current {
            let audio_progress = self.ctx.audio.as_ref().map_or(1.0, |a| a.get_progress());
            let bga_on = self.resource.as_ref().is_some_and(|r| r.is_bga_on());
            current.update_loading_progress(audio_progress, bga_on);
        }

        // current.render() -- take the state out to avoid borrow conflict
        // between `self.current` and `self.ctx`.
        if let Some(mut current) = self.current.take() {
            // Move PlayerResource into ctx so states using render_with_game_context
            // can access it via ctx.resource without a separate accessor.
            self.ctx.resource = self.resource.take();

            // Render with GameContext. ChangeTo/Exit are stored and applied
            // AFTER the outbox drain so that sounds, score handoff, and config
            // updates are not lost.
            let transition = current.render_with_game_context(&mut self.ctx);
            match transition {
                StateTransition::Continue => { /* continue normal frame */ }
                StateTransition::ChangeTo(state_type) => {
                    self.ctx.transition = Some(StateTransition::ChangeTo(state_type));
                }
                StateTransition::Exit => {
                    self.ctx.transition = Some(StateTransition::Exit);
                }
            }

            // Restore resource from ctx back to controller.
            self.resource = self.ctx.resource.take();
            self.current = Some(current);
        }

        if let Some(ref mut current) = self.current
            && let Some(ref mut audio) = self.ctx.audio
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

            // Skin update and draw -- delegated to state via render_skin() override.
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
                    .set_boot_time_millis(self.ctx.lifecycle.boottime.elapsed().as_millis() as i64);

                if current.main_state_data().skin.is_some() {
                    if let Some(ref mut s) = sprite {
                        current.render_skin(s);
                    }
                } else {
                    use std::sync::Once;
                    static WARN_ONCE: Once = Once::new();
                    WARN_ONCE.call_once(|| {
                        log::warn!("No skin loaded for current state -- screen will be blank");
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

        // --- Process state transition from render_with_game_context ---
        // All outbox fields (score handoff, play config, quick retry, reload BMS,
        // player config) are now drained directly by states in their
        // render_with_game_context methods. Only the transition result needs
        // processing here.
        let mut pending_change: Option<MainStateType> = None;

        if let Some(transition) = self.ctx.transition.take() {
            match transition {
                StateTransition::ChangeTo(state_type) => {
                    pending_change = Some(state_type);
                }
                StateTransition::Exit => {
                    self.exit();
                    return;
                }
                StateTransition::Continue => {}
            }
        }

        // State change (destroys current state)
        let has_state_change = pending_change.is_some();
        if let Some(state_type) = pending_change {
            self.change_state(state_type);
        }

        // Emit OutboxDrained event when state changes were processed.
        if has_state_change {
            self.emit_state_event(rubato_types::state_event::StateEvent::OutboxDrained {
                state_change: has_state_change,
            });
        }

        // Drain modmenu outbox (egui callbacks -> MainController)
        {
            let modmenu_actions = self.ctx.modmenu_outbox.drain();
            for (mode, play_config) in modmenu_actions.play_config_updates {
                let pc = *play_config;
                self.ctx
                    .player
                    .play_config(mode)
                    .playconfig
                    .apply_modmenu_fields(&pc);
                if let Some(ref mut state) = self.current {
                    state.receive_updated_play_config(mode, pc);
                }
            }
            if let Some(pc) = modmenu_actions.load_new_profile {
                self.load_new_profile(*pc);
            }
            if modmenu_actions.save_config {
                self.save_config();
            }
            for (id, skin_config) in modmenu_actions.skin_config_updates {
                self.ctx.update_skin_config(id, skin_config.map(|c| *c));
            }
            for (path, skin_config) in modmenu_actions.skin_history_updates {
                self.ctx.update_skin_history(&path, *skin_config);
            }
        }

        // Drain typed command queue
        for cmd in std::mem::take(&mut self.ctx.commands) {
            match cmd {
                crate::core::command::Command::UpdateSong(path_opt) => {
                    let path = path_opt.as_deref().unwrap_or("");
                    self.update_song(path);
                }
                crate::core::command::Command::UpdateTable(source) => {
                    self.update_table(source);
                }
                crate::core::command::Command::LoadNewProfile(pc) => {
                    self.load_new_profile(*pc);
                }
            }
        }

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
        let time = match self.ctx.lifecycle.override_input_gate_time.take() {
            Some(t) => t,
            None => std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64,
        };
        if time > self.ctx.lifecycle.prevtime {
            self.ctx.lifecycle.prevtime = time;
            // Poll input (Java: done in a separate thread, Rust: done synchronously).
            // Polling inside the time gate ensures no intermediate key transitions
            // are lost between poll and sync_input_from/input/sync_input_back_to.
            if let Some(ref mut input) = self.ctx.input {
                input.poll();
            }
            if let Some(ref input) = self.ctx.input
                && let Some(ref mut current) = self.current
            {
                current.sync_input_from(input);
            }
            // Build a read-only input snapshot and pass it to the current state.
            // This coexists with sync_input_from during migration; states opt in
            // by overriding sync_input_snapshot().
            if let Some(ref input) = self.ctx.input
                && let Some(ref mut current) = self.current
            {
                let snapshot = input.build_snapshot();
                current.sync_input_snapshot(&snapshot);
            }
            // Take the state out to avoid borrow conflict between
            // `self.current` and `self.ctx`.
            if let Some(mut current) = self.current.take() {
                // Move PlayerResource into ctx for input handling.
                self.ctx.resource = self.resource.take();

                current.input_with_game_context(&mut self.ctx);

                // Restore resource from ctx back to controller.
                self.resource = self.ctx.resource.take();
                self.current = Some(current);
            }
            if let Some(ref mut input) = self.ctx.input
                && let Some(ref mut current) = self.current
            {
                current.sync_input_back_to(input);
            }
            // Mouse pressed/dragged -> skin
            // Java: if (input.isMousePressed()) {
            //     current.getSkin().mousePressed(current, input.getMouseButton(), input.getMouseX(), input.getMouseY());
            // }
            // Java: if (input.isMouseDragged()) {
            //     current.getSkin().mouseDragged(current, input.getMouseButton(), input.getMouseX(), input.getMouseY());
            // }
            if let Some(ref mut input) = self.ctx.input {
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

                // Mouse moved -> cursor visibility timer
                if input.is_mouse_moved() {
                    self.ctx.lifecycle.mouse_moved_time = time;
                    input.mouse_moved = false;
                }
            }

            // KeyCommand handlers (Java: MainController.render() lines 727-819)
            if let Some(ref mut input) = self.ctx.input {
                // FPS display toggle
                if input.is_activated(KeyCommand::ShowFps) {
                    self.ctx.showfps = !self.ctx.showfps;
                    log::info!(
                        "FPS display: {}",
                        if self.ctx.showfps { "ON" } else { "OFF" }
                    );
                }

                // Fullscreen / windowed toggle (F4 without Alt held)
                // Java: if (!ALT_LEFT && !ALT_RIGHT && SWITCH_SCREEN_MODE)
                if !input.is_alt_held() && input.is_activated(KeyCommand::SwitchScreenMode) {
                    crate::core::window_command::request_fullscreen_toggle();
                    log::info!("Fullscreen toggle requested");
                }

                // Screenshot
                if input.is_activated(KeyCommand::SaveScreenshot) {
                    crate::core::window_command::request_screenshot();
                    log::info!("Screenshot requested");
                }

                // Mod menu toggle
                if input.is_activated(KeyCommand::ToggleModMenu)
                    && let Some(ref mut imgui) = self.ctx.integration.imgui
                {
                    imgui.toggle_menu();
                }
            }
        }
    }

    /// Dispose lifecycle -- called on application shutdown.
    ///
    /// Translated from: MainController.dispose()
    pub fn dispose(&mut self) {
        self.save_config();

        // Stop input polling
        self.ctx
            .input_poll_quit
            .store(true, std::sync::atomic::Ordering::Release);

        // Dispose input processor
        if let Some(ref mut input) = self.ctx.input {
            input.dispose();
        }

        // Dispose current state
        if let Some(ref mut current) = self.current {
            current.dispose();
        }
        self.current = None;

        // Java: if (streamController != null) { streamController.dispose(); }
        if let Some(ref mut sc) = self.ctx.integration.stream_controller {
            sc.dispose();
        }
        self.ctx.integration.stream_controller = None;

        if let Some(mut imgui) = self.ctx.integration.imgui.take() {
            imgui.dispose();
        }
        if let Some(mut resource) = self.resource.take() {
            resource.dispose();
        }
        // ShaderManager removed: LibGDX shader management not needed with wgpu.

        // Stop the IR resend background thread.
        if let Some(ref service) = self.ctx.integration.ir_resend_service {
            service.stop();
        }
        self.ctx.integration.ir_resend_service = None;

        // Dispose OBS client before audio driver so its Drop impl runs while
        // other resources are still intact. ObsAccess has no explicit close()
        // method; dropping it disconnects the WebSocket.
        if let Some(obs) = self.ctx.integration.obs_client.take() {
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
        if let Some(ref mut audio) = self.ctx.audio {
            audio.dispose();
        }
        self.ctx.audio = None;

        info!("All resources disposed");
    }

    /// Pause lifecycle -- dispatches to current state.
    ///
    /// Translated from: MainController.pause()
    pub fn pause(&mut self) {
        if let Some(ref mut current) = self.current {
            current.pause();
        }
    }

    /// Resize lifecycle -- dispatches to current state.
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

    /// Resume lifecycle -- dispatches to current state.
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
        if let Err(e) = Config::write(&self.ctx.config) {
            log::error!("Failed to write config: {}", e);
        }
        if let Err(e) = PlayerConfig::write(&self.ctx.config.paths.playerpath, &self.ctx.player) {
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
    /// In Java, Gdx.app.exit() triggers the LibGDX lifecycle (pause -> dispose),
    /// and dispose() calls saveConfig(). In Rust, we set an exit flag and save
    /// config immediately, since the main loop checks is_exit_requested().
    pub fn exit(&self) {
        self.ctx.exit_requested.store(true, Ordering::Release);
        self.save_config();
        info!("Exit requested");
    }

    /// Check whether exit has been requested.
    ///
    /// The main event loop should poll this and initiate shutdown when true.
    pub fn is_exit_requested(&self) -> bool {
        self.ctx.exit_requested.load(Ordering::Acquire)
    }
}
