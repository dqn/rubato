// Queued proxy implementations for MainControllerAccess and AudioDriver.
// These proxies enqueue commands for later execution by MainController,
// allowing screen states to issue side effects without direct controller access.

use std::any::Any;
use std::collections::HashSet;
use std::sync::Arc;

use rubato_audio::audio_driver::AudioDriver;
use rubato_core::main_controller::MainController;
use rubato_core::play_data_accessor::PlayDataAccessor;
use rubato_core::system_sound_manager::SystemSoundManager;
use rubato_ir::ir_chart_data::IRChartData;
use rubato_ir::ir_connection::IRConnection;
use rubato_ir::ir_course_data::IRCourseData;
use rubato_ir::ranking_data_cache::RankingDataCache;
use rubato_types::main_controller_access::{
    MainControllerAccess, MainControllerCommand, MainControllerCommandQueue,
};
use rubato_types::player_information::PlayerInformation;
use rubato_types::player_resource_access::PlayerResourceAccess;
use rubato_types::score_data::ScoreData;
use rubato_types::sound_type::SoundType;

pub(super) struct QueuedControllerAccess {
    config: rubato_core::config::Config,
    player_config: rubato_core::player_config::PlayerConfig,
    commands: MainControllerCommandQueue,
    sound: SystemSoundManager,
    play_data_accessor: PlayDataAccessor,
    ranking_data_cache: Box<dyn rubato_types::ranking_data_cache_access::RankingDataCacheAccess>,
    ir_connection: Option<Arc<dyn IRConnection + Send + Sync>>,
    rivals: Vec<PlayerInformation>,
    ipfs_download_alive: bool,
    http_downloader: Option<Arc<dyn rubato_types::http_download_submitter::HttpDownloadSubmitter>>,
    active_audio_paths: HashSet<String>,
}

fn ensure_controller_ranking_cache(controller: &mut MainController) {
    if controller.ranking_data_cache().is_none() {
        controller.set_ranking_data_cache(Box::new(RankingDataCache::new()));
    }
}

impl QueuedControllerAccess {
    pub(super) fn from_controller(
        controller: &mut MainController,
        commands: MainControllerCommandQueue,
    ) -> Self {
        ensure_controller_ranking_cache(controller);
        let config = controller.config().clone();
        let player_config = controller.player_config().clone();
        let ir_connection = controller.ir_connection_any().and_then(|any| {
            any.downcast_ref::<Arc<dyn IRConnection + Send + Sync>>()
                .cloned()
        });
        let rivals = (0..controller.rival_count())
            .filter_map(|i| controller.rival_information(i))
            .collect();
        let ranking_data_cache = controller
            .ranking_data_cache()
            .map(|cache| cache.clone_box())
            .unwrap_or_else(|| Box::new(RankingDataCache::new()));

        Self {
            sound: SystemSoundManager::new(
                Some(config.paths.bgmpath.as_str()),
                Some(config.paths.soundpath.as_str()),
            ),
            play_data_accessor: PlayDataAccessor::new(&config),
            ranking_data_cache,
            ipfs_download_alive: controller.is_ipfs_download_alive(),
            http_downloader: controller.clone_http_download_processor(),
            config,
            player_config,
            commands,
            ir_connection,
            rivals,
            active_audio_paths: HashSet::new(),
        }
    }
}

impl MainControllerAccess for QueuedControllerAccess {
    fn config(&self) -> &rubato_types::config::Config {
        &self.config
    }

    fn player_config(&self) -> &rubato_types::player_config::PlayerConfig {
        &self.player_config
    }

    fn change_state(&mut self, state: rubato_core::main_state::MainStateType) {
        self.commands
            .push(MainControllerCommand::ChangeState(state));
    }

    fn save_config(&self) {
        self.commands.push(MainControllerCommand::SaveConfig);
    }

    fn exit(&self) {
        self.commands.push(MainControllerCommand::Exit);
    }

    fn save_last_recording(&self, reason: &str) {
        self.commands
            .push(MainControllerCommand::SaveLastRecording(reason.to_string()));
    }

    fn update_song(&mut self, path: Option<&str>) {
        self.commands
            .push(MainControllerCommand::UpdateSong(path.map(str::to_string)));
    }

    fn player_resource(&self) -> Option<&dyn PlayerResourceAccess> {
        None
    }

    fn player_resource_mut(&mut self) -> Option<&mut dyn PlayerResourceAccess> {
        None
    }

    fn play_sound(&mut self, sound: &SoundType, loop_sound: bool) {
        self.commands
            .push(MainControllerCommand::PlaySound(*sound, loop_sound));
    }

    fn stop_sound(&mut self, sound: &SoundType) {
        self.commands.push(MainControllerCommand::StopSound(*sound));
    }

    fn sound_path(&self, sound: &SoundType) -> Option<String> {
        self.sound.sound(sound).cloned()
    }

    fn play_audio_path(&mut self, path: &str, volume: f32, loop_play: bool) {
        if path.is_empty() {
            return;
        }
        self.active_audio_paths.insert(path.to_string());
        self.commands.push(MainControllerCommand::PlayAudioPath(
            path.to_string(),
            volume,
            loop_play,
        ));
    }

    fn set_audio_path_volume(&mut self, path: &str, volume: f32) {
        if path.is_empty() {
            return;
        }
        self.commands
            .push(MainControllerCommand::SetAudioPathVolume(
                path.to_string(),
                volume,
            ));
    }

    fn is_audio_path_playing(&self, path: &str) -> bool {
        self.active_audio_paths.contains(path)
    }

    fn stop_audio_path(&mut self, path: &str) {
        if path.is_empty() {
            return;
        }
        self.active_audio_paths.remove(path);
        self.commands
            .push(MainControllerCommand::StopAudioPath(path.to_string()));
    }

    fn dispose_audio_path(&mut self, path: &str) {
        if path.is_empty() {
            return;
        }
        self.active_audio_paths.remove(path);
        self.commands
            .push(MainControllerCommand::DisposeAudioPath(path.to_string()));
    }

    fn shuffle_sounds(&mut self) {
        self.sound.shuffle();
        self.commands.push(MainControllerCommand::ShuffleSounds);
    }

    fn read_replay_data(
        &self,
        sha256: &str,
        has_ln: bool,
        lnmode: i32,
        index: i32,
    ) -> Option<rubato_types::replay_data::ReplayData> {
        self.play_data_accessor
            .read_replay_data(sha256, has_ln, lnmode, index)
    }

    fn ir_song_url(&self, song_data: &rubato_types::song_data::SongData) -> Option<String> {
        self.ir_connection
            .as_ref()
            .and_then(|conn| conn.get_song_url(&IRChartData::new(song_data)))
    }

    fn ir_course_url(&self, course_data: &rubato_types::course_data::CourseData) -> Option<String> {
        self.ir_connection.as_ref().and_then(|conn| {
            conn.get_course_url(&IRCourseData::new_with_lntype(
                course_data,
                self.player_config.play_settings.lnmode,
            ))
        })
    }

    fn update_table(
        &mut self,
        source: Box<dyn rubato_types::table_update_source::TableUpdateSource>,
    ) {
        self.commands
            .push(MainControllerCommand::UpdateTable(source));
    }

    fn http_downloader(
        &self,
    ) -> Option<&dyn rubato_types::http_download_submitter::HttpDownloadSubmitter> {
        self.http_downloader
            .as_ref()
            .map(|downloader| downloader.as_ref())
    }

    fn is_ipfs_download_alive(&self) -> bool {
        self.ipfs_download_alive
    }

    fn start_ipfs_download(&mut self, song: &rubato_types::song_data::SongData) -> bool {
        if !self.ipfs_download_alive {
            return false;
        }
        self.commands
            .push(MainControllerCommand::StartIpfsDownload(Box::new(
                song.clone(),
            )));
        true
    }

    fn ranking_data_cache(
        &self,
    ) -> Option<&dyn rubato_types::ranking_data_cache_access::RankingDataCacheAccess> {
        Some(&*self.ranking_data_cache)
    }

    fn ranking_data_cache_mut(
        &mut self,
    ) -> Option<&mut (dyn rubato_types::ranking_data_cache_access::RankingDataCacheAccess + 'static)>
    {
        Some(&mut *self.ranking_data_cache)
    }

    fn rival_count(&self) -> usize {
        self.rivals.len()
    }

    fn rival_information(&self, index: usize) -> Option<PlayerInformation> {
        self.rivals.get(index).cloned()
    }

    fn read_score_data_by_hash(&self, hash: &str, ln: bool, lnmode: i32) -> Option<ScoreData> {
        self.play_data_accessor
            .read_score_data_by_hash(hash, ln, lnmode)
    }

    fn read_player_data(&self) -> Option<rubato_types::player_data::PlayerData> {
        self.play_data_accessor.read_player_data()
    }

    fn ir_connection_any(&self) -> Option<&dyn Any> {
        self.ir_connection.as_ref().map(|conn| conn as &dyn Any)
    }
}

pub fn new_state_main_controller_access(
    controller: &mut MainController,
) -> Box<dyn MainControllerAccess + Send> {
    Box::new(QueuedControllerAccess::from_controller(
        controller,
        controller.controller_command_queue(),
    ))
}

pub(super) struct QueuedAudioDriver {
    commands: MainControllerCommandQueue,
    global_pitch: f32,
}

impl QueuedAudioDriver {
    pub(super) fn new(commands: MainControllerCommandQueue) -> Self {
        Self {
            commands,
            global_pitch: 1.0,
        }
    }
}

impl AudioDriver for QueuedAudioDriver {
    fn play_path(&mut self, _path: &str, _volume: f32, _loop_play: bool) {}
    fn set_volume_path(&mut self, _path: &str, _volume: f32) {}
    fn is_playing_path(&self, _path: &str) -> bool {
        false
    }
    fn stop_path(&mut self, _path: &str) {}
    fn dispose_path(&mut self, _path: &str) {}
    fn set_model(&mut self, _model: &bms_model::bms_model::BMSModel) {}
    fn set_additional_key_sound(&mut self, _judge: i32, _fast: bool, _path: Option<&str>) {}
    fn abort(&mut self) {}
    fn get_progress(&self) -> f32 {
        1.0
    }
    fn play_note(&mut self, _n: &bms_model::note::Note, _volume: f32, _pitch: i32) {}
    fn play_judge(&mut self, _judge: i32, _fast: bool) {}
    fn stop_note(&mut self, _n: Option<&bms_model::note::Note>) {
        self.commands.push(MainControllerCommand::StopAllNotes);
    }
    fn set_volume_note(&mut self, _n: &bms_model::note::Note, _volume: f32) {}
    fn set_global_pitch(&mut self, pitch: f32) {
        self.global_pitch = pitch;
        self.commands
            .push(MainControllerCommand::SetGlobalPitch(pitch));
    }
    fn get_global_pitch(&self) -> f32 {
        self.global_pitch
    }
    fn dispose_old(&mut self) {}
    fn dispose(&mut self) {}
}
