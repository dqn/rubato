use super::*;

impl MainControllerAccess for MainController {
    fn config(&self) -> &Config {
        &self.config
    }

    fn player_config(&self) -> &PlayerConfig {
        &self.player
    }

    fn change_state(&mut self, state: MainStateType) {
        MainController::change_state(self, state);
    }

    fn save_config(&self) -> anyhow::Result<()> {
        MainController::save_config(self);
        Ok(())
    }

    fn exit(&self) -> anyhow::Result<()> {
        MainController::exit(self);
        Ok(())
    }

    fn save_last_recording(&self, reason: &str) {
        MainController::save_last_recording(self, reason);
    }

    fn update_song(&mut self, path: Option<&str>) {
        if let Some(p) = path {
            MainController::update_song(self, p);
        }
    }

    fn player_resource(&self) -> Option<&dyn PlayerResourceAccess> {
        self.resource
            .as_ref()
            .map(|r| r as &dyn PlayerResourceAccess)
    }

    fn player_resource_mut(&mut self) -> Option<&mut dyn PlayerResourceAccess> {
        self.resource
            .as_mut()
            .map(|r| r as &mut dyn PlayerResourceAccess)
    }

    fn play_sound(&mut self, sound: &SoundType, loop_sound: bool) {
        let volume = self.config.audio.as_ref().map_or(1.0, |a| a.systemvolume);
        let path = self.sound.as_ref().and_then(|sm| sm.sound(sound).cloned());
        if let Some(path) = path
            && let Some(ref mut audio) = self.audio
        {
            audio.play_path(&path, volume, loop_sound);
        }
    }

    fn stop_sound(&mut self, sound: &SoundType) {
        let path = self.sound.as_ref().and_then(|sm| sm.sound(sound).cloned());
        if let Some(path) = path
            && let Some(ref mut audio) = self.audio
        {
            audio.stop_path(&path);
        }
    }

    fn sound_path(&self, sound: &SoundType) -> Option<String> {
        self.sound.as_ref().and_then(|sm| sm.sound(sound).cloned())
    }

    fn play_audio_path(&mut self, path: &str, volume: f32, loop_play: bool) {
        if path.is_empty() {
            return;
        }
        if let Some(ref mut audio) = self.audio {
            audio.play_path(path, volume, loop_play);
        }
    }

    fn set_audio_path_volume(&mut self, path: &str, volume: f32) {
        if path.is_empty() {
            return;
        }
        if let Some(ref mut audio) = self.audio {
            audio.set_volume_path(path, volume);
        }
    }

    fn is_audio_path_playing(&self, path: &str) -> bool {
        if path.is_empty() {
            return false;
        }
        self.audio
            .as_ref()
            .is_some_and(|audio| audio.is_playing_path(path))
    }

    fn stop_audio_path(&mut self, path: &str) {
        if path.is_empty() {
            return;
        }
        if let Some(ref mut audio) = self.audio {
            audio.stop_path(path);
        }
    }

    fn dispose_audio_path(&mut self, path: &str) {
        if path.is_empty() {
            return;
        }
        if let Some(ref mut audio) = self.audio {
            audio.dispose_path(path);
        }
    }

    fn shuffle_sounds(&mut self) {
        if let Some(ref mut sm) = self.sound {
            let old_paths = sm.shuffle();
            if let Some(ref mut audio) = self.audio {
                for path in &old_paths {
                    audio.dispose_path(path);
                }
            }
        }

        // Preload all system sound paths into the audio driver's path sound cache
        // to avoid blocking file I/O on the render thread when play_path() is
        // first called for each sound.
        if let Some(ref sm) = self.sound {
            let paths: Vec<String> = SoundType::values()
                .iter()
                .filter_map(|st| sm.sound(st).cloned())
                .collect();
            if let Some(ref mut audio) = self.audio {
                for path in &paths {
                    audio.preload_path(path);
                }
            }
        }
    }

    fn exists_replay_data(&self, sha256: &str, has_ln: bool, lnmode: i32, index: i32) -> bool {
        self.db
            .playdata
            .as_ref()
            .is_some_and(|pda| pda.exists_replay_data(sha256, has_ln, lnmode, index))
    }

    fn read_replay_data(
        &self,
        sha256: &str,
        has_ln: bool,
        lnmode: i32,
        index: i32,
    ) -> Option<rubato_types::replay_data::ReplayData> {
        self.db
            .playdata
            .as_ref()
            .and_then(|pda| pda.read_replay_data(sha256, has_ln, lnmode, index))
    }

    fn update_table(
        &mut self,
        source: Box<dyn rubato_types::table_update_source::TableUpdateSource>,
    ) {
        MainController::update_table(self, source);
    }

    fn ranking_data_cache(
        &self,
    ) -> Option<&dyn rubato_types::ranking_data_cache_access::RankingDataCacheAccess> {
        MainController::ranking_data_cache(self)
    }

    fn ranking_data_cache_mut(
        &mut self,
    ) -> Option<&mut (dyn rubato_types::ranking_data_cache_access::RankingDataCacheAccess + 'static)>
    {
        self.db.ircache.as_deref_mut()
    }

    fn http_downloader(
        &self,
    ) -> Option<&dyn rubato_types::http_download_submitter::HttpDownloadSubmitter> {
        self.integration
            .http_download_processor
            .as_ref()
            .map(|processor| processor.as_ref())
    }

    fn is_ipfs_download_alive(&self) -> bool {
        self.integration
            .download
            .as_ref()
            .is_some_and(|dl| dl.is_alive())
    }

    fn start_ipfs_download(&mut self, song: &rubato_types::song_data::SongData) -> bool {
        if let Some(ref dl) = self.integration.download {
            dl.start_download(song);
            true
        } else {
            false
        }
    }

    fn rival_count(&self) -> usize {
        self.db.rivals.rival_count()
    }

    fn rival_information(
        &self,
        index: usize,
    ) -> Option<rubato_types::player_information::PlayerInformation> {
        self.db.rivals.rival_information(index).cloned()
    }

    fn read_score_data_by_hash(
        &self,
        hash: &str,
        ln: bool,
        lnmode: i32,
    ) -> Option<rubato_types::score_data::ScoreData> {
        self.db
            .playdata
            .as_ref()
            .and_then(|pda| pda.read_score_data_by_hash(hash, ln, lnmode))
    }

    fn read_player_data(&self) -> Option<rubato_types::player_data::PlayerData> {
        self.db
            .playdata
            .as_ref()
            .and_then(|pda| pda.read_player_data())
    }

    fn info_database(&self) -> Option<&dyn rubato_types::song_information_db::SongInformationDb> {
        self.db.infodb.as_deref()
    }

    fn ir_connection_any(&self) -> Option<&dyn std::any::Any> {
        self.db
            .ir
            .first()
            .and_then(|status| status.connection.as_ref())
            .map(|conn| conn.as_ref() as &dyn std::any::Any)
    }

    fn load_new_profile(&self, pc: PlayerConfig) {
        self.command_queue.push(
            rubato_types::main_controller_access::MainControllerCommand::LoadNewProfile(Box::new(
                pc,
            )),
        );
    }

    fn offset_value(&self, id: i32) -> Option<&rubato_types::skin_offset::SkinOffset> {
        self.offset(id)
    }
}
