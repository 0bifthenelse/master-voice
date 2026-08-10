use core::fmt;
use core::mem::MaybeUninit;
use core::ptr;

use master_voice_linguistics::phoneme::{Boundary, Phoneme, PhonemeKind, Stress};

pub(crate) const ABI_VERSION: u32 = 1;
pub(crate) const STATUS_OK: u32 = 0;
pub(crate) const STATUS_NULL: u32 = 1;
pub(crate) const STATUS_ABI: u32 = 2;
pub(crate) const STATUS_PHONE: u32 = 3;
pub(crate) const STATUS_NONFINITE: u32 = 4;
pub(crate) const STATUS_OVERFLOW: u32 = 5;
pub(crate) const STATUS_CAPACITY: u32 = 6;
pub(crate) const STATUS_STATE: u32 = 7;
pub(crate) const STATUS_RANGE: u32 = 8;

const CHUNK_FIRST: u64 = 1;
const CHUNK_LAST: u64 = 2;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MaPhone {
    pub kind: u32,
    pub stress: u32,
    pub boundary: u32,
    pub pitch_shift: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub(crate) struct MaOptions {
    pub abi_version: u32,
    pub flags: u32,
    pub rate: f32,
    pub pitch: f32,
    pub volume: f32,
    pub robotic_depth: f32,
    pub seed: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct MaRenderResult {
    pub written: u64,
    pub peak: f32,
    pub status: u32,
}

#[repr(C, align(16))]
pub(crate) struct MaState {
    bytes: [u8; 2048],
}

#[repr(C)]
struct MaRenderRequest {
    state: *mut MaState,
    phones: *const MaPhone,
    phone_count: u64,
    options: *const MaOptions,
    output: *mut f32,
    output_capacity: u64,
    result: *mut MaRenderResult,
    chunk_flags: u64,
}

unsafe extern "C" {
    fn mv_state_init(state: *mut MaState) -> u32;
    fn mv_measure(request: *const MaRenderRequest) -> u32;
    fn mv_render(request: *const MaRenderRequest) -> u32;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FfiError {
    pub status: u32,
}

impl fmt::Display for FfiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self.status {
            STATUS_NULL => "required pointer was null",
            STATUS_ABI => "assembly ABI version mismatch",
            STATUS_PHONE => "invalid phone data",
            STATUS_NONFINITE => "non-finite synthesis value",
            STATUS_OVERFLOW => "sample count overflow",
            STATUS_CAPACITY => "output capacity is too short",
            STATUS_STATE => "synthesis state is not initialized",
            STATUS_RANGE => "synthesis value is outside its supported range",
            _ => "unknown assembly synthesis failure",
        };
        write!(formatter, "{message} (status {})", self.status)
    }
}

impl std::error::Error for FfiError {}

pub(crate) struct EngineState {
    raw: MaState,
}

impl EngineState {
    pub(crate) fn new() -> Result<Self, FfiError> {
        let mut raw = MaybeUninit::<MaState>::uninit();
        let status = unsafe { mv_state_init(raw.as_mut_ptr()) };
        ensure_status(status)?;
        Ok(Self {
            raw: unsafe { raw.assume_init() },
        })
    }

    pub(crate) fn as_mut_ptr(&mut self) -> *mut MaState {
        &mut self.raw
    }
}

pub(crate) fn encode_phones(phonemes: &[Phoneme]) -> Vec<MaPhone> {
    phonemes.iter().copied().map(MaPhone::from).collect()
}

impl From<Phoneme> for MaPhone {
    fn from(phoneme: Phoneme) -> Self {
        Self {
            kind: map_kind(phoneme.kind),
            stress: map_stress(phoneme.stress),
            boundary: map_boundary(phoneme.boundary_after),
            pitch_shift: phoneme.pitch_shift,
        }
    }
}

fn map_kind(kind: PhonemeKind) -> u32 {
    match kind {
        PhonemeKind::IY => 0,
        PhonemeKind::IH => 1,
        PhonemeKind::EH => 2,
        PhonemeKind::EY => 3,
        PhonemeKind::AE => 4,
        PhonemeKind::AA => 5,
        PhonemeKind::AH => 6,
        PhonemeKind::AO => 7,
        PhonemeKind::UH => 8,
        PhonemeKind::UW => 9,
        PhonemeKind::UX => 10,
        PhonemeKind::AX => 11,
        PhonemeKind::ER => 12,
        PhonemeKind::UE => 13,
        PhonemeKind::OE => 14,
        PhonemeKind::OEU => 15,
        PhonemeKind::EN => 16,
        PhonemeKind::AN => 17,
        PhonemeKind::ON => 18,
        PhonemeKind::UN => 19,
        PhonemeKind::EI => 20,
        PhonemeKind::AI => 21,
        PhonemeKind::OI => 22,
        PhonemeKind::OW => 23,
        PhonemeKind::AU => 24,
        PhonemeKind::IA => 25,
        PhonemeKind::EA => 26,
        PhonemeKind::UA => 27,
        PhonemeKind::P => 28,
        PhonemeKind::B => 29,
        PhonemeKind::T => 30,
        PhonemeKind::D => 31,
        PhonemeKind::K => 32,
        PhonemeKind::G => 33,
        PhonemeKind::F => 34,
        PhonemeKind::V => 35,
        PhonemeKind::TH => 36,
        PhonemeKind::DH => 37,
        PhonemeKind::S => 38,
        PhonemeKind::Z => 39,
        PhonemeKind::SH => 40,
        PhonemeKind::ZH => 41,
        PhonemeKind::CH => 42,
        PhonemeKind::JH => 43,
        PhonemeKind::H => 44,
        PhonemeKind::M => 45,
        PhonemeKind::N => 46,
        PhonemeKind::NG => 47,
        PhonemeKind::NY => 48,
        PhonemeKind::L => 49,
        PhonemeKind::R => 50,
        PhonemeKind::RR => 51,
        PhonemeKind::W => 52,
        PhonemeKind::Y => 53,
    }
}

fn map_stress(stress: Stress) -> u32 {
    match stress {
        Stress::None => 0,
        Stress::Secondary => 1,
        Stress::Primary => 2,
    }
}

fn map_boundary(boundary: Boundary) -> u32 {
    match boundary {
        Boundary::None => 0,
        Boundary::Word => 1,
        Boundary::Clause => 2,
        Boundary::Sentence => 3,
        Boundary::Question => 4,
        Boundary::Exclaim => 5,
    }
}

pub(crate) fn options(rate: f32, pitch: f32, volume: f32, robotic_depth: f32) -> MaOptions {
    MaOptions {
        abi_version: ABI_VERSION,
        flags: 0,
        rate,
        pitch,
        volume,
        robotic_depth,
        seed: 0x9e37_79b9_7f4a_7c15,
    }
}

fn flags(first: bool, last: bool) -> u64 {
    (u64::from(first) * CHUNK_FIRST) | (u64::from(last) * CHUNK_LAST)
}

fn request(
    state: *mut MaState,
    phones: &[MaPhone],
    options: &MaOptions,
    output: *mut f32,
    output_capacity: usize,
    result: &mut MaRenderResult,
    chunk_flags: u64,
) -> Result<MaRenderRequest, FfiError> {
    Ok(MaRenderRequest {
        state,
        phones: if phones.is_empty() {
            ptr::null()
        } else {
            phones.as_ptr()
        },
        phone_count: u64::try_from(phones.len()).map_err(|_| FfiError {
            status: STATUS_OVERFLOW,
        })?,
        options,
        output,
        output_capacity: u64::try_from(output_capacity).map_err(|_| FfiError {
            status: STATUS_OVERFLOW,
        })?,
        result,
        chunk_flags,
    })
}

pub(crate) fn measure(
    phones: &[MaPhone],
    options: &MaOptions,
    first: bool,
    last: bool,
) -> Result<usize, FfiError> {
    let mut result = MaRenderResult::default();
    let request = request(
        ptr::null_mut(),
        phones,
        options,
        ptr::null_mut(),
        0,
        &mut result,
        flags(first, last),
    )?;
    let status = unsafe { mv_measure(&request) };
    ensure_status(status)?;
    ensure_status(result.status)?;
    usize::try_from(result.written).map_err(|_| FfiError {
        status: STATUS_OVERFLOW,
    })
}

pub(crate) fn render(
    state: &mut EngineState,
    phones: &[MaPhone],
    options: &MaOptions,
    output: &mut [f32],
    first: bool,
    last: bool,
) -> Result<MaRenderResult, FfiError> {
    let mut result = MaRenderResult::default();
    let request = request(
        state.as_mut_ptr(),
        phones,
        options,
        if output.is_empty() {
            ptr::null_mut()
        } else {
            output.as_mut_ptr()
        },
        output.len(),
        &mut result,
        flags(first, last),
    )?;
    let status = unsafe { mv_render(&request) };
    ensure_status(status)?;
    ensure_status(result.status)?;
    Ok(result)
}

fn ensure_status(status: u32) -> Result<(), FfiError> {
    if status == STATUS_OK {
        Ok(())
    } else {
        Err(FfiError { status })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{align_of, offset_of, size_of};

    #[test]
    fn packed_abi_layout_is_exact() {
        assert_eq!(size_of::<MaPhone>(), 16);
        assert_eq!(offset_of!(MaPhone, kind), 0);
        assert_eq!(offset_of!(MaPhone, stress), 4);
        assert_eq!(offset_of!(MaPhone, boundary), 8);
        assert_eq!(offset_of!(MaPhone, pitch_shift), 12);

        assert_eq!(size_of::<MaOptions>(), 32);
        assert_eq!(offset_of!(MaOptions, abi_version), 0);
        assert_eq!(offset_of!(MaOptions, flags), 4);
        assert_eq!(offset_of!(MaOptions, rate), 8);
        assert_eq!(offset_of!(MaOptions, pitch), 12);
        assert_eq!(offset_of!(MaOptions, volume), 16);
        assert_eq!(offset_of!(MaOptions, robotic_depth), 20);
        assert_eq!(offset_of!(MaOptions, seed), 24);

        assert_eq!(size_of::<MaRenderResult>(), 16);
        assert_eq!(offset_of!(MaRenderResult, written), 0);
        assert_eq!(offset_of!(MaRenderResult, peak), 8);
        assert_eq!(offset_of!(MaRenderResult, status), 12);

        assert_eq!(size_of::<MaState>(), 2048);
        assert_eq!(align_of::<MaState>(), 16);
        assert_eq!(size_of::<MaRenderRequest>(), 64);
        assert_eq!(offset_of!(MaRenderRequest, state), 0);
        assert_eq!(offset_of!(MaRenderRequest, phones), 8);
        assert_eq!(offset_of!(MaRenderRequest, phone_count), 16);
        assert_eq!(offset_of!(MaRenderRequest, options), 24);
        assert_eq!(offset_of!(MaRenderRequest, output), 32);
        assert_eq!(offset_of!(MaRenderRequest, output_capacity), 40);
        assert_eq!(offset_of!(MaRenderRequest, result), 48);
        assert_eq!(offset_of!(MaRenderRequest, chunk_flags), 56);
    }

    #[test]
    fn enum_mapping_covers_every_phone_in_order() {
        use PhonemeKind::*;
        let kinds = [
            IY, IH, EH, EY, AE, AA, AH, AO, UH, UW, UX, AX, ER, UE, OE, OEU, EN, AN, ON, UN, EI,
            AI, OI, OW, AU, IA, EA, UA, P, B, T, D, K, G, F, V, TH, DH, S, Z, SH, ZH, CH, JH, H, M,
            N, NG, NY, L, R, RR, W, Y,
        ];
        for (expected, kind) in kinds.into_iter().enumerate() {
            assert_eq!(map_kind(kind), expected as u32);
        }
    }

    #[test]
    fn assembly_rejects_invalid_requests_with_stable_statuses() {
        assert_eq!(unsafe { mv_measure(ptr::null()) }, STATUS_NULL);

        let phone = MaPhone {
            kind: 0,
            stress: 0,
            boundary: 0,
            pitch_shift: 0.0,
        };
        let mut invalid_abi = options(1.0, 1.0, 1.0, 0.22);
        invalid_abi.abi_version = ABI_VERSION + 1;
        let mut result = MaRenderResult::default();
        let mut request = MaRenderRequest {
            state: ptr::null_mut(),
            phones: &phone,
            phone_count: 1,
            options: &invalid_abi,
            output: ptr::null_mut(),
            output_capacity: 0,
            result: &mut result,
            chunk_flags: CHUNK_FIRST | CHUNK_LAST,
        };

        assert_eq!(unsafe { mv_measure(&request) }, STATUS_ABI);
        assert_eq!(result.status, STATUS_ABI);

        let mut nonfinite = options(1.0, 1.0, 1.0, 0.22);
        nonfinite.pitch = f32::NAN;
        request.options = &nonfinite;
        assert_eq!(unsafe { mv_measure(&request) }, STATUS_NONFINITE);
        assert_eq!(result.status, STATUS_NONFINITE);

        let valid = options(1.0, 1.0, 1.0, 0.22);
        request.options = &valid;
        let invalid_phone = MaPhone { kind: 54, ..phone };
        request.phones = &invalid_phone;
        assert_eq!(unsafe { mv_measure(&request) }, STATUS_PHONE);
        assert_eq!(result.status, STATUS_PHONE);

        request.phones = ptr::null();
        request.phone_count = 0x1000_0000;
        assert_eq!(unsafe { mv_measure(&request) }, STATUS_OVERFLOW);
        assert_eq!(result.status, STATUS_OVERFLOW);
    }

    #[test]
    fn short_capacity_preserves_every_output_canary() {
        let phone = MaPhone {
            kind: 0,
            stress: 2,
            boundary: 3,
            pitch_shift: 0.0,
        };
        let opts = options(1.0, 1.0, 1.0, 0.22);
        let mut state = EngineState::new().expect("initialize state");
        let sentinel = f32::from_bits(0x3f1a_2b3c);
        let mut guarded = [sentinel; 8];
        let before = guarded.map(f32::to_bits);
        let mut result = MaRenderResult::default();
        let request = MaRenderRequest {
            state: state.as_mut_ptr(),
            phones: &phone,
            phone_count: 1,
            options: &opts,
            output: guarded[2..].as_mut_ptr(),
            output_capacity: 1,
            result: &mut result,
            chunk_flags: CHUNK_FIRST | CHUNK_LAST,
        };

        assert_eq!(unsafe { mv_render(&request) }, STATUS_CAPACITY);
        assert_eq!(result.status, STATUS_CAPACITY);
        assert_eq!(guarded.map(f32::to_bits), before);
    }

    #[test]
    fn assembly_renders_finite_audible_pcm() {
        let mut phoneme = Phoneme::new(PhonemeKind::IY);
        phoneme.stress = Stress::Primary;
        phoneme.boundary_after = Boundary::Sentence;
        let phones = encode_phones(&[phoneme]);
        let opts = options(1.0, 1.0, 1.0, 0.22);
        let sample_count = measure(&phones, &opts, true, true).expect("measure");
        assert!((10_000..11_000).contains(&sample_count));

        let mut state = EngineState::new().expect("initialize state");
        let mut output = vec![0.0; sample_count];
        let result = render(&mut state, &phones, &opts, &mut output, true, true).expect("render");
        assert_eq!(result.written as usize, sample_count);
        assert!(output.iter().all(|sample| sample.is_finite()));
        let peak = output.iter().copied().map(f32::abs).fold(0.0, f32::max);
        assert!(peak > 0.001, "peak {peak}");
        assert!(peak < 0.95, "peak {peak}");
    }
}
