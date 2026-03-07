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
    /// Java lines 345-358:
    /// ```java
    /// private void changeState(MainState newState) {
    ///     newState.create();
    ///     if(newState.getSkin() != null) { newState.getSkin().prepare(newState); }
    ///     if(current != null) { current.shutdown(); current.setSkin(null); }
    ///     current = newState;
    ///     timer.setMainState(newState);
    ///     current.prepare();
    ///     updateMainStateListener(0);
    /// }
    /// ```
    fn transition_to_state(&mut self, mut new_state: Box<dyn MainState>) {
        // Create the new state
        new_state.create();

        // In Java: if(newState.getSkin() != null) { newState.getSkin().prepare(newState); }
        if let Some(ref mut skin) = new_state.main_state_data_mut().skin {
            skin.prepare_skin();
        }

        // Shutdown the old state
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
            old_state.shutdown();
            if let Some(ref mut audio) = self.audio {
                old_state.sync_audio(audio.as_mut());
            }
            // setSkin(null) equivalent
            old_state.main_state_data_mut().skin = None;
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
            }
        }

        if let Some(state) = pending_change {
            self.change_state(state);
        }
    }
}
