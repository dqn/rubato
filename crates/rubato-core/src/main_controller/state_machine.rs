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
        self.emit_state_event(rubato_types::state_event::StateEvent::TransitionStart {
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
        let actual_type = if state == MainStateType::Decide && self.config.select.skip_decide_screen
        {
            MainStateType::Play
        } else {
            state
        };

        // Check if we're already in this state type
        if let Some(ref current) = self.current
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
            && let Some(any_box) = current.take_player_resource_box()
            && let Ok(core_resource) = any_box.downcast::<PlayerResource>()
        {
            self.resource = Some(*core_resource);
        }

        // Create the new state via factory.
        // Take the factory out temporarily to avoid borrow conflict
        // (factory is borrowed immutably, but create_state needs &mut self).
        let factory = self.state_factory.take().unwrap_or_else(|| {
            panic!(
                "No state factory set; cannot create state {:?}. \
                 Caller must call set_state_factory() before any state transitions.",
                actual_type
            );
        });
        let result = factory.create_state(actual_type, self);
        self.state_factory = Some(factory);

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
    fn transition_to_state(&mut self, mut new_state: Box<dyn MainState>) {
        // Shutdown the old state BEFORE creating the new one (matching Java order).
        // This frees GPU resources (textures, skins) and flushes audio before the
        // new state loads its own resources, preventing resource contention.
        if let Some(ref mut old_state) = self.current {
            // Extract BGA processor cache before shutdown for reuse in subsequent plays.
            // Java: BGAProcessor lives in BMSResource and persists across state transitions.
            if let Some(bga_cache) = old_state.take_bga_cache()
                && let Some(ref mut resource) = self.resource
            {
                resource.set_bga_any(bga_cache);
            }
            // Restore PlayerResource from the exiting state back to MainController.
            // States receive ownership via take_player_resource() in the factory;
            // we reclaim it here so it's available for the next state.
            if let Some(any_box) = old_state.take_player_resource_box()
                && let Ok(core_resource) = any_box.downcast::<PlayerResource>()
            {
                self.resource = Some(*core_resource);
            }
            // Flush pending audio commands before shutdown so they operate on
            // live state rather than potentially disposed resources.
            if let Some(ref mut audio) = self.audio {
                old_state.sync_audio(audio.as_mut());
            }
            // Emit state shutdown event before shutdown.
            // Access state_event_log directly to avoid borrowing all of `self`
            // while `self.current` is mutably borrowed as `old_state`.
            if let Some(st) = old_state.state_type()
                && let Some(ref log) = self.state_event_log
                && let Ok(mut guard) = log.lock()
            {
                guard.push(rubato_types::state_event::StateEvent::StateShutdown { state: st });
            }
            old_state.shutdown();
            // Flush audio again after shutdown so tick-based processors (e.g.
            // PreviewMusicProcessor) can see the stop flag and actually halt playback.
            // In Java the preview thread exits its loop autonomously, but in Rust
            // preview runs via sync_audio ticks on the main thread.
            if let Some(ref mut audio) = self.audio {
                old_state.sync_audio(audio.as_mut());
            }
            // setSkin(null) equivalent -- Java's setSkin(null) calls skin.dispose() first
            if let Some(ref mut skin) = old_state.main_state_data_mut().skin {
                skin.dispose_skin();
            }
            old_state.main_state_data_mut().skin = None;
        }
        // Drop the old state now that it has been shut down
        self.current = None;

        // Create the new state
        new_state.create();

        // Emit state created event
        if let Some(st) = new_state.state_type() {
            self.emit_state_event(rubato_types::state_event::StateEvent::StateCreated {
                state: st,
            });
        }

        // Apply create side effects (input mode, guide SE)
        // Java: BMSPlayer.create() directly modifies input processor; in Rust the
        // side effects are queued and applied here since create() can't access
        // MainController's input processor.
        if let Some(effects) = new_state.take_state_create_effects() {
            if effects.disable_input {
                if let Some(ref mut input) = self.input {
                    input.set_enable(false);
                }
            } else if let Some(mode) = effects.play_config_mode
                && let Some(ref mut input) = self.input
            {
                input.set_enable(true);
                input.set_play_config(self.player.play_config(mode));
            }
            if let Some(ref mut audio) = self.audio {
                if effects.guide_se {
                    if let Some(ref sm) = self.sound {
                        use rubato_types::sound_type::SoundType;
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
            && let Some(ref mut audio) = self.audio
        {
            audio.set_model(model);
        }

        // In Java: if(newState.getSkin() != null) { newState.getSkin().prepare(newState); }
        if let Some(ref mut skin) = new_state.main_state_data_mut().skin {
            skin.prepare_skin();
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
            timer.set_timer_on(rubato_types::timer_id::TimerId(0));
        }

        // Prepare the new state
        if let Some(ref mut current) = self.current {
            current.prepare();
        }

        self.process_queued_controller_commands();

        // Emit transition complete event
        if let Some(ref current) = self.current
            && let Some(st) = current.state_type()
        {
            self.emit_state_event(rubato_types::state_event::StateEvent::TransitionComplete {
                state: st,
            });
        }

        self.update_main_state_listener(0);
    }

    pub(super) fn process_queued_controller_commands(&mut self) {
        use rubato_types::main_controller_access::MainControllerCommand;

        let mut pending_change: Option<MainStateType> = None;
        for command in self.command_queue.drain() {
            match command {
                MainControllerCommand::ChangeState(state) => pending_change = Some(state),
                MainControllerCommand::SaveConfig => self.save_config(),
                MainControllerCommand::Exit => self.exit(),
                MainControllerCommand::SaveLastRecording(reason) => {
                    self.save_last_recording(&reason);
                }
                MainControllerCommand::UpdateSong(Some(path)) => self.update_song(&path),
                MainControllerCommand::UpdateSong(None) => {}
                MainControllerCommand::PlaySound(sound, loop_sound) => {
                    <Self as MainControllerAccess>::play_sound(self, &sound, loop_sound);
                }
                MainControllerCommand::StopSound(sound) => {
                    <Self as MainControllerAccess>::stop_sound(self, &sound);
                }
                MainControllerCommand::ShuffleSounds => {
                    <Self as MainControllerAccess>::shuffle_sounds(self);
                }
                MainControllerCommand::UpdateTable(source) => {
                    self.update_table(source);
                }
                MainControllerCommand::StartIpfsDownload(song) => {
                    let _ = <Self as MainControllerAccess>::start_ipfs_download(self, &song);
                }
                MainControllerCommand::SetGlobalPitch(pitch) => {
                    if let Some(ref mut audio) = self.audio {
                        audio.set_global_pitch(pitch);
                    }
                }
                MainControllerCommand::StopAllNotes => {
                    if let Some(ref mut audio) = self.audio {
                        audio.stop_note(None);
                    }
                }
                MainControllerCommand::PlayAudioPath(path, volume, loop_play) => {
                    if let Some(ref mut audio) = self.audio {
                        audio.play_path(&path, volume, loop_play);
                    }
                }
                MainControllerCommand::SetAudioPathVolume(path, volume) => {
                    if let Some(ref mut audio) = self.audio {
                        audio.set_volume_path(&path, volume);
                    }
                }
                MainControllerCommand::StopAudioPath(path) => {
                    if let Some(ref mut audio) = self.audio {
                        audio.stop_path(&path);
                    }
                }
                MainControllerCommand::DisposeAudioPath(path) => {
                    if let Some(ref mut audio) = self.audio {
                        audio.dispose_path(&path);
                    }
                }
                MainControllerCommand::LoadNewProfile(pc) => {
                    self.load_new_profile(*pc);
                }
                MainControllerCommand::UpdatePlayConfig(mode, play_config) => {
                    let pc = *play_config;
                    self.player.play_config(mode).playconfig = pc.clone();
                    if let Some(ref mut state) = self.current {
                        state.receive_updated_play_config(mode, pc);
                    }
                }
                MainControllerCommand::UpdateAudioConfig(audio) => {
                    self.config.audio = Some(audio);
                }
            }
        }

        if let Some(state) = pending_change {
            self.change_state(state);
        }
    }
}
