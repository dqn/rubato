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

        // Trim trailing silence
        let mut bytes = self.pcm_data.len() as i32;
        let frame_size = if self.channels > 1 {
            self.bits_per_sample / 4
        } else {
            self.bits_per_sample / 8
        };
        bytes -= bytes % frame_size;

        let sample_frame = self.channels * self.bits_per_sample / 8;
        while bytes > sample_frame {
            let frame_start = (bytes - sample_frame) as usize;
            let frame_end = bytes as usize;
            let zero = self.pcm_data[frame_start..frame_end]
                .iter()
                .all(|&b| b == 0x00);
            if zero {
                bytes -= sample_frame;
            } else {
                break;
            }
        }

        if bytes < sample_frame {
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

        match wav.format_type {
            1 | 3 => {
                // PCM or IEEE float
                self.channels = wav.channels;
                self.sample_rate = wav.sample_rate;
                self.bits_per_sample = wav.bits_per_sample;

                self.pcm_data = wav.read_data()?;
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
            if chunk_length == -1 {
                bail!(
                    "Chunk not found: {}{}{}{}",
                    c1 as char,
                    c2 as char,
                    c3 as char,
                    c4 as char
                );
            }
            if found {
                return Ok(chunk_length);
            }
            self.skip_fully(chunk_length as usize)?;
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
            self.skip_fully((fmt_chunk_length - 16) as usize)?;
        }

        self.data_remaining = self.seek_to_chunk(b'd', b'a', b't', b'a')? as usize;
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
        let total_data_len = (pcm_len * 2 + 36) as i64;
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
        let data_size = pcm_len * 2;
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
                    let s = short_pcm.sample[data_offset / 2 + short_pcm.start as usize];
                    if self.pos.is_multiple_of(2) {
                        result = (s as i32) & 0x00ff;
                    } else {
                        result = ((s as i32) & 0xff00i32) >> 8 & 0xff;
                    }
                }
                PCM::Float(float_pcm) => {
                    let s = (float_pcm.sample[data_offset / 2 + float_pcm.start as usize]
                        * i16::MAX as f32) as i16;
                    if self.pos.is_multiple_of(2) {
                        result = (s as i32) & 0x00ff;
                    } else {
                        result = ((s as i32) & 0xff00i32) >> 8 & 0xff;
                    }
                }
                PCM::Byte(byte_pcm) => {
                    if !self.pos.is_multiple_of(2) {
                        result = (byte_pcm.sample[data_offset / 2 + byte_pcm.start as usize]
                            as i32)
                            & 0x000000ff;
                    } else {
                        result = 0;
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
