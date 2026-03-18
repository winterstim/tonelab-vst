#![allow(dead_code)]

use serde::Deserialize;
use std::cell::RefCell;

mod dsp_core;

use dsp_core::Chain;

thread_local! {
    static ENGINE: RefCell<EngineState> = RefCell::new(EngineState::new());
}

#[derive(Debug, Deserialize)]
struct ParamChange {
    index: i32,
    param_key: String,
    value: f32,
}

struct EngineState {
    chain: Chain,
    sample_rate: f32,
}

impl EngineState {
    fn new() -> Self {
        let mut chain = Chain::new();
        let sample_rate = 44_100.0;
        chain.reset(sample_rate);
        Self { chain, sample_rate }
    }

    fn process(&mut self, input: &[f32], output: &mut [f32], frames: usize) {
        for frame in 0..frames {
            let i = frame * 2;
            let in_l = input[i];
            let in_r = input[i + 1];
            let (out_l, out_r) = self.chain.process(in_l, in_r);
            output[i] = out_l;
            output[i + 1] = out_r;
        }
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate.clamp(8_000.0, 192_000.0);
        self.chain.reset(self.sample_rate);
    }

    fn set_chain_json(&mut self, chain_json: &str) -> Result<(), String> {
        let mut chain = Chain::from_json(chain_json)?;
        chain.reset(self.sample_rate);
        self.chain = chain;
        Ok(())
    }

    fn set_param(&self, index: usize, key: &str, value: f32) {
        self.chain.set_param(index, key, value);
    }
}

#[no_mangle]
pub extern "C" fn alloc(size: i32) -> i32 {
    if size <= 0 {
        return 0;
    }
    let mut buffer = Vec::<f32>::with_capacity(size as usize);
    let ptr = buffer.as_mut_ptr();
    std::mem::forget(buffer);
    ptr as i32
}

#[no_mangle]
pub extern "C" fn alloc_bytes(size: i32) -> i32 {
    if size <= 0 {
        return 0;
    }
    let mut buffer = Vec::<u8>::with_capacity(size as usize);
    let ptr = buffer.as_mut_ptr();
    std::mem::forget(buffer);
    ptr as i32
}

#[no_mangle]
pub extern "C" fn process(input_ptr: i32, output_ptr: i32, samples: i32) {
    if input_ptr <= 0 || output_ptr <= 0 || samples <= 0 {
        return;
    }

    let frames = samples as usize;
    let samples_interleaved = match frames.checked_mul(2) {
        Some(value) => value,
        None => return,
    };

    let input = unsafe { std::slice::from_raw_parts(input_ptr as *const f32, samples_interleaved) };
    let output =
        unsafe { std::slice::from_raw_parts_mut(output_ptr as *mut f32, samples_interleaved) };

    ENGINE.with(|engine| {
        engine.borrow_mut().process(input, output, frames);
    });
}

#[no_mangle]
pub extern "C" fn set_sample_rate(sample_rate: f32) -> i32 {
    ENGINE.with(|engine| {
        engine.borrow_mut().set_sample_rate(sample_rate);
    });
    0
}

#[no_mangle]
pub extern "C" fn set_chain_json(ptr: i32, len: i32) -> i32 {
    if ptr <= 0 || len <= 0 {
        return 1;
    }

    let bytes = unsafe { std::slice::from_raw_parts(ptr as *const u8, len as usize) };
    let chain_json = match std::str::from_utf8(bytes) {
        Ok(value) => value,
        Err(_) => return 2,
    };

    let result = ENGINE.with(|engine| engine.borrow_mut().set_chain_json(chain_json));
    if result.is_ok() {
        0
    } else {
        3
    }
}

#[no_mangle]
pub extern "C" fn set_param_json(ptr: i32, len: i32) -> i32 {
    if ptr <= 0 || len <= 0 {
        return 1;
    }

    let bytes = unsafe { std::slice::from_raw_parts(ptr as *const u8, len as usize) };
    let payload = match std::str::from_utf8(bytes) {
        Ok(value) => value,
        Err(_) => return 2,
    };
    let payload: ParamChange = match serde_json::from_str(payload) {
        Ok(value) => value,
        Err(_) => return 3,
    };

    if payload.index < 0 {
        return 4;
    }

    ENGINE.with(|engine| {
        engine
            .borrow()
            .set_param(payload.index as usize, &payload.param_key, payload.value);
    });
    0
}
