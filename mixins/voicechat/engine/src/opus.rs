//! Opus from the system's libopus, loaded when voice chat first starts: 48 kHz
//! mono, 20 ms frames, as Simple Voice Chat codes them.
use libloading::Library;
use std::ffi::{c_int, c_void};
use std::sync::OnceLock;

pub const SAMPLE_RATE: u32 = 48_000;
pub const FRAME: usize = 960;

pub const APPLICATION_VOIP: i32 = 2048;
pub const APPLICATION_AUDIO: i32 = 2049;
pub const APPLICATION_RESTRICTED_LOWDELAY: i32 = 2051;
const OPUS_RESET_STATE: c_int = 4028;

type EncoderCreate = unsafe extern "C" fn(i32, c_int, c_int, *mut c_int) -> *mut c_void;
type Encode = unsafe extern "C" fn(*mut c_void, *const i16, c_int, *mut u8, i32) -> i32;
type DecoderCreate = unsafe extern "C" fn(i32, c_int, *mut c_int) -> *mut c_void;
type Decode = unsafe extern "C" fn(*mut c_void, *const u8, i32, *mut i16, c_int, c_int) -> c_int;
type Destroy = unsafe extern "C" fn(*mut c_void);
type Ctl = unsafe extern "C" fn(*mut c_void, c_int, ...) -> c_int;

struct Api {
    _lib: Library,
    encoder_create: EncoderCreate,
    encode: Encode,
    encoder_destroy: Destroy,
    encoder_ctl: Ctl,
    decoder_create: DecoderCreate,
    decode: Decode,
    decoder_destroy: Destroy,
    decoder_ctl: Ctl,
}

const NAMES: &[&str] = &[
    "libopus.so.0",
    "libopus.so",
    "libopus.0.dylib",
    "libopus.dylib",
    "/opt/homebrew/lib/libopus.0.dylib",
    "/usr/local/lib/libopus.0.dylib",
    "opus.dll",
    "libopus-0.dll",
    "libopus.dll",
    "opus-0.dll",
];

static BUNDLED: OnceLock<Option<(&'static str, &'static [u8])>> = OnceLock::new();

pub fn register_bundled(lib: Option<(&'static str, &'static [u8])>) {
    let _ = BUNDLED.set(lib);
}

fn api() -> Result<&'static Api, String> {
    static API: OnceLock<Result<Api, String>> = OnceLock::new();
    API.get_or_init(load).as_ref().map_err(Clone::clone)
}

fn load() -> Result<Api, String> {
    let mut errors = Vec::new();
    for name in NAMES {
        // SAFETY: libopus runs no code of its own when it loads.
        match unsafe { Library::new(name) } {
            Ok(lib) => return unsafe { symbols(lib) }.map_err(|e| format!("{name}: {e}")),
            Err(e) => errors.push(e.to_string()),
        }
    }
    if let Some(Some((name, bytes))) = BUNDLED.get().copied() {
        match load_bundled(name, bytes) {
            Ok(api) => return Ok(api),
            Err(e) => errors.push(format!("bundled {name}: {e}")),
        }
    }
    Err(format!(
        "libopus not found (install the opus package, or put a libopus beside the client): {}",
        errors.join("; ")
    ))
}

fn load_bundled(name: &str, bytes: &[u8]) -> Result<Api, String> {
    let path = cache_lib(name, bytes).map_err(|e| e.to_string())?;
    // SAFETY: libopus runs no code of its own when it loads.
    let lib = unsafe { Library::new(&path) }.map_err(|e| e.to_string())?;
    unsafe { symbols(lib) }.map_err(|e| e.to_string())
}

fn cache_lib(name: &str, bytes: &[u8]) -> std::io::Result<std::path::PathBuf> {
    use std::io::Write;
    let base = std::path::Path::new(name);
    let stem = base.file_stem().and_then(|s| s.to_str()).unwrap_or("libopus");
    let versioned = match base.extension().and_then(|s| s.to_str()) {
        Some(ext) => format!("{stem}-{}.{ext}", bytes.len()),
        None => format!("{stem}-{}", bytes.len()),
    };
    let dir = std::env::temp_dir().join("horizon-voicechat");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(&versioned);
    let stale = std::fs::metadata(&path).map(|m| m.len() != bytes.len() as u64).unwrap_or(true);
    if stale {
        let tmp = dir.join(format!("{versioned}.{}.tmp", std::process::id()));
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        drop(f);
        std::fs::rename(&tmp, &path)?;
    }
    Ok(path)
}

unsafe fn symbols(lib: Library) -> Result<Api, libloading::Error> {
    unsafe {
        Ok(Api {
            encoder_create: *lib.get(b"opus_encoder_create\0")?,
            encode: *lib.get(b"opus_encode\0")?,
            encoder_destroy: *lib.get(b"opus_encoder_destroy\0")?,
            encoder_ctl: *lib.get(b"opus_encoder_ctl\0")?,
            decoder_create: *lib.get(b"opus_decoder_create\0")?,
            decode: *lib.get(b"opus_decode\0")?,
            decoder_destroy: *lib.get(b"opus_decoder_destroy\0")?,
            decoder_ctl: *lib.get(b"opus_decoder_ctl\0")?,
            _lib: lib,
        })
    }
}

/// Whether libopus loads, and why not.
pub fn check() -> Result<(), String> {
    api().map(|_| ())
}

pub struct Encoder {
    api: &'static Api,
    raw: *mut c_void,
    max_bytes: usize,
}

// SAFETY: an encoder is used by one thread at a time.
unsafe impl Send for Encoder {}

impl Encoder {
    /// `max_bytes`: the server's MTU, Opus's 1275 at most.
    pub fn new(application: i32, max_bytes: usize) -> Result<Self, String> {
        let api = api()?;
        let mut err = 0;
        let raw = unsafe { (api.encoder_create)(SAMPLE_RATE as i32, 1, application, &mut err) };
        if raw.is_null() || err != 0 {
            return Err(format!("opus_encoder_create failed ({err})"));
        }
        Ok(Self { api, raw, max_bytes: max_bytes.clamp(64, 1275) })
    }

    pub fn encode(&mut self, frame: &[i16; FRAME]) -> Result<Vec<u8>, String> {
        let mut out = vec![0u8; self.max_bytes];
        let n = unsafe {
            (self.api.encode)(self.raw, frame.as_ptr(), FRAME as c_int, out.as_mut_ptr(), out.len() as i32)
        };
        if n < 0 {
            return Err(format!("opus_encode failed ({n})"));
        }
        out.truncate(n as usize);
        Ok(out)
    }

    pub fn reset(&mut self) {
        unsafe { (self.api.encoder_ctl)(self.raw, OPUS_RESET_STATE) };
    }
}

impl Drop for Encoder {
    fn drop(&mut self) {
        unsafe { (self.api.encoder_destroy)(self.raw) };
    }
}

pub struct Decoder {
    api: &'static Api,
    raw: *mut c_void,
}

// SAFETY: a decoder is used by one thread at a time.
unsafe impl Send for Decoder {}

impl Decoder {
    pub fn new() -> Result<Self, String> {
        let api = api()?;
        let mut err = 0;
        let raw = unsafe { (api.decoder_create)(SAMPLE_RATE as i32, 1, &mut err) };
        if raw.is_null() || err != 0 {
            return Err(format!("opus_decoder_create failed ({err})"));
        }
        Ok(Self { api, raw })
    }

    /// A frame from `packet`; None conceals a lost one, and `fec` rebuilds
    /// the lost frame before `packet` from the redundancy in it.
    pub fn decode(&mut self, packet: Option<&[u8]>, fec: bool) -> Vec<i16> {
        let mut pcm = vec![0i16; FRAME];
        let (ptr, len) = packet.map_or((std::ptr::null(), 0), |p| (p.as_ptr(), p.len() as i32));
        let n = unsafe {
            (self.api.decode)(self.raw, ptr, len, pcm.as_mut_ptr(), FRAME as c_int, fec as c_int)
        };
        if n < 0 {
            pcm.fill(0);
        } else {
            pcm.truncate(n as usize);
            pcm.resize(FRAME, 0);
        }
        pcm
    }

    pub fn reset(&mut self) {
        unsafe { (self.api.decoder_ctl)(self.raw, OPUS_RESET_STATE) };
    }
}

impl Drop for Decoder {
    fn drop(&mut self) {
        unsafe { (self.api.decoder_destroy)(self.raw) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tone_survives_encoding() {
        if check().is_err() {
            eprintln!("no libopus here: skipped");
            return;
        }
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

    #[test]
    fn bundled_cache_writes_and_reuses() {
        let bytes = format!("opus-test-{}", std::process::id()).into_bytes();
        let first = cache_lib("opus.dll", &bytes).unwrap();
        assert_eq!(std::fs::read(&first).unwrap(), bytes);
        assert_eq!(first.extension().and_then(|e| e.to_str()), Some("dll"));
        let again = cache_lib("opus.dll", &bytes).unwrap();
        assert_eq!(first, again);
        let _ = std::fs::remove_file(&first);
    }
}
