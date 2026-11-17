//! Opus, statically linked from the vendored libopus in `opus/` (see
//! `opus/README.md`): 48 kHz mono, 20 ms frames, as Simple Voice Chat codes
//! them. No system libopus is needed, so voice chat also works on Android.
use std::ffi::{c_int, c_void};

pub const SAMPLE_RATE: u32 = 48_000;
pub const FRAME: usize = 960;

pub const APPLICATION_VOIP: i32 = 2048;
pub const APPLICATION_AUDIO: i32 = 2049;
pub const APPLICATION_RESTRICTED_LOWDELAY: i32 = 2051;
const OPUS_RESET_STATE: c_int = 4028;

extern "C" {
    fn opus_encoder_create(fs: c_int, channels: c_int, application: c_int, error: *mut c_int)
        -> *mut c_void;
    fn opus_encode(
        st: *mut c_void,
        pcm: *const i16,
        frame_size: c_int,
        data: *mut u8,
        max_data_bytes: i32,
    ) -> i32;
    fn opus_encoder_destroy(st: *mut c_void);
    fn opus_encoder_ctl(st: *mut c_void, request: c_int, ...) -> c_int;
    fn opus_decoder_create(fs: c_int, channels: c_int, error: *mut c_int) -> *mut c_void;
    fn opus_decode(
        st: *mut c_void,
        data: *const u8,
        len: i32,
        pcm: *mut i16,
        frame_size: c_int,
        decode_fec: c_int,
    ) -> c_int;
    fn opus_decoder_destroy(st: *mut c_void);
    fn opus_decoder_ctl(st: *mut c_void, request: c_int, ...) -> c_int;
}

/// Whether Opus works: make an encoder and a decoder and drop them.
pub fn check() -> Result<(), String> {
    let _enc = Encoder::new(APPLICATION_VOIP, 1275)?;
    let _dec = Decoder::new()?;
    Ok(())
}

pub struct Encoder {
    raw: *mut c_void,
    max_bytes: usize,
}

// SAFETY: an encoder is used by one thread at a time.
unsafe impl Send for Encoder {}

impl Encoder {
    /// `max_bytes`: the server's MTU, Opus's 1275 at most.
    pub fn new(application: i32, max_bytes: usize) -> Result<Self, String> {
        let mut err = 0;
        let raw = unsafe { opus_encoder_create(SAMPLE_RATE as i32, 1, application, &mut err) };
        if raw.is_null() || err != 0 {
            return Err(format!("opus_encoder_create failed ({err})"));
        }
        Ok(Self { raw, max_bytes: max_bytes.clamp(64, 1275) })
    }

    pub fn encode(&mut self, frame: &[i16; FRAME]) -> Result<Vec<u8>, String> {
        let mut out = vec![0u8; self.max_bytes];
        let n = unsafe { opus_encode(self.raw, frame.as_ptr(), FRAME as c_int, out.as_mut_ptr(), out.len() as i32) };
        if n < 0 {
            return Err(format!("opus_encode failed ({n})"));
        }
        out.truncate(n as usize);
        Ok(out)
    }

    pub fn reset(&mut self) {
        unsafe { opus_encoder_ctl(self.raw, OPUS_RESET_STATE) };
    }
}

impl Drop for Encoder {
    fn drop(&mut self) {
        unsafe { opus_encoder_destroy(self.raw) };
    }
}

pub struct Decoder {
    raw: *mut c_void,
}

// SAFETY: a decoder is used by one thread at a time.
unsafe impl Send for Decoder {}

impl Decoder {
    pub fn new() -> Result<Self, String> {
        let mut err = 0;
        let raw = unsafe { opus_decoder_create(SAMPLE_RATE as i32, 1, &mut err) };
        if raw.is_null() || err != 0 {
            return Err(format!("opus_decoder_create failed ({err})"));
        }
        Ok(Self { raw })
    }

    /// A frame from `packet`; None conceals a lost one, and `fec` rebuilds
    /// the lost frame before `packet` from the redundancy in it.
    pub fn decode(&mut self, packet: Option<&[u8]>, fec: bool) -> Vec<i16> {
        let mut pcm = vec![0i16; FRAME];
        let (ptr, len) = packet.map_or((std::ptr::null(), 0), |p| (p.as_ptr(), p.len() as i32));
        let n = unsafe { opus_decode(self.raw, ptr, len, pcm.as_mut_ptr(), FRAME as c_int, fec as c_int) };
        if n < 0 {
            pcm.fill(0);
        } else {
            pcm.truncate(n as usize);
            pcm.resize(FRAME, 0);
        }
        pcm
    }

    pub fn reset(&mut self) {
        unsafe { opus_decoder_ctl(self.raw, OPUS_RESET_STATE) };
    }
}

impl Drop for Decoder {
    fn drop(&mut self) {
        unsafe { opus_decoder_destroy(self.raw) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tone_survives_encoding() {
        let mut enc = Encoder::new(APPLICATION_VOIP, 1275).unwrap();
        let mut dec = Decoder::new().unwrap();
        let mut last = Vec::new();
        for f in 0..10 {
            let frame: [i16; FRAME] = std::array::from_fn(|i| {
                let t = (f * FRAME + i) as f32 / SAMPLE_RATE as f32;
                ((t * 440.0 * std::f32::consts::TAU).sin() * 8000.0) as i16
            });
            let packet = enc.encode(&frame).unwrap();
            assert!(!packet.is_empty() && packet.len() <= 1275);
            last = dec.decode(Some(&packet), false);
        }
        let peak = last.iter().map(|s| s.unsigned_abs()).max().unwrap();
        assert!(peak > 4000, "peak {peak}");
        assert_eq!(dec.decode(None, false).len(), FRAME);
    }
}
