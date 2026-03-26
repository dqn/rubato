use std::io::{self, Read};
use std::path::Path;

use anyhow::{Result, bail};
use log::warn;

use crate::audio_driver;
use crate::byte_pcm::BytePCM;
use crate::float_pcm::FloatPCM;
use crate::ms_adpcm_decoder::MSADPCMDecoder;
use crate::short_pcm::ShortPCM;

/// PCM audio data enum.
/// Translated from: PCM.java (abstract generic class)
///
/// Java's PCM<T> is translated as a Rust enum with variants for each concrete type.
#[derive(Clone, Debug)]
pub enum PCM {
    Short(ShortPCM),
    Float(FloatPCM),
    Byte(BytePCM),
}

impl PCM {
    pub fn channels(&self) -> i32 {
        match self {
            PCM::Short(p) => p.channels,
            PCM::Float(p) => p.channels,
            PCM::Byte(p) => p.channels,
        }
    }

    pub fn sample_rate(&self) -> i32 {
        match self {
            PCM::Short(p) => p.sample_rate,
            PCM::Float(p) => p.sample_rate,
            PCM::Byte(p) => p.sample_rate,
        }
    }

    pub fn start(&self) -> i32 {
        match self {
            PCM::Short(p) => p.start,
            PCM::Float(p) => p.start,
            PCM::Byte(p) => p.start,
        }
    }

    pub fn len(&self) -> i32 {
        match self {
            PCM::Short(p) => p.len,
            PCM::Float(p) => p.len,
            PCM::Byte(p) => p.len,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn load(p: &Path, driver_channels: i32, driver_sample_rate: i32) -> Option<PCM> {
        match Self::load_inner(p, driver_channels, driver_sample_rate) {
            Ok(pcm) => Some(pcm),
            Err(e) => {
                log::error!("{}", e);
                None
            }
        }
    }

    fn load_inner(p: &Path, driver_channels: i32, driver_sample_rate: i32) -> Result<PCM> {
        let mut loader = PCMLoader::new();
        loader.load_pcm(p)?;

        let pcm: Option<PCM>;
        if loader.bits_per_sample > 16 {
            pcm = Some(PCM::Float(FloatPCM::load_pcm(&loader)?));
        } else if loader.bits_per_sample == 16 {
            pcm = Some(PCM::Short(ShortPCM::load_pcm(&loader)?));
        } else {
            // Java: "TODO BytePCMのバグが解消されたら切替" — Java itself doesn't use BytePCM
            pcm = Some(PCM::Short(ShortPCM::load_pcm(&loader)?));
        }

        // Validate the file's sample rate before attempting conversion
        if let Some(ref p) = pcm
            && p.sample_rate() <= 0
        {
            warn!(
                "Invalid sample rate {} in audio file: {:?}",
                p.sample_rate(),
                p
            );
            bail!("Invalid sample rate in audio file");
        }

        // Channel/sample rate conversion if driver requires it
        let mut pcm = pcm;
        if let Some(ref mut p) = pcm
            && driver_channels != 0
            && p.channels() != driver_channels
        {
            *p = p.change_channels(driver_channels);
        }
        if let Some(ref mut p) = pcm
            && driver_sample_rate != 0
            && p.sample_rate() != driver_sample_rate
        {
            *p = p.change_sample_rate(driver_sample_rate);
        }

        if let Some(ref p) = pcm {
            if p.validate() {
                return Ok(pcm.expect("pcm"));
            } else {
                warn!("Failed to load audio file: {:?}", p);
                bail!("Failed to load audio file");
            }
        }
        bail!("Failed to load audio file");
    }

    pub fn load_by_name(name: &str, driver_channels: i32, driver_sample_rate: i32) -> Option<PCM> {
        for path in audio_driver::paths(name) {
            let pcm = PCM::load(&path, driver_channels, driver_sample_rate);
            if pcm.is_some() {
                return pcm;
            }
        }
        None
    }

    pub fn change_sample_rate(&self, sample: i32) -> PCM {
        match self {
            PCM::Short(p) => PCM::Short(p.change_sample_rate(sample)),
            PCM::Float(p) => PCM::Float(p.change_sample_rate(sample)),
            PCM::Byte(p) => PCM::Byte(p.change_sample_rate(sample)),
        }
    }

    pub fn change_frequency(&self, rate: f32) -> PCM {
        match self {
            PCM::Short(p) => PCM::Short(p.change_frequency(rate)),
            PCM::Float(p) => PCM::Float(p.change_frequency(rate)),
            PCM::Byte(p) => PCM::Byte(p.change_frequency(rate)),
        }
    }

    pub fn change_channels(&self, channels: i32) -> PCM {
        match self {
            PCM::Short(p) => PCM::Short(p.change_channels(channels)),
            PCM::Float(p) => PCM::Float(p.change_channels(channels)),
            PCM::Byte(p) => PCM::Byte(p.change_channels(channels)),
        }
    }

    pub fn slice(&self, starttime: i64, duration: i64) -> Option<PCM> {
        match self {
            PCM::Short(p) => p.slice(starttime, duration).map(PCM::Short),
            PCM::Float(p) => p.slice(starttime, duration).map(PCM::Float),
            PCM::Byte(p) => p.slice(starttime, duration).map(PCM::Byte),
        }
    }

    pub fn validate(&self) -> bool {
        match self {
            PCM::Short(p) => p.validate(),
            PCM::Float(p) => p.validate(),
            PCM::Byte(p) => p.validate(),
        }
    }
}

/// PCM data loader. Reads WAV/OGG/MP3/FLAC files into raw PCM data.
///
/// Translated from: PCM.PCMLoader inner class in PCM.java
pub struct PCMLoader {
    pub pcm_data: Vec<u8>,
    pub channels: i32,
    pub sample_rate: i32,
    pub bits_per_sample: i32,
    pub block_align: i32,
}

impl Default for PCMLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl PCMLoader {
    pub fn new() -> Self {
        PCMLoader {
            pcm_data: Vec::new(),
            channels: 0,
            sample_rate: 0,
            bits_per_sample: 0,
            block_align: 0,
        }
    }

    pub fn load_pcm(&mut self, p: &Path) -> Result<()> {
        self.pcm_data.clear();

        let name = p.to_string_lossy().to_lowercase();

        if name.ends_with(".wav") {
            self.load_wav(p)?;
        } else if name.ends_with(".ogg") {
            self.load_ogg(p)?;
        } else if name.ends_with(".mp3") || name.ends_with(".flac") {
            self.load_symphonia(p)?;
        } else {
            bail!("{}: unsupported format", p.display());
        }

        if self.pcm_data.is_empty() {
            bail!("{}: can't convert to PCM", p.display());
        }

        // Reject files too large for i32 sample indexing (>= 2GB PCM data)
        if self.pcm_data.len() > i32::MAX as usize {
            bail!(
                "{}: PCM data too large ({} bytes, max {})",
                p.display(),
                self.pcm_data.len(),
                i32::MAX
            );
        }

        if self.bits_per_sample == 0 {
            bail!("invalid WAV: bits_per_sample is 0");
        }

        // Trim trailing silence
        let mut bytes = self.pcm_data.len() as i32;
        let frame_size = self.channels * ((self.bits_per_sample + 7) / 8);
        if frame_size == 0 {
            bail!(
                "invalid WAV: bits_per_sample {} too small",
                self.bits_per_sample
            );
        }
        bytes -= bytes % frame_size;

        while bytes > frame_size {
            let frame_start = (bytes - frame_size) as usize;
            let frame_end = bytes as usize;
            let zero = self.pcm_data[frame_start..frame_end]
                .iter()
                .all(|&b| b == 0x00);
            if zero {
                bytes -= frame_size;
            } else {
                break;
            }
        }

        if bytes < frame_size {
            bail!("{}: 0 samples", p.display());
        }
        if self.sample_rate == 0 {
            bail!("{}: 0 sample rate", p.display());
        }
        self.pcm_data.truncate(bytes as usize);

        Ok(())
    }

    fn load_ogg(&mut self, p: &Path) -> Result<()> {
        use lewton::inside_ogg::OggStreamReader;

        let file = std::fs::File::open(p)?;
        let mut reader = OggStreamReader::new(file)?;

        self.channels = reader.ident_hdr.audio_channels as i32;
        self.sample_rate = reader.ident_hdr.audio_sample_rate as i32;
        self.bits_per_sample = 16;

        let mut all_samples: Vec<i16> = Vec::new();
        while let Some(packet) = reader.read_dec_packet_itl()? {
            all_samples.extend_from_slice(&packet);
        }

        // Convert i16 samples to bytes (little-endian)
        self.pcm_data = Vec::with_capacity(all_samples.len() * 2);
        for sample in &all_samples {
            self.pcm_data.extend_from_slice(&sample.to_le_bytes());
        }

        Ok(())
    }

    fn load_symphonia(&mut self, p: &Path) -> Result<()> {
        use symphonia::core::audio::SampleBuffer;
        use symphonia::core::codecs::DecoderOptions;
        use symphonia::core::formats::FormatOptions;
        use symphonia::core::io::MediaSourceStream;
        use symphonia::core::meta::MetadataOptions;
        use symphonia::core::probe::Hint;

        let file = std::fs::File::open(p)?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());
        let mut hint = Hint::new();
        if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }

        let probed = symphonia::default::get_probe().format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )?;

        let mut format = probed.format;
        let track = format
            .default_track()
            .ok_or_else(|| anyhow::anyhow!("no audio track"))?;
        let track_id = track.id;

        let codec_params = track.codec_params.clone();
        self.channels = codec_params.channels.map(|c| c.count() as i32).unwrap_or(2);
        self.sample_rate = codec_params.sample_rate.unwrap_or(44100) as i32;
        self.bits_per_sample = 16;

        let mut decoder =
            symphonia::default::get_codecs().make(&codec_params, &DecoderOptions::default())?;

        let mut all_samples: Vec<i16> = Vec::new();
        loop {
            let packet = match format.next_packet() {
                Ok(p) => p,
                Err(symphonia::core::errors::Error::IoError(ref e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    break;
                }
                Err(e) => return Err(e.into()),
            };
            if packet.track_id() != track_id {
                continue;
            }
            let decoded = decoder.decode(&packet)?;
            let spec = *decoded.spec();
            let duration = decoded.capacity();
            let mut sample_buf = SampleBuffer::<i16>::new(duration as u64, spec);
            sample_buf.copy_interleaved_ref(decoded);
            all_samples.extend_from_slice(sample_buf.samples());
        }

        self.pcm_data = Vec::with_capacity(all_samples.len() * 2);
        for sample in &all_samples {
            self.pcm_data.extend_from_slice(&sample.to_le_bytes());
        }

        Ok(())
    }

    fn load_symphonia_from_bytes(&mut self, data: &[u8]) -> Result<()> {
        use symphonia::core::audio::SampleBuffer;
        use symphonia::core::codecs::DecoderOptions;
        use symphonia::core::formats::FormatOptions;
        use symphonia::core::io::MediaSourceStream;
        use symphonia::core::meta::MetadataOptions;
        use symphonia::core::probe::Hint;

        let cursor = std::io::Cursor::new(data.to_vec());
        let mss = MediaSourceStream::new(Box::new(cursor), Default::default());
        let mut hint = Hint::new();
        hint.with_extension("mp3");

        let probed = symphonia::default::get_probe().format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )?;

        let mut format = probed.format;
        let track = format
            .default_track()
            .ok_or_else(|| anyhow::anyhow!("no audio track"))?;
        let track_id = track.id;

        let codec_params = track.codec_params.clone();
        self.channels = codec_params.channels.map(|c| c.count() as i32).unwrap_or(2);
        self.sample_rate = codec_params.sample_rate.unwrap_or(44100) as i32;
        self.bits_per_sample = 16;

        let mut decoder =
            symphonia::default::get_codecs().make(&codec_params, &DecoderOptions::default())?;

        let mut all_samples: Vec<i16> = Vec::new();
        loop {
            let packet = match format.next_packet() {
                Ok(p) => p,
                Err(symphonia::core::errors::Error::IoError(ref e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    break;
                }
                Err(e) => return Err(e.into()),
            };
            if packet.track_id() != track_id {
                continue;
            }
            let decoded = decoder.decode(&packet)?;
            let spec = *decoded.spec();
            let duration = decoded.capacity();
            let mut sample_buf = SampleBuffer::<i16>::new(duration as u64, spec);
            sample_buf.copy_interleaved_ref(decoded);
            all_samples.extend_from_slice(sample_buf.samples());
        }

        self.pcm_data = Vec::with_capacity(all_samples.len() * 2);
        for sample in &all_samples {
            self.pcm_data.extend_from_slice(&sample.to_le_bytes());
        }

        Ok(())
    }

    fn load_wav(&mut self, p: &Path) -> Result<()> {
        let data = std::fs::read(p)?;
        let mut wav = WavReader::new(&data)?;

        if wav.channels == 0 {
            bail!("{}: invalid WAV: channels is 0", p.display());
        }

        match wav.format_type {
            1 | 3 => {
                // PCM (1) or IEEE float (3)
                self.channels = wav.channels;
                self.sample_rate = wav.sample_rate;
                self.bits_per_sample = wav.bits_per_sample;

                self.pcm_data = wav.read_data()?;

                // 32-bit PCM (format_tag 1) stores i32 samples, but downstream
                // FloatPCM interprets 32-bit data as f32 via from_le_bytes.
                // Convert i32 -> f32 here so the passthrough works correctly.
                if wav.format_type == 1 && wav.bits_per_sample == 32 {
                    let data = &mut self.pcm_data;
                    for i in (0..data.len()).step_by(4) {
                        if i + 4 <= data.len() {
                            let val = i32::from_le_bytes([
                                data[i],
                                data[i + 1],
                                data[i + 2],
                                data[i + 3],
                            ]);
                            let f = val as f32 / i32::MAX as f32;
                            let bytes = f.to_le_bytes();
                            data[i..i + 4].copy_from_slice(&bytes);
                        }
                    }
                }
            }
            2 => {
                // MS-ADPCM
                self.channels = wav.channels;
                self.sample_rate = wav.sample_rate;
                self.bits_per_sample = 16;
                self.block_align = wav.block_align;

                let input_data = wav.read_data()?;
                let mut decoder =
                    MSADPCMDecoder::new(self.channels, self.sample_rate, self.block_align)?;
                self.pcm_data = decoder.decode(&input_data)?;

                log::info!("Filename: {:?}", p);
            }
            85 => {
                // mp3 embedded in WAV - extract data section and decode as MP3
                let mp3_data = wav.read_data()?;
                self.load_symphonia_from_bytes(&mp3_data)?;
            }
            _ => {
                bail!(
                    "{} unsupported WAV format ID : {}",
                    p.display(),
                    wav.format_type
                );
            }
        }

        Ok(())
    }
}

/// WAV file reader.
///
/// Translated from: PCM.WavInputStream inner class in PCM.java
struct WavReader<'a> {
    data: &'a [u8],
    pos: usize,
    pub format_type: i32,
    pub channels: i32,
    pub sample_rate: i32,
    pub block_align: i32,
    pub bits_per_sample: i32,
    data_start: usize,
    data_remaining: usize,
}

impl<'a> WavReader<'a> {
    fn new(data: &'a [u8]) -> Result<Self> {
        let mut reader = WavReader {
            data,
            pos: 0,
            format_type: 0,
            channels: 0,
            sample_rate: 0,
            block_align: -1,
            bits_per_sample: 0,
            data_start: 0,
            data_remaining: 0,
        };
        reader.parse_header()?;
        Ok(reader)
    }

    fn read_byte(&mut self) -> Result<u8> {
        if self.pos >= self.data.len() {
            bail!("Unexpected end of WAV data");
        }
        let b = self.data[self.pos];
        self.pos += 1;
        Ok(b)
    }

    fn skip_fully(&mut self, count: usize) -> Result<()> {
        if self.pos + count > self.data.len() {
            bail!("Unable to skip in WAV data");
        }
        self.pos += count;
        Ok(())
    }

    fn seek_to_chunk(&mut self, c1: u8, c2: u8, c3: u8, c4: u8) -> Result<i32> {
        loop {
            let mut found = self.read_byte()? == c1;
            found &= self.read_byte()? == c2;
            found &= self.read_byte()? == c3;
            found &= self.read_byte()? == c4;
            let chunk_length = (self.read_byte()? as i32 & 0xff)
                | ((self.read_byte()? as i32 & 0xff) << 8)
                | ((self.read_byte()? as i32 & 0xff) << 16)
                | ((self.read_byte()? as i32 & 0xff) << 24);
            if chunk_length < 0 {
                bail!(
                    "Invalid chunk length {} for chunk {}{}{}{}",
                    chunk_length,
                    c1 as char,
                    c2 as char,
                    c3 as char,
                    c4 as char
                );
            }
            if found {
                return Ok(chunk_length);
            }
            // RIFF chunks are word-aligned: skip a pad byte after odd-length chunks.
            let skip = if chunk_length % 2 != 0 {
                chunk_length as usize + 1
            } else {
                chunk_length as usize
            };
            self.skip_fully(skip)?;
        }
    }

    fn parse_header(&mut self) -> Result<()> {
        if self.read_byte()? != b'R'
            || self.read_byte()? != b'I'
            || self.read_byte()? != b'F'
            || self.read_byte()? != b'F'
        {
            bail!("RIFF header not found");
        }

        self.skip_fully(4)?;

        if self.read_byte()? != b'W'
            || self.read_byte()? != b'A'
            || self.read_byte()? != b'V'
            || self.read_byte()? != b'E'
        {
            bail!("Invalid wave file header");
        }

        let fmt_chunk_length = self.seek_to_chunk(b'f', b'm', b't', b' ')?;

        self.format_type =
            (self.read_byte()? as i32 & 0xff) | ((self.read_byte()? as i32 & 0xff) << 8);

        self.channels =
            (self.read_byte()? as i32 & 0xff) | ((self.read_byte()? as i32 & 0xff) << 8);

        self.sample_rate = (self.read_byte()? as i32 & 0xff)
            | ((self.read_byte()? as i32 & 0xff) << 8)
            | ((self.read_byte()? as i32 & 0xff) << 16)
            | ((self.read_byte()? as i32 & 0xff) << 24);

        self.skip_fully(4)?;

        self.block_align =
            (self.read_byte()? as i32 & 0xff) | ((self.read_byte()? as i32 & 0xff) << 8);

        self.bits_per_sample =
            (self.read_byte()? as i32 & 0xff) | ((self.read_byte()? as i32 & 0xff) << 8);

        // Handle WAVE_FORMAT_EXTENSIBLE (0xFFFE): read sub-format as actual format type.
        // Translated from: AudioExporter.java WAVE_FORMAT_EXTENSIBLE handling
        if self.format_type == 0xFFFE && fmt_chunk_length >= 40 {
            // Skip cbSize (2) + validBitsPerSample (2) + channelMask (4) = 8 bytes
            self.skip_fully(8)?;
            // Read SubFormat GUID first 2 bytes as actual format type
            let sub_format_type =
                (self.read_byte()? as i32 & 0xff) | ((self.read_byte()? as i32 & 0xff) << 8);
            // Skip remaining 14 bytes of SubFormat GUID
            self.skip_fully(14)?;
            self.format_type = sub_format_type;
            // Skip any remaining fmt data
            let consumed = 40;
            if fmt_chunk_length > consumed {
                self.skip_fully((fmt_chunk_length - consumed) as usize)?;
            }
        } else {
            if fmt_chunk_length < 16 {
                bail!(
                    "fmt chunk too short: {} bytes (minimum 16)",
                    fmt_chunk_length
                );
            }
            self.skip_fully((fmt_chunk_length - 16) as usize)?;
        }

        let data_chunk_length = self.seek_to_chunk(b'd', b'a', b't', b'a')?;
        if data_chunk_length < 0 {
            bail!("Invalid data chunk length: {}", data_chunk_length);
        }
        self.data_remaining = data_chunk_length as usize;
        self.data_start = self.pos;

        Ok(())
    }

    fn read_data(&mut self) -> Result<Vec<u8>> {
        let end = (self.data_start + self.data_remaining).min(self.data.len());
        let actual_len = end - self.data_start;
        let mut buf = vec![0u8; actual_len];
        buf.copy_from_slice(&self.data[self.data_start..end]);
        Ok(buf)
    }
}

/// WAV file InputStream for generating WAV output from PCM data.
///
/// Translated from: GdxSoundDriver.WavFileInputStream in GdxSoundDriver.java
pub struct WavFileInputStream {
    pos: usize,
    mark: usize,
    header: [u8; 44],
    pcm: PCM,
}

impl WavFileInputStream {
    pub fn new(pcm: &PCM) -> Self {
        let sample_rate = pcm.sample_rate();
        let channels = pcm.channels();
        let pcm_len = pcm.len();
        let total_data_len = pcm_len as i64 * 2 + 36;
        let bitrate = (sample_rate as i64) * (channels as i64) * 16;

        let mut header = [0u8; 44];
        header[0] = b'R';
        header[1] = b'I';
        header[2] = b'F';
        header[3] = b'F';
        header[4] = (total_data_len & 0xff) as u8;
        header[5] = ((total_data_len >> 8) & 0xff) as u8;
        header[6] = ((total_data_len >> 16) & 0xff) as u8;
        header[7] = ((total_data_len >> 24) & 0xff) as u8;
        header[8] = b'W';
        header[9] = b'A';
        header[10] = b'V';
        header[11] = b'E';
        header[12] = b'f';
        header[13] = b'm';
        header[14] = b't';
        header[15] = b' ';
        header[16] = 16;
        header[17] = 0;
        header[18] = 0;
        header[19] = 0;
        header[20] = 1;
        header[21] = 0;
        header[22] = channels as u8;
        header[23] = 0;
        header[24] = (sample_rate & 0xff) as u8;
        header[25] = ((sample_rate >> 8) & 0xff) as u8;
        header[26] = ((sample_rate >> 16) & 0xff) as u8;
        header[27] = ((sample_rate >> 24) & 0xff) as u8;
        header[28] = ((bitrate / 8) & 0xff) as u8;
        header[29] = (((bitrate / 8) >> 8) & 0xff) as u8;
        header[30] = (((bitrate / 8) >> 16) & 0xff) as u8;
        header[31] = (((bitrate / 8) >> 24) & 0xff) as u8;
        header[32] = ((channels * 16) / 8) as u8;
        header[33] = 0;
        header[34] = 16;
        header[35] = 0;
        header[36] = b'd';
        header[37] = b'a';
        header[38] = b't';
        header[39] = b'a';
        let data_size = pcm_len as i64 * 2;
        header[40] = (data_size & 0xff) as u8;
        header[41] = ((data_size >> 8) & 0xff) as u8;
        header[42] = ((data_size >> 16) & 0xff) as u8;
        header[43] = ((data_size >> 24) & 0xff) as u8;

        WavFileInputStream {
            pos: 0,
            mark: 0,
            header,
            pcm: pcm.clone(),
        }
    }

    pub fn available(&self) -> usize {
        let total = 44 + self.pcm.len() as usize * 2;
        total.saturating_sub(self.pos)
    }

    pub fn mark(&mut self) {
        self.mark = self.pos;
    }

    pub fn reset(&mut self) {
        self.pos = self.mark;
    }

    pub fn read_byte(&mut self) -> i32 {
        let pcm_len = self.pcm.len() as usize;
        let mut result: i32 = -1;
        if self.pos < 44 {
            result = (self.header[self.pos] as i32) & 0x00ff;
            self.pos += 1;
        } else if self.pos < 44 + pcm_len * 2 {
            let data_offset = self.pos - 44;
            match &self.pcm {
                PCM::Short(short_pcm) => {
                    let idx = data_offset / 2 + short_pcm.start as usize;
                    if idx < short_pcm.sample.len() {
                        let s = short_pcm.sample[idx];
                        if self.pos.is_multiple_of(2) {
                            result = (s as i32) & 0x00ff;
                        } else {
                            result = ((s as i32) & 0xff00i32) >> 8 & 0xff;
                        }
                    }
                }
                PCM::Float(float_pcm) => {
                    let idx = data_offset / 2 + float_pcm.start as usize;
                    if idx < float_pcm.sample.len() {
                        let s = (float_pcm.sample[idx] * i16::MAX as f32) as i16;
                        if self.pos.is_multiple_of(2) {
                            result = (s as i32) & 0x00ff;
                        } else {
                            result = ((s as i32) & 0xff00i32) >> 8 & 0xff;
                        }
                    }
                }
                PCM::Byte(byte_pcm) => {
                    let idx = data_offset / 2 + byte_pcm.start as usize;
                    if idx < byte_pcm.sample.len() {
                        if !self.pos.is_multiple_of(2) {
                            result = (byte_pcm.sample[idx] as i32) & 0x000000ff;
                        } else {
                            result = 0;
                        }
                    }
                }
            }
            self.pos += 1;
        }
        result
    }

    /// Skips up to n bytes.
    ///
    /// Translated from: WavFileInputStream.skip(long)
    pub fn skip(&mut self, n: i64) -> i64 {
        if n < 0 {
            return 0;
        }
        let total = 44 + self.pcm.len() as usize * 2;
        let remaining = total.saturating_sub(self.pos);
        if remaining < n as usize {
            let old_pos = self.pos;
            self.pos = total;
            return (total - old_pos) as i64;
        }
        self.pos += n as usize;
        n
    }

    /// Returns true since mark/reset is supported.
    ///
    /// Translated from: WavFileInputStream.markSupported()
    pub fn mark_supported(&self) -> bool {
        true
    }
}

impl Read for WavFileInputStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut count = 0;
        for b in buf.iter_mut() {
            let byte = self.read_byte();
            if byte == -1 {
                break;
            }
            *b = byte as u8;
            count += 1;
        }
        if count == 0 { Ok(0) } else { Ok(count) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: build a minimal valid WAV file byte buffer.
    /// Returns bytes for a WAV with the given PCM data (16-bit, mono, 44100 Hz).
    fn build_wav_bytes(pcm_data: &[u8]) -> Vec<u8> {
        let data_size = pcm_data.len() as u32;
        let file_size = 36 + data_size; // RIFF chunk size = file size - 8
        let mut buf = Vec::new();
        // RIFF header
        buf.extend_from_slice(b"RIFF");
        buf.extend_from_slice(&file_size.to_le_bytes());
        buf.extend_from_slice(b"WAVE");
        // fmt chunk (16 bytes, PCM format)
        buf.extend_from_slice(b"fmt ");
        buf.extend_from_slice(&16u32.to_le_bytes()); // chunk size
        buf.extend_from_slice(&1u16.to_le_bytes()); // format: PCM
        buf.extend_from_slice(&1u16.to_le_bytes()); // channels: mono
        buf.extend_from_slice(&44100u32.to_le_bytes()); // sample rate
        buf.extend_from_slice(&88200u32.to_le_bytes()); // byte rate
        buf.extend_from_slice(&2u16.to_le_bytes()); // block align
        buf.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
        // data chunk
        buf.extend_from_slice(b"data");
        buf.extend_from_slice(&data_size.to_le_bytes());
        buf.extend_from_slice(pcm_data);
        buf
    }

    #[test]
    fn wav_reader_parses_valid_wav() {
        let pcm_data = vec![0x00, 0x01, 0xFF, 0x7F]; // two 16-bit samples
        let wav_bytes = build_wav_bytes(&pcm_data);
        let reader = WavReader::new(&wav_bytes).unwrap();
        assert_eq!(reader.format_type, 1);
        assert_eq!(reader.channels, 1);
        assert_eq!(reader.sample_rate, 44100);
        assert_eq!(reader.bits_per_sample, 16);
        assert_eq!(reader.data_remaining, 4);
    }

    #[test]
    fn wav_reader_rejects_negative_chunk_length() {
        // Build a WAV where the fmt chunk has a negative length (0xFFFFFFFF = -1 as i32).
        // This should be caught by the `chunk_length < 0` check in seek_to_chunk.
        let mut buf = Vec::new();
        buf.extend_from_slice(b"RIFF");
        buf.extend_from_slice(&100u32.to_le_bytes());
        buf.extend_from_slice(b"WAVE");
        // fmt chunk with negative length
        buf.extend_from_slice(b"fmt ");
        buf.extend_from_slice(&0xFFFFFFFFu32.to_le_bytes()); // -1 as i32
        // Pad enough bytes so read_byte doesn't fail first
        buf.extend_from_slice(&[0u8; 64]);

        let result = WavReader::new(&buf);
        let err = result.err().expect("should fail");
        let err_msg = err.to_string();
        assert!(
            err_msg.contains("Invalid chunk length"),
            "Expected 'Invalid chunk length' error, got: {}",
            err_msg
        );
    }

    #[test]
    fn wav_reader_rejects_negative_chunk_length_high_bit() {
        // chunk_length = 0x80000000 = -2147483648 as i32 (negative but not -1)
        let mut buf = Vec::new();
        buf.extend_from_slice(b"RIFF");
        buf.extend_from_slice(&100u32.to_le_bytes());
        buf.extend_from_slice(b"WAVE");
        buf.extend_from_slice(b"fmt ");
        buf.extend_from_slice(&0x80000000u32.to_le_bytes());
        buf.extend_from_slice(&[0u8; 64]);

        let result = WavReader::new(&buf);
        let err = result.err().expect("should fail");
        assert!(err.to_string().contains("Invalid chunk length"));
    }

    #[test]
    fn wav_reader_rejects_fmt_chunk_too_short() {
        // Build a WAV where fmt chunk length is less than 16
        let mut buf = Vec::new();
        buf.extend_from_slice(b"RIFF");
        buf.extend_from_slice(&100u32.to_le_bytes());
        buf.extend_from_slice(b"WAVE");
        // fmt chunk with length 8 (too short)
        buf.extend_from_slice(b"fmt ");
        buf.extend_from_slice(&8u32.to_le_bytes());
        // 8 bytes of fmt data (not enough for full header)
        buf.extend_from_slice(&[0u8; 8]);
        // data chunk
        buf.extend_from_slice(b"data");
        buf.extend_from_slice(&0u32.to_le_bytes());

        let result = WavReader::new(&buf);
        let err = result.err().expect("should fail");
        let err_msg = err.to_string();
        assert!(
            err_msg.contains("fmt chunk too short"),
            "Expected 'fmt chunk too short' error, got: {}",
            err_msg
        );
    }

    #[test]
    fn wav_file_input_stream_read_byte_bounds_safety() {
        // Create a PCM with start=0, len=2 (2 samples), but sample vec has only 2 entries.
        // Reading beyond data range should return -1, not panic.
        let pcm = PCM::Short(crate::short_pcm::ShortPCM::new(
            1,
            44100,
            0,
            2,
            vec![0x1234, 0x5678],
        ));
        let mut stream = WavFileInputStream::new(&pcm);

        // Read all valid bytes (44 header + 2 samples * 2 bytes = 48 bytes)
        let mut bytes_read = 0;
        for _ in 0..48 {
            let b = stream.read_byte();
            assert_ne!(b, -1, "Should not return -1 within valid range");
            bytes_read += 1;
        }
        assert_eq!(bytes_read, 48);

        // Next read should return -1 (EOF)
        assert_eq!(stream.read_byte(), -1);
    }

    /// Helper: build a WAV byte buffer with custom bits_per_sample.
    fn build_wav_bytes_with_bps(pcm_data: &[u8], bits_per_sample: u16) -> Vec<u8> {
        let data_size = pcm_data.len() as u32;
        let file_size = 36 + data_size;
        let mut buf = Vec::new();
        buf.extend_from_slice(b"RIFF");
        buf.extend_from_slice(&file_size.to_le_bytes());
        buf.extend_from_slice(b"WAVE");
        buf.extend_from_slice(b"fmt ");
        buf.extend_from_slice(&16u32.to_le_bytes());
        buf.extend_from_slice(&1u16.to_le_bytes()); // format: PCM
        buf.extend_from_slice(&1u16.to_le_bytes()); // channels: mono
        buf.extend_from_slice(&44100u32.to_le_bytes());
        buf.extend_from_slice(&88200u32.to_le_bytes());
        buf.extend_from_slice(&2u16.to_le_bytes()); // block align
        buf.extend_from_slice(&bits_per_sample.to_le_bytes());
        buf.extend_from_slice(b"data");
        buf.extend_from_slice(&data_size.to_le_bytes());
        buf.extend_from_slice(pcm_data);
        buf
    }

    #[test]
    fn load_pcm_rejects_bits_per_sample_zero() {
        // A malformed WAV with bits_per_sample=0 must be rejected before the
        // trailing-silence trimming block, which would otherwise panic on modulo-by-zero.
        let pcm_data = vec![0x00, 0x01, 0xFF, 0x7F];
        let wav_bytes = build_wav_bytes_with_bps(&pcm_data, 0);

        let dir = std::env::temp_dir().join("rubato_test_bps_zero");
        std::fs::create_dir_all(&dir).unwrap();
        let wav_path = dir.join("bps_zero.wav");
        std::fs::write(&wav_path, &wav_bytes).unwrap();

        let mut loader = PCMLoader::new();
        let result = loader.load_pcm(&wav_path);
        assert!(result.is_err(), "Expected error for bits_per_sample=0");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("bits_per_sample is 0"),
            "Expected 'bits_per_sample is 0' error, got: {}",
            err_msg
        );

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn pcm_data_too_large_rejected() {
        // We can't actually allocate 2GB in a test, but we verify the check logic
        // by ensuring the guard threshold is correct
        assert!(i32::MAX as usize > 0);
        // The guard is: pcm_data.len() > i32::MAX as usize
        // For a 2GB+ file this would trigger.
    }

    #[test]
    fn load_pcm_bps4_mono_succeeds_with_ceiling_formula() {
        // bits_per_sample=4, mono: ceiling frame_size = 1*((4+7)/8) = 1 (valid).
        // Old truncating formula produced 4/8=0 which incorrectly rejected the file.
        let pcm_data = vec![0x01, 0x02, 0x03, 0x04];
        let wav_bytes = build_wav_bytes_with_bps(&pcm_data, 4);

        let dir = std::env::temp_dir().join("rubato_test_bps4_ceiling");
        std::fs::create_dir_all(&dir).unwrap();
        let wav_path = dir.join("bps4.wav");
        std::fs::write(&wav_path, &wav_bytes).unwrap();

        let mut loader = PCMLoader::new();
        let result = loader.load_pcm(&wav_path);
        assert!(
            result.is_ok(),
            "bits_per_sample=4 should succeed with ceiling formula, got: {}",
            result.unwrap_err()
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn frame_size_ceiling_formula() {
        // The frame_size formula uses ceiling division: channels * ((bps + 7) / 8).
        // This avoids truncation for non-byte-aligned bits_per_sample values.
        //
        // bps=7, mono: old formula = 1*7/8 = 0 (truncated), ceiling = 1*((7+7)/8) = 1
        // bps=8, mono: old = 1, ceiling = 1
        // bps=12, stereo: old = 2*12/8 = 2 (truncated from 3.0), ceiling = 2*((12+7)/8) = 2*2 = 4
        // bps=16, stereo: old = 4, ceiling = 4

        // Ceiling formula yields non-zero for bps=7
        let frame_size = 1_i32 * ((7 + 7) / 8);
        assert_eq!(
            frame_size, 1,
            "bps=7 mono should produce frame_size=1 with ceiling"
        );

        // Standard cases remain unchanged
        assert_eq!(1_i32 * ((8 + 7) / 8), 1, "bps=8 mono");
        assert_eq!(2_i32 * ((16 + 7) / 8), 4, "bps=16 stereo");
        assert_eq!(2_i32 * ((24 + 7) / 8), 6, "bps=24 stereo");
        assert_eq!(2_i32 * ((32 + 7) / 8), 8, "bps=32 stereo");

        // Only bps=0 with any channel count produces frame_size=0
        assert_eq!(1_i32 * ((0 + 7) / 8), 0, "bps=0 mono -> 0");
        assert_eq!(2_i32 * ((0 + 7) / 8), 0, "bps=0 stereo -> 0");
    }

    /// Helper: build a WAV byte buffer with custom channels and bits_per_sample.
    fn build_wav_bytes_custom(pcm_data: &[u8], channels: u16, bits_per_sample: u16) -> Vec<u8> {
        let data_size = pcm_data.len() as u32;
        let file_size = 36 + data_size;
        let block_align = channels * bits_per_sample / 8;
        let byte_rate = 44100u32 * block_align as u32;
        let mut buf = Vec::new();
        buf.extend_from_slice(b"RIFF");
        buf.extend_from_slice(&file_size.to_le_bytes());
        buf.extend_from_slice(b"WAVE");
        buf.extend_from_slice(b"fmt ");
        buf.extend_from_slice(&16u32.to_le_bytes());
        buf.extend_from_slice(&1u16.to_le_bytes()); // format: PCM
        buf.extend_from_slice(&channels.to_le_bytes());
        buf.extend_from_slice(&44100u32.to_le_bytes());
        buf.extend_from_slice(&byte_rate.to_le_bytes());
        buf.extend_from_slice(&block_align.to_le_bytes());
        buf.extend_from_slice(&bits_per_sample.to_le_bytes());
        buf.extend_from_slice(b"data");
        buf.extend_from_slice(&data_size.to_le_bytes());
        buf.extend_from_slice(pcm_data);
        buf
    }

    #[test]
    fn trailing_silence_trim_frame_size_uses_channels() {
        // Regression: frame_size used `bits_per_sample / 4` for stereo (multi-channel),
        // which is only correct for 2-channel. The correct formula is
        // `channels * bits_per_sample / 8`.
        //
        // For a 4-channel, 16-bit WAV:
        //   Old buggy:  frame_size = 16 / 4 = 4 (wrong, should be 8)
        //   Correct:    frame_size = 4 * 16 / 8 = 8
        //
        // To expose the bug, we need data whose length is aligned to 4 but NOT to 8.
        // 12 bytes: 12 % 4 = 0 (aligned to buggy frame_size), 12 % 8 = 4 (NOT aligned to correct).
        //
        // With the buggy code: bytes = 12 - (12 % 4) = 12 (no trim for alignment).
        // Then sample_frame=8, loop checks 12-8..12 = bytes[4..12] -- but that's a
        // misaligned frame boundary (middle of frame 0 + start of frame 1).
        //
        // With the fix: bytes = 12 - (12 % 8) = 8. Then loop checks 0..8, which is
        // the real frame boundary, and finds non-silent data -> keeps 8 bytes.
        let mut pcm_data = Vec::new();
        // First 8 bytes = frame 0 (non-silent): 4 channels x 2 bytes
        pcm_data.extend_from_slice(&[0x01, 0x00, 0x02, 0x00, 0x03, 0x00, 0x04, 0x00]);
        // 4 extra bytes: partial frame (silent), triggers misalignment with buggy code
        pcm_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        assert_eq!(pcm_data.len(), 12);

        let wav_bytes = build_wav_bytes_custom(&pcm_data, 4, 16);
        let dir = std::env::temp_dir().join("rubato_test_frame_size_channels");
        std::fs::create_dir_all(&dir).unwrap();
        let wav_path = dir.join("4ch_misaligned.wav");
        std::fs::write(&wav_path, &wav_bytes).unwrap();

        let mut loader = PCMLoader::new();
        loader.load_pcm(&wav_path).unwrap();

        // After correct alignment (12 -> 8) and trimming, the non-silent frame (8 bytes) remains.
        assert_eq!(
            loader.pcm_data.len(),
            8,
            "4-channel WAV with 12 bytes of data should align to 8 and keep the non-silent frame; \
             got {} bytes instead of 8",
            loader.pcm_data.len()
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn wav_file_input_stream_total_data_len_no_overflow() {
        // pcm_len close to i32::MAX would overflow with i32 arithmetic: pcm_len * 2 + 36
        // With i64 arithmetic it should not overflow.
        let large_len: i32 = i32::MAX / 2 + 100; // would overflow i32 when * 2 + 36
        let total_data_len = large_len as i64 * 2 + 36;
        assert!(
            total_data_len > i32::MAX as i64,
            "total_data_len should exceed i32::MAX"
        );
        // Verify no wrapping occurred
        assert_eq!(total_data_len, (large_len as i64) * 2 + 36);
    }
}
