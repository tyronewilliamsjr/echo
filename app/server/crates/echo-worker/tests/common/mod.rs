pub mod database;

use std::{
    path::PathBuf,
    sync::{Arc, LazyLock},
};

pub use database::{create_recording, create_user};
use whisper_rs::{WhisperContext, WhisperContextParameters};

static WHISPER_CONTEXT: LazyLock<Arc<WhisperContext>> = LazyLock::new(|| {
    let path_buf = match std::env::var("WHISPER_MODEL_PATH") {
        Ok(p) => PathBuf::from(p),
        Err(_) => PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("models")
            .join("ggml-base.en.bin"),
    };

    Arc::new(
        WhisperContext::new_with_params(path_buf, WhisperContextParameters::default())
            .expect("Failed to load Whisper model"),
    )
});

pub fn whisper_context() -> Arc<WhisperContext> {
    Arc::clone(&WHISPER_CONTEXT)
}
