pub mod effects;
use effects::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", content = "params")]
pub enum AudioEffectParams {
    Overdrive(effects::OverdriveParams),
    Delay(effects::DelayParams),
    NoiseGate(effects::NoiseGateParams),
    Equalizer(effects::EqualizerParams),
    Reverb(effects::ReverbParams),
}

#[derive(Debug)]
pub enum AudioEffect {
    Overdrive(Arc<effects::Overdrive>),
    Delay(Arc<effects::Delay>),
    NoiseGate(Arc<effects::NoiseGate>),
    Equalizer(Arc<effects::Equalizer>),
    Reverb(Arc<effects::Reverb>),
}

impl AudioEffect {
    pub fn from_params(p: AudioEffectParams) -> Self {
        match p {
            AudioEffectParams::Overdrive(d) => AudioEffect::Overdrive(Arc::new(d.into())),
            AudioEffectParams::Delay(d) => AudioEffect::Delay(Arc::new(d.into())),
            AudioEffectParams::NoiseGate(d) => AudioEffect::NoiseGate(Arc::new(d.into())),
            AudioEffectParams::Equalizer(d) => AudioEffect::Equalizer(Arc::new(d.into())),
            AudioEffectParams::Reverb(d) => AudioEffect::Reverb(Arc::new(d.into())),
        }
    }

    pub fn to_params(&self) -> AudioEffectParams {
        match self {
            AudioEffect::Overdrive(d) => AudioEffectParams::Overdrive(d.as_ref().into()),
            AudioEffect::Delay(d) => AudioEffectParams::Delay(d.as_ref().into()),
            AudioEffect::NoiseGate(d) => AudioEffectParams::NoiseGate(d.as_ref().into()),
            AudioEffect::Equalizer(d) => AudioEffectParams::Equalizer(d.as_ref().into()),
            AudioEffect::Reverb(d) => AudioEffectParams::Reverb(d.as_ref().into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full_chain_json() -> String {
        serde_json::json!([
            {
                "type": "Overdrive",
                "params": { "drive": 0.35, "mix": 0.9, "output_gain": 1.0 }
            },
            {
                "type": "Delay",
                "params": { "time_ms": 300.0, "feedback": 0.4, "mix": 0.25 }
            },
            {
                "type": "NoiseGate",
                "params": { "threshold_db": -40.0, "ratio": 10.0, "attack_ms": 2.0, "release_ms": 120.0 }
            },
            {
                "type": "Equalizer",
                "params": {
                    "low_freq": 100.0,
                    "low_gain": 2.0,
                    "mid_freq": 1200.0,
                    "mid_gain": -1.5,
                    "mid_q": 1.1,
                    "high_freq": 6000.0,
                    "high_gain": 1.0
                }
            },
            {
                "type": "Reverb",
                "params": { "room_size": 0.45, "damping": 0.4, "width": 1.0, "mix": 0.2, "pre_delay_ms": 10.0 }
            }
        ])
        .to_string()
    }

    #[test]
    fn chain_rejects_invalid_params() {
        let invalid = serde_json::json!([
            {
                "type": "Delay",
                "params": { "time_ms": 100.0, "feedback": 1.2, "mix": 0.5 }
            }
        ])
        .to_string();

        let err = Chain::from_json(&invalid).expect_err("invalid params should fail");
        assert!(err.contains("feedback"));
    }

    #[test]
    fn chain_rejects_malformed_json() {
        let malformed = r#"[{ "type": "Delay", "params": { "time_ms": 100.0 }"#;
        let err = Chain::from_json(malformed).expect_err("malformed json should fail");
        assert!(err.contains("JSON Parsing Error"));
    }

    #[test]
    fn chain_rejects_unknown_effect_type() {
        let unknown = serde_json::json!([
            {
                "type": "NotARealEffect",
                "params": { "value": 1.0 }
            }
        ])
        .to_string();

        let err = Chain::from_json(&unknown).expect_err("unknown effect type should fail");
        assert!(err.contains("unknown variant") || err.contains("NotARealEffect"));
    }

    #[test]
    fn chain_parses_empty_json_array() {
        let chain = Chain::from_json("[]").expect("empty array should parse as empty chain");
        assert_eq!(chain.effects.len(), 0);
        let (l, r) = chain.process(0.2, -0.4);
        assert_eq!(l, 0.2);
        assert_eq!(r, -0.4);
    }

    #[test]
    fn chain_roundtrip_preserves_effect_count() {
        let json = full_chain_json();
        let chain = Chain::from_json(&json).expect("input JSON should parse");
        assert_eq!(chain.effects.len(), 5);

        let serialized = chain.to_json().expect("serialization should succeed");
        let reparsed = Chain::from_json(&serialized).expect("roundtrip JSON should parse");
        assert_eq!(reparsed.effects.len(), chain.effects.len());
    }

    #[test]
    fn chain_set_param_changes_processing_result() {
        let json = serde_json::json!([
            {
                "type": "Overdrive",
                "params": { "drive": 0.0, "mix": 1.0, "output_gain": 1.0 }
            }
        ])
        .to_string();

        let chain = Chain::from_json(&json).expect("chain should parse");
        let baseline = chain.process(0.5, 0.5);
        chain.set_param(0, "drive", 1.0);
        let updated = chain.process(0.5, 0.5);

        assert!(updated.0.is_finite() && updated.1.is_finite());
        assert!(baseline != updated, "parameter update should affect output");
    }

    #[test]
    fn chain_set_param_ignores_unknown_key_without_panicking() {
        let json = serde_json::json!([
            {
                "type": "Overdrive",
                "params": { "drive": 0.2, "mix": 0.7, "output_gain": 1.0 }
            }
        ])
        .to_string();
        let chain = Chain::from_json(&json).expect("chain should parse");

        let before = chain.process(0.3, -0.3);
        chain.set_param(0, "definitely_unknown_param", 999.0);
        let after = chain.process(0.3, -0.3);
        assert_eq!(before, after);
    }

    #[test]
    fn chain_set_param_out_of_bounds_index_is_noop() {
        let json = serde_json::json!([
            {
                "type": "Overdrive",
                "params": { "drive": 0.8, "mix": 1.0, "output_gain": 1.0 }
            }
        ])
        .to_string();
        let chain = Chain::from_json(&json).expect("chain should parse");

        let before = chain.process(0.25, 0.25);
        chain.set_param(999, "drive", 0.0);
        let after = chain.process(0.25, 0.25);
        assert_eq!(before, after);
    }

    #[test]
    fn chain_stress_process_stays_finite() {
        let mut chain = Chain::from_json(&full_chain_json()).expect("full chain should parse");
        chain.reset(48000.0);

        for i in 0..20000usize {
            if i % 257 == 0 {
                let t = (i as f32 * 0.001).sin() * 0.5 + 0.5;
                chain.set_param(0, "drive", t);
                chain.set_param(1, "feedback", t.min(0.99));
                chain.set_param(2, "ratio", 1.0 + t * 40.0);
                chain.set_param(3, "mid_q", 0.1 + t * 9.0);
                chain.set_param(4, "mix", t);
            }

            let phase = i as f32 * 0.015;
            let input_l = phase.sin() * 0.8;
            let input_r = phase.cos() * 0.8;
            let (out_l, out_r) = chain.process(input_l, input_r);

            assert!(
                out_l.is_finite(),
                "left channel became non-finite at sample {i}"
            );
            assert!(
                out_r.is_finite(),
                "right channel became non-finite at sample {i}"
            );
            assert!(
                out_l.abs() < 100.0,
                "left channel exploded at sample {i}: {out_l}"
            );
            assert!(
                out_r.abs() < 100.0,
                "right channel exploded at sample {i}: {out_r}"
            );
        }
    }

    #[test]
    fn chain_randomized_parameter_updates_stay_finite() {
        let mut chain = Chain::from_json(&full_chain_json()).expect("full chain should parse");
        chain.reset(44100.0);

        let mut seed: u64 = 0xDEADBEEFCAFEBABE;
        let mut next = || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            ((seed >> 33) as f32) / ((1u64 << 31) as f32)
        };

        for i in 0..15000usize {
            if i % 97 == 0 {
                chain.set_param(0, "drive", next().fract().abs());
                chain.set_param(1, "feedback", (next().fract().abs() * 0.99).min(0.99));
                chain.set_param(2, "ratio", 1.0 + next().fract().abs() * 80.0);
                chain.set_param(3, "mid_q", 0.1 + next().fract().abs() * 9.9);
                chain.set_param(4, "mix", next().fract().abs());
            }

            let t = i as f32 * 0.019;
            let (out_l, out_r) = chain.process(t.sin() * 0.9, t.cos() * 0.9);
            assert!(out_l.is_finite(), "left non-finite at sample {i}");
            assert!(out_r.is_finite(), "right non-finite at sample {i}");
            assert!(out_l.abs() < 150.0, "left exploded at sample {i}: {out_l}");
            assert!(out_r.abs() < 150.0, "right exploded at sample {i}: {out_r}");
        }
    }

    #[test]
    fn empty_chain_is_passthrough() {
        let chain = Chain::new();
        let (l, r) = chain.process(-0.25, 0.8);
        assert_eq!(l, -0.25);
        assert_eq!(r, 0.8);
    }
}

impl EffectImpl for AudioEffect {
    fn process(&self, l: f32, r: f32) -> (f32, f32) {
        match self {
            AudioEffect::Overdrive(e) => e.process(l, r),
            AudioEffect::Delay(e) => e.process(l, r),
            AudioEffect::NoiseGate(e) => e.process(l, r),
            AudioEffect::Equalizer(e) => e.process(l, r),
            AudioEffect::Reverb(e) => e.process(l, r),
        }
    }

    fn reset(&mut self, sample_rate: f32) {
        match self {
            AudioEffect::Overdrive(e) => {
                if let Some(m) = Arc::get_mut(e) {
                    m.reset(sample_rate);
                }
            }
            AudioEffect::Delay(e) => {
                if let Some(m) = Arc::get_mut(e) {
                    m.reset(sample_rate);
                }
            }
            AudioEffect::NoiseGate(e) => {
                if let Some(m) = Arc::get_mut(e) {
                    m.reset(sample_rate);
                }
            }
            AudioEffect::Equalizer(e) => {
                if let Some(m) = Arc::get_mut(e) {
                    m.reset(sample_rate);
                }
            }
            AudioEffect::Reverb(e) => {
                if let Some(m) = Arc::get_mut(e) {
                    m.reset(sample_rate);
                }
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
struct ChainState {
    effects: Vec<AudioEffectParams>,
    #[serde(default)]
    token: Option<String>,
}

#[derive(Default, Debug)]
pub struct Chain {
    pub effects: Vec<AudioEffect>,
    pub auth_token: Option<String>,
}

impl Chain {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_json(json: &str) -> Result<Self, String> {
        // Try parsing as new object format first
        if let Ok(state) = serde_json::from_str::<ChainState>(json) {
            let mut effects = Vec::new();
            for params in state.effects {
                match &params {
                    AudioEffectParams::Overdrive(p) => p
                        .validate()
                        .map_err(|e| format!("Validation Error: {}", e))?,
                    AudioEffectParams::Delay(p) => p
                        .validate()
                        .map_err(|e| format!("Validation Error: {}", e))?,
                    AudioEffectParams::NoiseGate(p) => p
                        .validate()
                        .map_err(|e| format!("Validation Error: {}", e))?,
                    AudioEffectParams::Equalizer(p) => p
                        .validate()
                        .map_err(|e| format!("Validation Error: {}", e))?,
                    AudioEffectParams::Reverb(p) => p
                        .validate()
                        .map_err(|e| format!("Validation Error: {}", e))?,
                }
                effects.push(AudioEffect::from_params(params));
            }
            return Ok(Chain {
                effects,
                auth_token: state.token,
            });
        }

        // Fallback to array (legacy format)
        let params_list: Vec<AudioEffectParams> =
            serde_json::from_str(json).map_err(|e| format!("JSON Parsing Error: {}", e))?;

        for params in &params_list {
            match params {
                AudioEffectParams::Overdrive(p) => p
                    .validate()
                    .map_err(|e| format!("Validation Error: {}", e))?,
                AudioEffectParams::Delay(p) => p
                    .validate()
                    .map_err(|e| format!("Validation Error: {}", e))?,
                AudioEffectParams::NoiseGate(p) => p
                    .validate()
                    .map_err(|e| format!("Validation Error: {}", e))?,
                AudioEffectParams::Equalizer(p) => p
                    .validate()
                    .map_err(|e| format!("Validation Error: {}", e))?,
                AudioEffectParams::Reverb(p) => p
                    .validate()
                    .map_err(|e| format!("Validation Error: {}", e))?,
            }
        }

        let effects = params_list
            .into_iter()
            .map(AudioEffect::from_params)
            .collect();
        Ok(Chain {
            effects,
            auth_token: None,
        })
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        let params_list: Vec<AudioEffectParams> =
            self.effects.iter().map(|e| e.to_params()).collect();

        let state = ChainState {
            effects: params_list,
            token: self.auth_token.clone(),
        };

        serde_json::to_string(&state)
    }

    pub fn process(&self, l: f32, r: f32) -> (f32, f32) {
        let mut curr_l = l;
        let mut curr_r = r;

        for effect in &self.effects {
            let (next_l, next_r) = effect.process(curr_l, curr_r);
            curr_l = next_l;
            curr_r = next_r;
        }

        (curr_l, curr_r)
    }

    pub fn reset(&mut self, sample_rate: f32) {
        for effect in &mut self.effects {
            effect.reset(sample_rate);
        }
    }

    pub fn set_param(&self, index: usize, key: &str, value: f32) {
        if let Some(effect) = self.effects.get(index) {
            effect.set_param(key, value);
        }
    }
}

impl AudioEffect {
    pub fn set_param(&self, key: &str, value: f32) {
        match self {
            AudioEffect::Overdrive(e) => match key {
                "drive" => e.drive.store(value, std::sync::atomic::Ordering::Relaxed),
                "mix" => e.mix.store(value, std::sync::atomic::Ordering::Relaxed),
                "output_gain" => e
                    .output_gain
                    .store(value, std::sync::atomic::Ordering::Relaxed),
                _ => {}
            },
            AudioEffect::Delay(e) => match key {
                "time_ms" => e.time_ms.store(value, std::sync::atomic::Ordering::Relaxed),
                "feedback" => e
                    .feedback
                    .store(value, std::sync::atomic::Ordering::Relaxed),
                "mix" => e.mix.store(value, std::sync::atomic::Ordering::Relaxed),
                _ => {}
            },
            AudioEffect::NoiseGate(e) => match key {
                "threshold_db" => e
                    .threshold_db
                    .store(value, std::sync::atomic::Ordering::Relaxed),
                "ratio" => e.ratio.store(value, std::sync::atomic::Ordering::Relaxed),
                "attack_ms" => e
                    .attack_ms
                    .store(value, std::sync::atomic::Ordering::Relaxed),
                "release_ms" => e
                    .release_ms
                    .store(value, std::sync::atomic::Ordering::Relaxed),
                _ => {}
            },
            AudioEffect::Equalizer(e) => match key {
                "low_freq" => e
                    .low_freq
                    .store(value, std::sync::atomic::Ordering::Relaxed),
                "low_gain" => e
                    .low_gain
                    .store(value, std::sync::atomic::Ordering::Relaxed),
                "mid_freq" => e
                    .mid_freq
                    .store(value, std::sync::atomic::Ordering::Relaxed),
                "mid_gain" => e
                    .mid_gain
                    .store(value, std::sync::atomic::Ordering::Relaxed),
                "mid_q" => e.mid_q.store(value, std::sync::atomic::Ordering::Relaxed),
                "high_freq" => e
                    .high_freq
                    .store(value, std::sync::atomic::Ordering::Relaxed),
                "high_gain" => e
                    .high_gain
                    .store(value, std::sync::atomic::Ordering::Relaxed),
                _ => {}
            },
            AudioEffect::Reverb(e) => match key {
                "room_size" => e
                    .room_size
                    .store(value, std::sync::atomic::Ordering::Relaxed),
                "damping" => e.damping.store(value, std::sync::atomic::Ordering::Relaxed),
                "width" => e.width.store(value, std::sync::atomic::Ordering::Relaxed),
                "mix" => e.mix.store(value, std::sync::atomic::Ordering::Relaxed),
                "pre_delay_ms" => e
                    .pre_delay_ms
                    .store(value, std::sync::atomic::Ordering::Relaxed),
                _ => {}
            },
        }
    }
}
