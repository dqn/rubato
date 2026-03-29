use super::*;

impl MainController {
    /// Change to the specified state type.
    ///
    /// Translated from: MainController.changeState(MainStateType)
    ///
    /// Java lines 305-343:
    /// ```java
    /// public void changeState(MainStateType state) {
    ///     MainState newState = null;
    ///     switch (state) {
    ///     case MUSICSELECT:
    ///         if (this.bmsfile != null) { exit(); } else { newState = selector; }
    ///         break;
    ///     case DECIDE: newState = config.isSkipDecideScreen() ? createBMSPlayerState() : decide; break;
    ///     case PLAY: newState = createBMSPlayerState(); break;
    ///     case RESULT: newState = result; break;
    ///     case COURSERESULT: newState = gresult; break;
    ///     case CONFIG: newState = keyconfig; break;
    ///     case SKINCONFIG: newState = skinconfig; break;
    ///     }
    ///     if (newState != null && current != newState) { changeState(newState); }
    /// }
    /// ```
    pub fn change_state(&mut self, state: MainStateType) {
        // Emit transition start event
        let from_state = self.current_state_type();
        self.emit_state_event(crate::skin::state_event::StateEvent::TransitionStart {
            from: from_state,
            to: state,
        });

        // Determine whether to create a new state
        let should_create = match state {
            MainStateType::MusicSelect => {
                if self.bmsfile.is_some() {
                    self.exit();
                    false
                } else {
                    true
                }
            }
            MainStateType::Decide => {
                // In Java: config.isSkipDecideScreen() ? createBMSPlayerState() : decide
                // When skip is true, create a Play state instead
                true
            }
            MainStateType::Play => true,
            MainStateType::Result => true,
            MainStateType::CourseResult => true,
            MainStateType::Config => true,
            MainStateType::SkinConfig => true,
        };

        if !should_create {
            return;
        }

        // Determine the actual state type to create
        // (for Decide with skip, we create Play instead)
        let actual_type =
            if state == MainStateType::Decide && self.ctx.config.select.skip_decide_screen {
                MainStateType::Play
            } else {
                state
            };

        // Check if we're already in this state type.
        // Allow Play->Play transitions for quick retry (creates a fresh Play state).
        if actual_type != MainStateType::Play
            && let Some(ref current) = self.current
            && current.state_type() == Some(actual_type)
        {
            return;
        }

        // Restore PlayerResource from the current state to the controller
        // BEFORE creating the new state, so the factory can access it.
        // Previously NullPlayerResource masked this sequencing issue: states
        // like Decide that take the resource from the controller would
        // silently get a null stub because the resource was still inside the
        // old state and only restored during transition_to_state (too late).
        if let Some(ref mut current) = self.current
            && let Some(resource) = current.take_player_resource()
        {
            self.resource = Some(resource);
        }

        // Create the new state.
        // If a custom factory has been set (test mocks), use it.
        // Otherwise, use the built-in creation logic.
        let result = if let Some(factory) = self.state_factory.take() {
            // Test override path: use custom factory.
            // Take the factory out temporarily to avoid borrow conflict
            // (factory closure captures nothing from self, but the call needs &mut self).
            // Restore the factory before resuming any panic so that subsequent
            // state transitions don't double-panic on a missing factory.
            let r = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                factory(actual_type, self)
            })) {
                Ok(result) => result,
                Err(payload) => {
                    self.state_factory = Some(factory);
                    std::panic::resume_unwind(payload);
                }
            };
            self.state_factory = Some(factory);
            r
        } else {
            // Default path: create state directly without factory indirection.
            self.create_state_for_type(actual_type)
        };

        if let Some(result) = result {
            // Apply target score to PlayerResource so the result screen can read it.
            // Java: resource.setTargetScoreData(targetScore)
            if let Some(target) = result.target_score
                && let Some(ref mut resource) = self.resource
            {
                resource.set_target_score_data(target);
            }
            self.transition_to_state(result.state);
        }

        // In Java: input processor setup based on current.getStage()
        // Phase 5+: Gdx.input.setInputProcessor(...)
    }

    /// Internal state transition: shutdown old state, create and prepare new state.
    ///
    /// Translated from: MainController.changeState(MainState) (private overload)
    ///
    /// Java lines 276-289:
    /// ```java
    /// if(current != null) {
    ///     current.shutdown();
    ///     current.setSkin(null);
    /// }
    /// newState.create();
    /// if(newState.getSkin() != null) { newState.getSkin().prepare(newState); }
    /// current = newState;
    /// timer.setMainState(newState);
    /// current.prepare();
    /// updateMainStateListener(0);
    /// ```
    fn transition_to_state(&mut self, mut new_state: crate::game_screen::GameScreen) {
        // Prune finished background threads before the transition so their Arc
        // references to shared resources (DB handles, IR caches, etc.) are released
        // before the old state shuts down and the new state is created.
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

        // Shutdown the old state BEFORE creating the new one (matching Java order).
        // This frees GPU resources (textures, skins) and flushes audio before the
        // new state loads its own resources, preventing resource contention.
        if let Some(ref mut old_state) = self.current {
            // Extract BGA processor cache before shutdown for reuse in subsequent plays.
            // Java: BGAProcessor lives in BMSResource and persists across state transitions.
            if let Some(bga_cache) = old_state.take_bga_cache()
                && let Some(ref mut resource) = self.resource
            {
                resource.set_bga(bga_cache);
            }
            // Restore PlayerResource from the exiting state back to MainController.
            // States receive ownership via take_player_resource() in the factory;
            // we reclaim it here so it's available for the next state.
            if let Some(resource) = old_state.take_player_resource() {
                self.resource = Some(resource);
            }
            // Flush pending audio commands before shutdown so they operate on
            // live state rather than potentially disposed resources.
            if let Some(ref mut audio) = self.ctx.audio {
                old_state.sync_audio(audio);
            }
            // Emit state shutdown event before shutdown.
            // Access state_event_log and event_senders directly to avoid
            // borrowing all of `self` while `self.current` is mutably
            // borrowed as `old_state`.
            if let Some(st) = old_state.state_type() {
                let event = crate::skin::state_event::StateEvent::StateShutdown { state: st };
                if let Some(ref log) = self.state_event_log
                    && let Ok(mut guard) = log.lock()
                {
                    guard.push(event.clone());
                }
                let app_event = crate::skin::app_event::AppEvent::Lifecycle(event);
                for sender in &self.event_senders {
                    let _ = sender.try_send(app_event.clone());
                }
            }
            old_state.shutdown();
            // Flush audio again after shutdown so tick-based processors (e.g.
            // PreviewMusicProcessor) can see the stop flag and actually halt playback.
            // In Java the preview thread exits its loop autonomously, but in Rust
            // preview runs via sync_audio ticks on the main thread.
            if let Some(ref mut audio) = self.ctx.audio {
                old_state.sync_audio(audio);
            }
            // Cache the decide skin for reuse instead of disposing it.
            // The decide skin is expensive to load (3+ seconds for complex skins)
            // but doesn't change between songs, so we keep it alive.
            if old_state.state_type() == Some(MainStateType::Decide) {
                // Dispose any previously cached skin before replacing
                if let Some(ref mut prev_cached) = self.decide_skin_cache {
                    prev_cached.dispose_skin();
                }
                self.decide_skin_cache = old_state.main_state_data_mut().skin.take();
            }
            // setSkin(null) equivalent -- Java's setSkin(null) calls skin.dispose() first
            if let Some(ref mut skin) = old_state.main_state_data_mut().skin {
                skin.dispose_skin();
            }
            old_state.main_state_data_mut().skin = None;
        }
        // Drop the old state now that it has been shut down
        self.current = None;

        // Invalidate decide skin cache when entering skin/key config screens
        // (user may change the decide skin path).
        if matches!(
            new_state.state_type(),
            Some(MainStateType::Config) | Some(MainStateType::SkinConfig)
        ) {
            if let Some(ref mut cached) = self.decide_skin_cache {
                cached.dispose_skin();
            }
            self.decide_skin_cache = None;
        }

        // Inject cached decide skin before create() to skip expensive reload.
        // The cached skin is already prepared from its first use; skin objects
        // read dynamic data (song title, score, etc.) from MainState at render time.
        let decide_skin_cached = if new_state.state_type() == Some(MainStateType::Decide) {
            if let Some(cached_skin) = self.decide_skin_cache.take() {
                new_state.main_state_data_mut().skin = Some(cached_skin);
                true
            } else {
                false
            }
        } else {
            false
        };

        // Create the new state
        new_state.create();

        // Emit state created event
        if let Some(st) = new_state.state_type() {
            self.emit_state_event(crate::skin::state_event::StateEvent::StateCreated { state: st });
        }

        // Apply create side effects (input mode, guide SE)
        // Java: BMSPlayer.create() directly modifies input processor; in Rust the
        // side effects are queued and applied here since create() can't access
        // MainController's input processor.
        if let Some(effects) = new_state.take_state_create_effects() {
            if effects.disable_input {
                if let Some(ref mut input) = self.ctx.input {
                    input.set_enable(false);
                }
            } else if let Some(mode) = effects.play_config_mode
                && let Some(ref mut input) = self.ctx.input
            {
                input.set_enable(true);
                input.set_play_config(self.ctx.player.play_config(mode));
            }
            if let Some(ref mut audio) = self.ctx.audio {
                if effects.guide_se {
                    if let Some(ref sm) = self.ctx.sound {
                        use crate::skin::sound_type::SoundType;
                        let guide_se_types = [
                            SoundType::GuidesePg,
                            SoundType::GuideseGr,
                            SoundType::GuideseGd,
                            SoundType::GuideseBd,
                            SoundType::GuidesePr,
                            SoundType::GuideseMs,
                        ];
                        for (judge, sound_type) in guide_se_types.iter().enumerate() {
                            let paths = sm.sound_paths(sound_type);
                            let path = paths.first().map(|p| p.to_string_lossy().to_string());
                            audio.set_additional_key_sound(judge as i32, true, path.as_deref());
                            audio.set_additional_key_sound(judge as i32, false, path.as_deref());
                        }
                    }
                } else {
                    // Clear any previously set guide SE sounds.
                    for judge in 0..6 {
                        audio.set_additional_key_sound(judge, true, None);
                        audio.set_additional_key_sound(judge, false, None);
                    }
                }
            }
        }

        // Load keysounds from the BMS model into the audio driver.
        // Java: audio.setModel(model) is called during resource loading in BMSPlayer;
        // in Rust the audio driver is owned by MainController, so we call it here
        // after create() has set up the model.
        if let Some(model) = new_state.bms_model()
            && let Some(ref mut audio) = self.ctx.audio
        {
            audio.set_model(model);
        }

        // Register BMS resource images (stagefile=100, backbmp=101, banner=102) into
        // the skin's image registry so SkinSourceReference-backed objects can render them.
        // In Java, MainState.getImage(id) reads from BMSResource directly at draw time;
        // in Rust, we populate the skin's registry once here.
        if let Some(ref resource) = self.resource
            && let Some(bms_res) = resource.bms_resource()
        {
            let msd = new_state.main_state_data_mut();
            if let Some(ref mut skin) = msd.skin {
                if let Some(tr) = bms_res.stagefile() {
                    skin.register_image(crate::core::bms_resource::IMAGE_STAGEFILE, tr.clone());
                }
                if let Some(tr) = bms_res.backbmp() {
                    skin.register_image(crate::core::bms_resource::IMAGE_BACKBMP, tr.clone());
                }
                if let Some(tr) = bms_res.banner() {
                    skin.register_image(crate::core::bms_resource::IMAGE_BANNER, tr.clone());
                }
            }
        }

        // Copy skin config offsets into both MainStateData.offsets (HashMap, for render contexts)
        // and MainController.offset[] (Vec, for trait delegation).
        // Java: MainState.setSkin() copies skin.getOffset() entries into main.offset[].
        // This must happen BEFORE skin.prepare() because skin objects read offsets during prepare.
        {
            let msd = new_state.main_state_data_mut();
            if let Some(ref skin) = msd.skin {
                msd.offsets = skin.skin_offsets();
            }
        }
        if let Some(ref skin) = new_state.main_state_data().skin {
            for (id, offset) in skin.offset_entries() {
                if let Some(rt) = self.offset_mut(id) {
                    *rt = offset;
                }
            }
        }

        // Propagate boot-relative time to the timer so skin properties (IDs 27-29)
        // can display hours/minutes/seconds since application start.
        // Java: main.getPlayTime() returns System.currentTimeMillis() - boottime.
        new_state
            .main_state_data_mut()
            .timer
            .set_boot_time_millis(self.play_time());

        // In Java: if(newState.getSkin() != null) { newState.getSkin().prepare(newState); }
        // Skip prepare for cached decide skins -- already prepared from first use.
        // prepare() is destructive (removes option-gated objects, clears option map),
        // so calling it again would incorrectly remove all option-gated objects.
        let st = new_state.state_type();
        if !decide_skin_cached && let Some(ref mut skin) = new_state.main_state_data_mut().skin {
            skin.prepare_skin(st);
        }

        // Set as current
        self.current = Some(new_state);

        // In Java: timer.setMainState(newState)
        // Java's setMainState resets all timers, restarts the clock, and turns on
        // timer 0 (TIMER_UNDEFINED) so skin objects referencing it will draw.
        if let Some(ref mut current) = self.current {
            let st = current.state_type();
            let timer = &mut current.main_state_data_mut().timer;
            timer.set_main_state();
            timer.state_type = st;
            timer.set_timer_on(crate::skin::timer_id::TimerId(0));
        }

        // Prepare the new state
        if let Some(ref mut current) = self.current {
            current.prepare();
        }

        // Emit transition complete event
        if let Some(ref current) = self.current
            && let Some(st) = current.state_type()
        {
            self.emit_state_event(crate::skin::state_event::StateEvent::TransitionComplete {
                state: st,
            });
        }

        self.update_main_state_listener(0);
    }
}
