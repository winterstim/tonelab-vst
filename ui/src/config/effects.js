export const EFFECTS_METADATA = {
    Overdrive: {
        id: "Overdrive",
        label: "Overdrive",
        params: {
            drive: {
                id: "drive",
                label: "Drive",
                min: 0.0,
                max: 1.0,
                default: 0.5,
                step: 0.01
            },
            mix: {
                id: "mix",
                label: "Mix",
                min: 0.0,
                max: 1.0,
                default: 1.0,
                step: 0.01
            },
            output_gain: {
                id: "output_gain",
                label: "Out Gain",
                min: 0.0,
                max: 2.0,
                default: 1.0,
                step: 0.01
            }
        }
    },
    Delay: {
        id: "Delay",
        label: "Delay",
        params: {
            time_ms: {
                id: "time_ms",
                label: "Time (ms)",
                min: 10.0,
                max: 4000.0,
                default: 250.0,
                step: 1.0
            },
            feedback: {
                id: "feedback",
                label: "Feedback",
                min: 0.0,
                max: 0.99,
                default: 0.3,
                step: 0.01
            },
            mix: {
                id: "mix",
                label: "Mix",
                min: 0.0,
                max: 1.0,
                default: 0.5,
                step: 0.01
            }
        }
    },
    NoiseGate: {
        id: "NoiseGate",
        label: "Noise Gate",
        params: {
            threshold_db: {
                id: "threshold_db",
                label: "Threshold (dB)",
                min: -100.0,
                max: 0.0,
                default: -40.0,
                step: 0.1
            },
            ratio: {
                id: "ratio",
                label: "Ratio",
                min: 1.0,
                max: 100.0,
                default: 10.0,
                step: 0.1
            },
            attack_ms: {
                id: "attack_ms",
                label: "Attack (ms)",
                min: 0.1,
                max: 100.0,
                default: 2.0,
                step: 0.1
            },
            release_ms: {
                id: "release_ms",
                label: "Release (ms)",
                min: 10.0,
                max: 1000.0,
                default: 100.0,
                step: 1.0
            }
        }
    },
    Equalizer: {
        id: "Equalizer",
        label: "Equalizer",
        params: {
            low_freq: {
                id: "low_freq",
                label: "Low Freq",
                min: 20.0,
                max: 1000.0,
                default: 100.0,
                step: 1.0
            },
            low_gain: {
                id: "low_gain",
                label: "Low Gain",
                min: -24.0,
                max: 24.0,
                default: 0.0,
                step: 0.1
            },
            mid_freq: {
                id: "mid_freq",
                label: "Mid Freq",
                min: 100.0,
                max: 5000.0,
                default: 1000.0,
                step: 10.0
            },
            mid_gain: {
                id: "mid_gain",
                label: "Mid Gain",
                min: -24.0,
                max: 24.0,
                default: 0.0,
                step: 0.1
            },
            mid_q: {
                id: "mid_q",
                label: "Mid Q",
                min: 0.1,
                max: 10.0,
                default: 1.0,
                step: 0.1
            },
            high_freq: {
                id: "high_freq",
                label: "High Freq",
                min: 1000.0,
                max: 20000.0,
                default: 5000.0,
                step: 10.0
            },
            high_gain: {
                id: "high_gain",
                label: "High Gain",
                min: -24.0,
                max: 24.0,
                default: 0.0,
                step: 0.1
            }
        }
    },
    Reverb: {
        id: "Reverb",
        label: "Reverb",
        params: {
            room_size: {
                id: "room_size",
                label: "Size",
                min: 0.0,
                max: 1.0,
                default: 0.5,
                step: 0.01
            },
            damping: {
                id: "damping",
                label: "Damping",
                min: 0.0,
                max: 1.0,
                default: 0.5,
                step: 0.01
            },
            width: {
                id: "width",
                label: "Width",
                min: 0.0,
                max: 1.0,
                default: 1.0,
                step: 0.01
            },
            mix: {
                id: "mix",
                label: "Mix",
                min: 0.0,
                max: 1.0,
                default: 0.3,
                step: 0.01
            },
            pre_delay_ms: {
                id: "pre_delay_ms",
                label: "Pre-Delay",
                min: 0.0,
                max: 200.0,
                default: 0.0,
                step: 1.0
            }
        }
    }
};


export const validateParam = (effectType, paramId, value) => {
    const effect = EFFECTS_METADATA[effectType];
    if (!effect) return false;

    const paramConfig = effect.params[paramId];
    if (!paramConfig) return false;


    if (value < paramConfig.min || value > paramConfig.max) {
        return false;
    }
    return true;
};
