use atomic_float::AtomicF32;
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;

pub trait EffectImpl {
    fn process(&self, l: f32, r: f32) -> (f32, f32);
    fn reset(&mut self, sample_rate: f32);
}

#[inline]
fn clamp_finite(value: f32, min: f32, max: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(min, max)
    } else {
        fallback.clamp(min, max)
    }
}

#[derive(Debug)]
pub struct Overdrive {
    pub drive: AtomicF32,
    pub mix: AtomicF32,
    pub output_gain: AtomicF32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OverdriveParams {
    pub drive: f32,
    pub mix: f32,
    pub output_gain: f32,
}

impl Default for OverdriveParams {
    fn default() -> Self {
        Self {
            drive: 0.5,
            mix: 1.0,
            output_gain: 1.0,
        }
    }
}

impl OverdriveParams {
    pub fn validate(&self) -> Result<(), String> {
        if self.drive < 0.0 || self.drive > 1.0 {
            return Err(format!(
                "Overdrive 'drive' must be between 0.0 and 1.0, got {}",
                self.drive
            ));
        }
        if self.mix < 0.0 || self.mix > 1.0 {
            return Err(format!(
                "Overdrive 'mix' must be between 0.0 and 1.0, got {}",
                self.mix
            ));
        }
        if self.output_gain < 0.0 || self.output_gain > 2.0 {
            return Err(format!(
                "Overdrive 'output_gain' must be between 0.0 and 2.0, got {}",
                self.output_gain
            ));
        }
        Ok(())
    }
}

impl From<OverdriveParams> for Overdrive {
    fn from(p: OverdriveParams) -> Self {
        Self {
            drive: AtomicF32::new(p.drive),
            mix: AtomicF32::new(p.mix),
            output_gain: AtomicF32::new(p.output_gain),
        }
    }
}

impl From<&Overdrive> for OverdriveParams {
    fn from(d: &Overdrive) -> Self {
        Self {
            drive: d.drive.load(Ordering::Relaxed),
            mix: d.mix.load(Ordering::Relaxed),
            output_gain: d.output_gain.load(Ordering::Relaxed),
        }
    }
}

impl EffectImpl for Overdrive {
    fn process(&self, l: f32, r: f32) -> (f32, f32) {
        let drive = clamp_finite(self.drive.load(Ordering::Relaxed), 0.0, 1.0, 0.5);
        let mix = clamp_finite(self.mix.load(Ordering::Relaxed), 0.0, 1.0, 1.0);
        let output_gain = clamp_finite(self.output_gain.load(Ordering::Relaxed), 0.0, 2.0, 1.0);

        let gain_factor = 1.0 + drive * 19.0;

        let wet_l = (l * gain_factor).tanh();
        let wet_r = (r * gain_factor).tanh();

        let out_l = l * (1.0 - mix) + wet_l * mix;
        let out_r = r * (1.0 - mix) + wet_r * mix;

        (out_l * output_gain, out_r * output_gain)
    }

    fn reset(&mut self, _sample_rate: f32) {}
}

#[derive(Debug)]
pub struct Delay {
    pub time_ms: AtomicF32,
    pub feedback: AtomicF32,
    pub mix: AtomicF32,

    buffer_l: std::cell::UnsafeCell<Vec<f32>>,
    buffer_r: std::cell::UnsafeCell<Vec<f32>>,
    write_pos: std::cell::UnsafeCell<usize>,
    sample_rate: std::cell::UnsafeCell<f32>,
}

unsafe impl Sync for Delay {}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DelayParams {
    pub time_ms: f32,
    pub feedback: f32,
    pub mix: f32,
}

impl Default for DelayParams {
    fn default() -> Self {
        Self {
            time_ms: 300.0,
            feedback: 0.5,
            mix: 0.4,
        }
    }
}

impl DelayParams {
    pub fn validate(&self) -> Result<(), String> {
        if self.time_ms < 10.0 || self.time_ms > 4000.0 {
            return Err(format!(
                "Delay 'time_ms' must be between 10.0 and 4000.0, got {}",
                self.time_ms
            ));
        }
        if self.feedback < 0.0 || self.feedback >= 1.0 {
            return Err(format!(
                "Delay 'feedback' must be between 0.0 and 0.99, got {}",
                self.feedback
            ));
        }
        if self.mix < 0.0 || self.mix > 1.0 {
            return Err(format!(
                "Delay 'mix' must be between 0.0 and 1.0, got {}",
                self.mix
            ));
        }
        Ok(())
    }
}

impl From<DelayParams> for Delay {
    fn from(p: DelayParams) -> Self {
        Self {
            time_ms: AtomicF32::new(p.time_ms),
            feedback: AtomicF32::new(p.feedback),
            mix: AtomicF32::new(p.mix),
            buffer_l: std::cell::UnsafeCell::new(vec![0.0; 192000]),
            buffer_r: std::cell::UnsafeCell::new(vec![0.0; 192000]),
            write_pos: std::cell::UnsafeCell::new(0),
            sample_rate: std::cell::UnsafeCell::new(44100.0),
        }
    }
}

impl From<&Delay> for DelayParams {
    fn from(d: &Delay) -> Self {
        Self {
            time_ms: d.time_ms.load(Ordering::Relaxed),
            feedback: d.feedback.load(Ordering::Relaxed),
            mix: d.mix.load(Ordering::Relaxed),
        }
    }
}

impl EffectImpl for Delay {
    fn process(&self, l: f32, r: f32) -> (f32, f32) {
        let buffer_l = unsafe { &mut *self.buffer_l.get() };
        let buffer_r = unsafe { &mut *self.buffer_r.get() };
        let write_pos = unsafe { &mut *self.write_pos.get() };
        let sample_rate = clamp_finite(
            unsafe { *self.sample_rate.get() },
            8000.0,
            192000.0,
            44100.0,
        );

        let time_ms = clamp_finite(self.time_ms.load(Ordering::Relaxed), 10.0, 4000.0, 300.0);
        let feedback = clamp_finite(self.feedback.load(Ordering::Relaxed), 0.0, 0.99, 0.5);
        let mix = clamp_finite(self.mix.load(Ordering::Relaxed), 0.0, 1.0, 0.4);

        let delay_samples = (time_ms / 1000.0 * sample_rate).round() as usize;
        let delay_samples = delay_samples.clamp(1, buffer_l.len() - 1);

        let read_pos = if *write_pos >= delay_samples {
            *write_pos - delay_samples
        } else {
            buffer_l.len() - (delay_samples - *write_pos)
        };

        let delayed_l = buffer_l[read_pos];
        let delayed_r = buffer_r[read_pos];

        let next_l = l + delayed_l * feedback;
        let next_r = r + delayed_r * feedback;

        buffer_l[*write_pos] = next_l;
        buffer_r[*write_pos] = next_r;

        *write_pos += 1;
        if *write_pos >= buffer_l.len() {
            *write_pos = 0;
        }

        let out_l = l * (1.0 - mix) + delayed_l * mix;
        let out_r = r * (1.0 - mix) + delayed_r * mix;

        (out_l, out_r)
    }

    fn reset(&mut self, sr: f32) {
        *self.sample_rate.get_mut() = sr;
        *self.write_pos.get_mut() = 0;

        let bl = self.buffer_l.get_mut();
        bl.fill(0.0);
        let br = self.buffer_r.get_mut();
        br.fill(0.0);
    }
}

#[derive(Debug)]
pub struct NoiseGate {
    pub threshold_db: AtomicF32,
    pub ratio: AtomicF32,
    pub attack_ms: AtomicF32,
    pub release_ms: AtomicF32,

    envelope: std::cell::UnsafeCell<f32>,
    smoothed_gain: std::cell::UnsafeCell<f32>,
    sample_rate: std::cell::UnsafeCell<f32>,
}

unsafe impl Sync for NoiseGate {}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NoiseGateParams {
    pub threshold_db: f32,
    pub ratio: f32,
    pub attack_ms: f32,
    pub release_ms: f32,
}

impl Default for NoiseGateParams {
    fn default() -> Self {
        Self {
            threshold_db: -30.0,
            ratio: 10.0,
            attack_ms: 2.0,
            release_ms: 100.0,
        }
    }
}

impl NoiseGateParams {
    pub fn validate(&self) -> Result<(), String> {
        if self.threshold_db < -100.0 || self.threshold_db > 0.0 {
            return Err(format!(
                "NoiseGate 'threshold_db' must be between -100.0 and 0.0, got {}",
                self.threshold_db
            ));
        }
        if self.ratio < 1.0 || self.ratio > 100.0 {
            return Err(format!(
                "NoiseGate 'ratio' must be between 1.0 and 100.0, got {}",
                self.ratio
            ));
        }
        if self.attack_ms < 0.1 || self.attack_ms > 100.0 {
            return Err(format!(
                "NoiseGate 'attack_ms' must be between 0.1 and 100.0, got {}",
                self.attack_ms
            ));
        }
        if self.release_ms < 10.0 || self.release_ms > 1000.0 {
            return Err(format!(
                "NoiseGate 'release_ms' must be between 10.0 and 1000.0, got {}",
                self.release_ms
            ));
        }
        Ok(())
    }
}

impl From<NoiseGateParams> for NoiseGate {
    fn from(p: NoiseGateParams) -> Self {
        Self {
            threshold_db: AtomicF32::new(p.threshold_db),
            ratio: AtomicF32::new(p.ratio),
            attack_ms: AtomicF32::new(p.attack_ms),
            release_ms: AtomicF32::new(p.release_ms),
            envelope: std::cell::UnsafeCell::new(0.0),
            smoothed_gain: std::cell::UnsafeCell::new(1.0),
            sample_rate: std::cell::UnsafeCell::new(44100.0),
        }
    }
}

impl From<&NoiseGate> for NoiseGateParams {
    fn from(n: &NoiseGate) -> Self {
        Self {
            threshold_db: n.threshold_db.load(Ordering::Relaxed),
            ratio: n.ratio.load(Ordering::Relaxed),
            attack_ms: n.attack_ms.load(Ordering::Relaxed),
            release_ms: n.release_ms.load(Ordering::Relaxed),
        }
    }
}

impl EffectImpl for NoiseGate {
    fn process(&self, l: f32, r: f32) -> (f32, f32) {
        let threshold_db = clamp_finite(
            self.threshold_db.load(Ordering::Relaxed),
            -100.0,
            0.0,
            -30.0,
        );
        let ratio = clamp_finite(self.ratio.load(Ordering::Relaxed), 1.0, 100.0, 10.0);
        let attack_ms = clamp_finite(self.attack_ms.load(Ordering::Relaxed), 0.1, 100.0, 2.0);
        let release_ms = clamp_finite(self.release_ms.load(Ordering::Relaxed), 10.0, 1000.0, 100.0);

        let sr = clamp_finite(
            unsafe { *self.sample_rate.get() },
            8000.0,
            192000.0,
            44100.0,
        );
        let env = unsafe { &mut *self.envelope.get() };

        let attack_coeff = (-1.0 / (attack_ms * 0.001 * sr)).exp();
        let release_coeff = (-1.0 / (release_ms * 0.001 * sr)).exp();

        let input_level = l.abs().max(r.abs());

        if input_level > *env {
            *env = attack_coeff * *env + (1.0 - attack_coeff) * input_level;
        } else {
            *env = release_coeff * *env + (1.0 - release_coeff) * input_level;
        }

        let env_db = if *env > 0.000001 {
            20.0 * env.log10()
        } else {
            -100.0
        };

        let mut target_gain = 1.0;
        if env_db < threshold_db {
            let diff = threshold_db - env_db;
            let reduction_db = diff * (1.0 - 1.0 / ratio);

            target_gain = 10.0_f32.powf(-reduction_db / 20.0);
        }

        let smoothed = unsafe { &mut *self.smoothed_gain.get() };
        let smooth_coeff = (-1.0 / (0.005 * sr)).exp();
        *smoothed = smooth_coeff * *smoothed + (1.0 - smooth_coeff) * target_gain;

        (l * *smoothed, r * *smoothed)
    }

    fn reset(&mut self, sr: f32) {
        *self.sample_rate.get_mut() = sr;
        *self.envelope.get_mut() = 0.0;
        *self.smoothed_gain.get_mut() = 1.0;
    }
}

#[derive(Debug, Clone, Copy)]
struct BiquadState {
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

#[derive(Debug)]
pub struct Equalizer {
    pub low_freq: AtomicF32,
    pub low_gain: AtomicF32,
    pub mid_freq: AtomicF32,
    pub mid_gain: AtomicF32,
    pub mid_q: AtomicF32,
    pub high_freq: AtomicF32,
    pub high_gain: AtomicF32,

    filters: std::cell::UnsafeCell<[BiquadState; 6]>,
    sample_rate: std::cell::UnsafeCell<f32>,

    cached_coeffs: std::cell::UnsafeCell<[(f32, f32, f32, f32, f32); 3]>,
    last_params: std::cell::UnsafeCell<Option<(f32, f32, f32, f32, f32, f32, f32)>>,
}

unsafe impl Sync for Equalizer {}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EqualizerParams {
    pub low_freq: f32,
    pub low_gain: f32,
    pub mid_freq: f32,
    pub mid_gain: f32,
    pub mid_q: f32,
    pub high_freq: f32,
    pub high_gain: f32,
}

impl Default for EqualizerParams {
    fn default() -> Self {
        Self {
            low_freq: 200.0,
            low_gain: 0.0,
            mid_freq: 1000.0,
            mid_gain: 0.0,
            mid_q: 1.0,
            high_freq: 4000.0,
            high_gain: 0.0,
        }
    }
}

impl EqualizerParams {
    pub fn validate(&self) -> Result<(), String> {
        if self.low_freq < 20.0 || self.low_freq > 1000.0 {
            return Err(format!(
                "Equalizer 'low_freq' must be between 20.0 and 1000.0, got {}",
                self.low_freq
            ));
        }
        if self.low_gain < -24.0 || self.low_gain > 24.0 {
            return Err(format!(
                "Equalizer 'low_gain' must be between -24.0 and 24.0, got {}",
                self.low_gain
            ));
        }
        if self.mid_freq < 100.0 || self.mid_freq > 5000.0 {
            return Err(format!(
                "Equalizer 'mid_freq' must be between 100.0 and 5000.0, got {}",
                self.mid_freq
            ));
        }
        if self.mid_gain < -24.0 || self.mid_gain > 24.0 {
            return Err(format!(
                "Equalizer 'mid_gain' must be between -24.0 and 24.0, got {}",
                self.mid_gain
            ));
        }
        if self.mid_q < 0.1 || self.mid_q > 10.0 {
            return Err(format!(
                "Equalizer 'mid_q' must be between 0.1 and 10.0, got {}",
                self.mid_q
            ));
        }
        if self.high_freq < 1000.0 || self.high_freq > 20000.0 {
            return Err(format!(
                "Equalizer 'high_freq' must be between 1000.0 and 20000.0, got {}",
                self.high_freq
            ));
        }
        if self.high_gain < -24.0 || self.high_gain > 24.0 {
            return Err(format!(
                "Equalizer 'high_gain' must be between -24.0 and 24.0, got {}",
                self.high_gain
            ));
        }
        Ok(())
    }
}

impl From<EqualizerParams> for Equalizer {
    fn from(p: EqualizerParams) -> Self {
        Self {
            low_freq: AtomicF32::new(p.low_freq),
            low_gain: AtomicF32::new(p.low_gain),
            mid_freq: AtomicF32::new(p.mid_freq),
            mid_gain: AtomicF32::new(p.mid_gain),
            mid_q: AtomicF32::new(p.mid_q),
            high_freq: AtomicF32::new(p.high_freq),
            high_gain: AtomicF32::new(p.high_gain),
            filters: std::cell::UnsafeCell::new(
                [BiquadState {
                    x1: 0.,
                    x2: 0.,
                    y1: 0.,
                    y2: 0.,
                }; 6],
            ),
            sample_rate: std::cell::UnsafeCell::new(44100.0),
            cached_coeffs: std::cell::UnsafeCell::new([(0., 0., 0., 0., 0.); 3]),
            last_params: std::cell::UnsafeCell::new(None),
        }
    }
}

impl From<&Equalizer> for EqualizerParams {
    fn from(e: &Equalizer) -> Self {
        Self {
            low_freq: e.low_freq.load(Ordering::Relaxed),
            low_gain: e.low_gain.load(Ordering::Relaxed),
            mid_freq: e.mid_freq.load(Ordering::Relaxed),
            mid_gain: e.mid_gain.load(Ordering::Relaxed),
            mid_q: e.mid_q.load(Ordering::Relaxed),
            high_freq: e.high_freq.load(Ordering::Relaxed),
            high_gain: e.high_gain.load(Ordering::Relaxed),
        }
    }
}

impl Equalizer {
    fn calc_biquad(
        &self,
        filter_type: i32,
        freq: f32,
        gain_db: f32,
        q: f32,
        sr: f32,
    ) -> (f32, f32, f32, f32, f32) {
        let a = 10.0f32.powf(gain_db / 40.0);
        let w0 = 2.0 * std::f32::consts::PI * freq / sr;
        let alpha = w0.sin() / (2.0 * q);
        let cos_w0 = w0.cos();

        let (b0, b1, b2, a0, a1, a2);

        if filter_type == 0 {
            let sqrt_a = a.sqrt();
            b0 = a * ((a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha);
            b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0);
            b2 = a * ((a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha);
            a0 = (a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha;
            a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cos_w0);
            a2 = (a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha;
        } else if filter_type == 1 {
            b0 = 1.0 + alpha * a;
            b1 = -2.0 * cos_w0;
            b2 = 1.0 - alpha * a;
            a0 = 1.0 + alpha / a;
            a1 = -2.0 * cos_w0;
            a2 = 1.0 - alpha / a;
        } else {
            let sqrt_a = a.sqrt();
            b0 = a * ((a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha);
            b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0);
            b2 = a * ((a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha);
            a0 = (a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha;
            a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_w0);
            a2 = (a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha;
        }

        (b0 / a0, b1 / a0, b2 / a0, a1 / a0, a2 / a0)
    }

    fn run_biquad(s: &mut BiquadState, input: f32, c: (f32, f32, f32, f32, f32)) -> f32 {
        let (b0, b1, b2, a1, a2) = c;
        let out = b0 * input + b1 * s.x1 + b2 * s.x2 - a1 * s.y1 - a2 * s.y2;

        let out = if out.abs() < 1e-20 { 0.0 } else { out };

        s.x2 = s.x1;
        s.x1 = input;
        s.y2 = s.y1;
        s.y1 = out;
        out
    }
}

impl EffectImpl for Equalizer {
    fn process(&self, l: f32, r: f32) -> (f32, f32) {
        let sr = clamp_finite(
            unsafe { *self.sample_rate.get() },
            8000.0,
            192000.0,
            44100.0,
        );
        let filters = unsafe { &mut *self.filters.get() };
        let coeffs = unsafe { &mut *self.cached_coeffs.get() };
        let last = unsafe { &mut *self.last_params.get() };

        let nyquist = sr * 0.49;
        let lf_max = nyquist.min(1000.0).max(20.0);
        let mf_max = nyquist.min(5000.0).max(100.0);
        let hf_max = nyquist.max(1000.0);

        let lf = clamp_finite(self.low_freq.load(Ordering::Relaxed), 20.0, lf_max, 200.0);
        let lg = clamp_finite(self.low_gain.load(Ordering::Relaxed), -24.0, 24.0, 0.0);
        let mf = clamp_finite(self.mid_freq.load(Ordering::Relaxed), 100.0, mf_max, 1000.0);
        let mg = clamp_finite(self.mid_gain.load(Ordering::Relaxed), -24.0, 24.0, 0.0);
        let mq = clamp_finite(self.mid_q.load(Ordering::Relaxed), 0.1, 10.0, 1.0);
        let hf = clamp_finite(
            self.high_freq.load(Ordering::Relaxed),
            1000.0f32.min(hf_max),
            hf_max,
            4000.0f32.min(hf_max),
        );
        let hg = clamp_finite(self.high_gain.load(Ordering::Relaxed), -24.0, 24.0, 0.0);

        let current_params = (lf, lg, mf, mg, mq, hf, hg);

        if last.is_none() || *last != Some(current_params) {
            coeffs[0] = self.calc_biquad(0, lf, lg, 0.707, sr);
            coeffs[1] = self.calc_biquad(1, mf, mg, mq, sr);
            coeffs[2] = self.calc_biquad(2, hf, hg, 0.707, sr);
            *last = Some(current_params);
        }

        let mut out_l = l;
        out_l = Self::run_biquad(&mut filters[0], out_l, coeffs[0]);
        out_l = Self::run_biquad(&mut filters[2], out_l, coeffs[1]);
        out_l = Self::run_biquad(&mut filters[4], out_l, coeffs[2]);

        let mut out_r = r;
        out_r = Self::run_biquad(&mut filters[1], out_r, coeffs[0]);
        out_r = Self::run_biquad(&mut filters[3], out_r, coeffs[1]);
        out_r = Self::run_biquad(&mut filters[5], out_r, coeffs[2]);

        (out_l, out_r)
    }

    fn reset(&mut self, sr: f32) {
        *self.sample_rate.get_mut() = sr;
        *self.last_params.get_mut() = None;
        let f = self.filters.get_mut();
        for s in f.iter_mut() {
            s.x1 = 0.;
            s.x2 = 0.;
            s.y1 = 0.;
            s.y2 = 0.;
        }
    }
}

#[derive(Debug)]
struct Comb {
    buffer: Vec<f32>,
    index: usize,
    feedback: f32,
    damp1: f32,
    damp2: f32,
    buffer_len: usize,
    filter_store: f32,
}

impl Comb {
    fn new(len: usize) -> Self {
        Self {
            buffer: vec![0.0; len],
            index: 0,
            feedback: 0.5,
            damp1: 0.5,
            damp2: 0.5,
            buffer_len: len,
            filter_store: 0.0,
        }
    }

    fn set_feedback(&mut self, val: f32) {
        self.feedback = val;
    }

    fn set_damp(&mut self, val: f32) {
        self.damp1 = val;
        self.damp2 = 1.0 - val;
    }

    fn process(&mut self, input: f32) -> f32 {
        let output = self.buffer[self.index];
        self.filter_store = (output * self.damp2) + (self.filter_store * self.damp1);

        if self.filter_store.abs() < 1e-20 {
            self.filter_store = 0.0;
        }

        self.buffer[self.index] = input + (self.filter_store * self.feedback);

        self.index += 1;
        if self.index >= self.buffer_len {
            self.index = 0;
        }

        output
    }

    fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.filter_store = 0.0;
        self.index = 0;
    }
}

#[derive(Debug)]
struct AllPass {
    buffer: Vec<f32>,
    index: usize,
    feedback: f32,
    buffer_len: usize,
}

impl AllPass {
    fn new(len: usize) -> Self {
        Self {
            buffer: vec![0.0; len],
            index: 0,
            feedback: 0.5,
            buffer_len: len,
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        let buffered_val = self.buffer[self.index];
        let output = -input + buffered_val;
        self.buffer[self.index] = input + (buffered_val * self.feedback);

        self.index += 1;
        if self.index >= self.buffer_len {
            self.index = 0;
        }

        output
    }

    fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.index = 0;
    }
}

#[derive(Debug)]
pub struct Reverb {
    pub room_size: AtomicF32,
    pub damping: AtomicF32,
    pub width: AtomicF32,
    pub mix: AtomicF32,
    pub pre_delay_ms: AtomicF32,

    comb_l: std::cell::UnsafeCell<Vec<Comb>>,
    allpass_l: std::cell::UnsafeCell<Vec<AllPass>>,

    comb_r: std::cell::UnsafeCell<Vec<Comb>>,
    allpass_r: std::cell::UnsafeCell<Vec<AllPass>>,

    pre_delay_buffer_l: std::cell::UnsafeCell<Vec<f32>>,
    pre_delay_buffer_r: std::cell::UnsafeCell<Vec<f32>>,
    pre_delay_write: std::cell::UnsafeCell<usize>,

    sample_rate: std::cell::UnsafeCell<f32>,
}

unsafe impl Sync for Reverb {}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ReverbParams {
    pub room_size: f32,
    pub damping: f32,
    pub width: f32,
    pub mix: f32,
    pub pre_delay_ms: f32,
}

impl Default for ReverbParams {
    fn default() -> Self {
        Self {
            room_size: 0.5,
            damping: 0.5,
            width: 1.0,
            mix: 0.3,
            pre_delay_ms: 0.0,
        }
    }
}

impl ReverbParams {
    pub fn validate(&self) -> Result<(), String> {
        if self.room_size < 0.0 || self.room_size > 1.0 {
            return Err(format!(
                "Reverb room_size out of bounds: {}",
                self.room_size
            ));
        }
        if self.damping < 0.0 || self.damping > 1.0 {
            return Err(format!("Reverb damping out of bounds: {}", self.damping));
        }
        if self.width < 0.0 || self.width > 1.0 {
            return Err(format!("Reverb width out of bounds: {}", self.width));
        }
        if self.mix < 0.0 || self.mix > 1.0 {
            return Err(format!("Reverb mix out of bounds: {}", self.mix));
        }
        if self.pre_delay_ms < 0.0 || self.pre_delay_ms > 500.0 {
            return Err(format!(
                "Reverb pre_delay_ms out of bounds: {}",
                self.pre_delay_ms
            ));
        }
        Ok(())
    }
}

impl From<ReverbParams> for Reverb {
    fn from(p: ReverbParams) -> Self {
        let reverb = Self {
            room_size: AtomicF32::new(p.room_size),
            damping: AtomicF32::new(p.damping),
            width: AtomicF32::new(p.width),
            mix: AtomicF32::new(p.mix),
            pre_delay_ms: AtomicF32::new(p.pre_delay_ms),

            comb_l: std::cell::UnsafeCell::new(Vec::new()),
            allpass_l: std::cell::UnsafeCell::new(Vec::new()),
            comb_r: std::cell::UnsafeCell::new(Vec::new()),
            allpass_r: std::cell::UnsafeCell::new(Vec::new()),

            pre_delay_buffer_l: std::cell::UnsafeCell::new(vec![0.0; 24000]),
            pre_delay_buffer_r: std::cell::UnsafeCell::new(vec![0.0; 24000]),
            pre_delay_write: std::cell::UnsafeCell::new(0),

            sample_rate: std::cell::UnsafeCell::new(44100.0),
        };

        reverb.init_filters();
        reverb
    }
}

impl From<&Reverb> for ReverbParams {
    fn from(r: &Reverb) -> Self {
        Self {
            room_size: r.room_size.load(Ordering::Relaxed),
            damping: r.damping.load(Ordering::Relaxed),
            width: r.width.load(Ordering::Relaxed),
            mix: r.mix.load(Ordering::Relaxed),
            pre_delay_ms: r.pre_delay_ms.load(Ordering::Relaxed),
        }
    }
}

impl Reverb {
    fn init_filters(&self) {
        let comb_tunings = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
        let allpass_tunings = [225, 556, 441, 341];
        let stereo_spread = 23;

        let combs_l = unsafe { &mut *self.comb_l.get() };
        let combs_r = unsafe { &mut *self.comb_r.get() };
        let allpasses_l = unsafe { &mut *self.allpass_l.get() };
        let allpasses_r = unsafe { &mut *self.allpass_r.get() };

        combs_l.clear();
        combs_r.clear();
        allpasses_l.clear();
        allpasses_r.clear();

        for len in comb_tunings.iter() {
            combs_l.push(Comb::new(*len));
            combs_r.push(Comb::new(*len + stereo_spread));
        }

        for len in allpass_tunings.iter() {
            allpasses_l.push(AllPass::new(*len));
            allpasses_r.push(AllPass::new(*len + stereo_spread));
        }
    }

    fn update_params(&self) {
        let room_size = clamp_finite(self.room_size.load(Ordering::Relaxed), 0.0, 1.0, 0.5);
        let damping = clamp_finite(self.damping.load(Ordering::Relaxed), 0.0, 1.0, 0.5);

        let feedback = room_size * 0.28 + 0.7;
        let damp = damping * 0.4;

        let combs_l = unsafe { &mut *self.comb_l.get() };
        let combs_r = unsafe { &mut *self.comb_r.get() };

        for c in combs_l.iter_mut() {
            c.set_feedback(feedback);
            c.set_damp(damp);
        }
        for c in combs_r.iter_mut() {
            c.set_feedback(feedback);
            c.set_damp(damp);
        }
    }
}

impl EffectImpl for Reverb {
    fn process(&self, l: f32, r: f32) -> (f32, f32) {
        self.update_params();

        let pre_delay_ms = clamp_finite(self.pre_delay_ms.load(Ordering::Relaxed), 0.0, 500.0, 0.0);
        let sr = clamp_finite(
            unsafe { *self.sample_rate.get() },
            8000.0,
            192000.0,
            44100.0,
        );

        let pd_buf_l = unsafe { &mut *self.pre_delay_buffer_l.get() };
        let pd_buf_r = unsafe { &mut *self.pre_delay_buffer_r.get() };
        let pd_write = unsafe { &mut *self.pre_delay_write.get() };
        let max_delay = pd_buf_l.len().saturating_sub(1);
        let delay_samples = ((pre_delay_ms * 0.001 * sr).round() as usize).min(max_delay);

        pd_buf_l[*pd_write] = l;
        pd_buf_r[*pd_write] = r;

        let mut read_idx = *pd_write as isize - delay_samples as isize;
        while read_idx < 0 {
            read_idx += pd_buf_l.len() as isize;
        }
        let read_idx = read_idx as usize;
        let in_l = pd_buf_l[read_idx];
        let in_r = pd_buf_r[read_idx];

        *pd_write += 1;
        if *pd_write >= pd_buf_l.len() {
            *pd_write = 0;
        }

        let gain = 0.015;
        let input_l = in_l * gain;
        let input_r = in_r * gain;

        let combs_l = unsafe { &mut *self.comb_l.get() };
        let combs_r = unsafe { &mut *self.comb_r.get() };
        let allpasses_l = unsafe { &mut *self.allpass_l.get() };
        let allpasses_r = unsafe { &mut *self.allpass_r.get() };

        let mut out_l = 0.0;
        let mut out_r = 0.0;

        for comb in combs_l.iter_mut() {
            out_l += comb.process(input_l);
        }
        for comb in combs_r.iter_mut() {
            out_r += comb.process(input_r);
        }

        for ap in allpasses_l.iter_mut() {
            out_l = ap.process(out_l);
        }
        for ap in allpasses_r.iter_mut() {
            out_r = ap.process(out_r);
        }

        let mix = clamp_finite(self.mix.load(Ordering::Relaxed), 0.0, 1.0, 0.3);
        let width = clamp_finite(self.width.load(Ordering::Relaxed), 0.0, 1.0, 1.0);

        let wet_l = out_l * (1.0 + width) + out_r * (1.0 - width);
        let wet_r = out_r * (1.0 + width) + out_l * (1.0 - width);

        let final_l = l * (1.0 - mix) + wet_l * mix;
        let final_r = r * (1.0 - mix) + wet_r * mix;

        (final_l, final_r)
    }

    fn reset(&mut self, sr: f32) {
        *self.sample_rate.get_mut() = sr;
        *self.pre_delay_write.get_mut() = 0;
        self.pre_delay_buffer_l.get_mut().fill(0.0);
        self.pre_delay_buffer_r.get_mut().fill(0.0);

        let combs_l = self.comb_l.get_mut();
        for c in combs_l.iter_mut() {
            c.reset();
        }

        let combs_r = self.comb_r.get_mut();
        for c in combs_r.iter_mut() {
            c.reset();
        }

        let allpasses_l = self.allpass_l.get_mut();
        for ap in allpasses_l.iter_mut() {
            ap.reset();
        }

        let allpasses_r = self.allpass_r.get_mut();
        for ap in allpasses_r.iter_mut() {
            ap.reset();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_finite_pair(idx: usize, l: f32, r: f32, label: &str) {
        assert!(l.is_finite(), "{label}: left became non-finite at {idx}");
        assert!(r.is_finite(), "{label}: right became non-finite at {idx}");
    }

    #[test]
    fn clamp_finite_uses_fallback_for_nan_and_clamps_range() {
        assert_eq!(clamp_finite(f32::NAN, 0.0, 1.0, 0.5), 0.5);
        assert_eq!(clamp_finite(2.0, 0.0, 1.0, 0.5), 1.0);
        assert_eq!(clamp_finite(-2.0, 0.0, 1.0, 0.5), 0.0);
        assert_eq!(clamp_finite(0.25, 0.0, 1.0, 0.5), 0.25);
    }

    #[test]
    fn validates_overdrive_bounds() {
        assert!(OverdriveParams::default().validate().is_ok());
        assert!(OverdriveParams {
            drive: 1.5,
            ..OverdriveParams::default()
        }
        .validate()
        .is_err());
        assert!(OverdriveParams {
            mix: -0.1,
            ..OverdriveParams::default()
        }
        .validate()
        .is_err());
    }

    #[test]
    fn validates_delay_bounds() {
        assert!(DelayParams::default().validate().is_ok());
        assert!(DelayParams {
            time_ms: 5.0,
            ..DelayParams::default()
        }
        .validate()
        .is_err());
        assert!(DelayParams {
            feedback: 1.0,
            ..DelayParams::default()
        }
        .validate()
        .is_err());
    }

    #[test]
    fn validates_reverb_bounds() {
        assert!(ReverbParams::default().validate().is_ok());
        assert!(ReverbParams {
            room_size: 2.0,
            ..ReverbParams::default()
        }
        .validate()
        .is_err());
        assert!(ReverbParams {
            pre_delay_ms: 600.0,
            ..ReverbParams::default()
        }
        .validate()
        .is_err());
    }

    #[test]
    fn validates_noise_gate_bounds() {
        assert!(NoiseGateParams::default().validate().is_ok());
        assert!(NoiseGateParams {
            ratio: 0.0,
            ..NoiseGateParams::default()
        }
        .validate()
        .is_err());
        assert!(NoiseGateParams {
            attack_ms: 0.01,
            ..NoiseGateParams::default()
        }
        .validate()
        .is_err());
    }

    #[test]
    fn validates_three_band_eq_bounds() {
        assert!(EqualizerParams::default().validate().is_ok());
        assert!(EqualizerParams {
            low_freq: 5.0,
            ..EqualizerParams::default()
        }
        .validate()
        .is_err());
        assert!(EqualizerParams {
            mid_q: 0.0,
            ..EqualizerParams::default()
        }
        .validate()
        .is_err());
    }

    #[test]
    fn overdrive_mix_zero_is_dry_passthrough() {
        let overdrive = Overdrive::from(OverdriveParams {
            drive: 1.0,
            mix: 0.0,
            output_gain: 1.0,
        });

        let input_l = 0.37;
        let input_r = -0.29;
        let (out_l, out_r) = overdrive.process(input_l, input_r);
        assert!((out_l - input_l).abs() < 1e-6);
        assert!((out_r - input_r).abs() < 1e-6);
    }

    #[test]
    fn delay_mix_zero_outputs_dry_signal() {
        let mut delay = Delay::from(DelayParams {
            time_ms: 250.0,
            feedback: 0.99,
            mix: 0.0,
        });
        delay.reset(44100.0);

        for i in 0..2000usize {
            let input_l = if i % 137 == 0 {
                0.9
            } else {
                (i as f32 * 0.1).sin() * 0.2
            };
            let input_r = -input_l;
            let (out_l, out_r) = delay.process(input_l, input_r);
            assert!((out_l - input_l).abs() < 1e-5);
            assert!((out_r - input_r).abs() < 1e-5);
        }
    }

    #[test]
    fn delay_reset_clears_existing_tail() {
        let mut delay = Delay::from(DelayParams {
            time_ms: 500.0,
            feedback: 0.7,
            mix: 1.0,
        });
        delay.reset(44100.0);

        let _ = delay.process(1.0, 1.0);
        for _ in 0..1000 {
            let _ = delay.process(0.0, 0.0);
        }

        delay.reset(44100.0);
        let (l, r) = delay.process(0.0, 0.0);
        assert!(l.abs() < 1e-6, "delay left tail not cleared after reset");
        assert!(r.abs() < 1e-6, "delay right tail not cleared after reset");
    }

    #[test]
    fn three_band_eq_zero_gains_is_near_passthrough() {
        let mut eq = Equalizer::from(EqualizerParams {
            low_freq: 120.0,
            low_gain: 0.0,
            mid_freq: 1200.0,
            mid_gain: 0.0,
            mid_q: 1.0,
            high_freq: 5000.0,
            high_gain: 0.0,
        });
        eq.reset(48000.0);

        for i in 0..5000usize {
            let input = (i as f32 * 0.013).sin() * 0.8;
            let (l, r) = eq.process(input, input);
            assert_finite_pair(i, l, r, "three_band_eq_passthrough");
            assert!(
                (l - input).abs() < 1e-3,
                "left eq deviated too much at sample {i}"
            );
            assert!(
                (r - input).abs() < 1e-3,
                "right eq deviated too much at sample {i}"
            );
        }
    }

    #[test]
    fn reverb_mix_zero_outputs_dry_signal() {
        let mut reverb = Reverb::from(ReverbParams {
            room_size: 0.8,
            damping: 0.5,
            width: 1.0,
            mix: 0.0,
            pre_delay_ms: 30.0,
        });
        reverb.reset(44100.0);

        for i in 0..2000usize {
            let input_l = (i as f32 * 0.02).sin() * 0.5;
            let input_r = (i as f32 * 0.031).cos() * 0.5;
            let (out_l, out_r) = reverb.process(input_l, input_r);
            assert!((out_l - input_l).abs() < 1e-6);
            assert!((out_r - input_r).abs() < 1e-6);
        }
    }

    #[test]
    fn reverb_reset_clears_tail() {
        let mut reverb = Reverb::from(ReverbParams {
            room_size: 1.0,
            damping: 0.2,
            width: 1.0,
            mix: 1.0,
            pre_delay_ms: 0.0,
        });
        reverb.reset(44100.0);

        let _ = reverb.process(1.0, 1.0);
        for _ in 0..4000 {
            let _ = reverb.process(0.0, 0.0);
        }

        reverb.reset(44100.0);
        let (l, r) = reverb.process(0.0, 0.0);
        assert!(l.abs() < 1e-5, "reverb left tail not cleared after reset");
        assert!(r.abs() < 1e-5, "reverb right tail not cleared after reset");
    }

    #[test]
    fn overdrive_stays_finite_under_extreme_valid_params() {
        let overdrive = Overdrive::from(OverdriveParams {
            drive: 1.0,
            mix: 1.0,
            output_gain: 2.0,
        });

        for i in 0..20000usize {
            let x = (i as f32 * 0.017).sin();
            let (l, r) = overdrive.process(x, -x);
            assert_finite_pair(i, l, r, "overdrive");
            assert!(l.abs() <= 2.1);
            assert!(r.abs() <= 2.1);
        }
    }

    #[test]
    fn delay_stays_finite_under_extreme_valid_params() {
        let mut delay = Delay::from(DelayParams {
            time_ms: 4000.0,
            feedback: 0.99,
            mix: 1.0,
        });
        delay.reset(48000.0);

        for i in 0..50000usize {
            let input = if i == 0 { 1.0 } else { 0.0 };
            let (l, r) = delay.process(input, input);
            assert_finite_pair(i, l, r, "delay");
            assert!(l.abs() < 50.0, "delay L unstable at {i}: {l}");
            assert!(r.abs() < 50.0, "delay R unstable at {i}: {r}");
        }
    }

    #[test]
    fn noise_gate_clamps_invalid_runtime_params() {
        let gate = NoiseGate::from(NoiseGateParams::default());

        gate.ratio.store(0.0, Ordering::Relaxed);
        gate.attack_ms.store(f32::NAN, Ordering::Relaxed);
        gate.release_ms.store(-20.0, Ordering::Relaxed);
        gate.threshold_db.store(5.0, Ordering::Relaxed);

        for i in 0..10000usize {
            let x = ((i as f32) * 0.03).sin() * 0.6;
            let (l, r) = gate.process(x, -x);
            assert_finite_pair(i, l, r, "noise_gate");
            assert!(l.abs() < 10.0);
            assert!(r.abs() < 10.0);
        }
    }

    #[test]
    fn three_band_eq_clamps_invalid_runtime_params() {
        let mut eq = Equalizer::from(EqualizerParams::default());
        eq.reset(44100.0);

        eq.low_freq.store(-100.0, Ordering::Relaxed);
        eq.mid_q.store(0.0, Ordering::Relaxed);
        eq.high_freq.store(1_000_000.0, Ordering::Relaxed);
        eq.low_gain.store(f32::NAN, Ordering::Relaxed);

        for i in 0..20000usize {
            let x = ((i as f32) * 0.02).sin() * 0.7;
            let (l, r) = eq.process(x, x);
            assert_finite_pair(i, l, r, "three_band_eq");
            assert!(l.abs() < 20.0);
            assert!(r.abs() < 20.0);
        }
    }

    #[test]
    fn reverb_clamps_invalid_runtime_params() {
        let mut reverb = Reverb::from(ReverbParams::default());
        reverb.reset(44100.0);

        reverb.room_size.store(f32::NAN, Ordering::Relaxed);
        reverb.damping.store(-10.0, Ordering::Relaxed);
        reverb.width.store(3.0, Ordering::Relaxed);
        reverb.mix.store(2.0, Ordering::Relaxed);
        reverb.pre_delay_ms.store(10_000.0, Ordering::Relaxed);

        for i in 0..20000usize {
            let input = if i % 1024 == 0 { 0.7 } else { 0.0 };
            let (l, r) = reverb.process(input, input);
            assert_finite_pair(i, l, r, "reverb");
            assert!(l.abs() < 50.0);
            assert!(r.abs() < 50.0);
        }
    }

    #[test]
    fn test_reverb_instantiation_and_process() {
        let mut reverb = Reverb::from(ReverbParams::default());
        reverb.reset(44100.0);

        let (out_l, out_r) = reverb.process(1.0, 1.0);

        assert!(out_l > 0.6);
        assert!(out_r > 0.6);

        let mut max_out = 0.0;
        for _ in 0..10000 {
            let (l, r) = reverb.process(0.0, 0.0);
            if l.abs() > max_out {
                max_out = l.abs();
            }
            if r.abs() > max_out {
                max_out = r.abs();
            }
        }

        assert!(max_out.is_finite());

        assert!(max_out > 0.0);
    }
}
