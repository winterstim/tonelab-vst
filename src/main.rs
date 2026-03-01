use nih_plug::prelude::*;
use tonelab_vst::TonelabPlugin;

fn main() {
    nih_export_standalone::<TonelabPlugin>();
}
