use std::{
    fs::File,
    io::{BufReader, ErrorKind},
    path::PathBuf,
    time::Duration,
};

use echo_db::{
    recordings::{
        ClaimInputs, FailureCode, Recording, RecordingStatus, UpdateRecording,
        invalidate_processing,
    },
    transcript::{self, CreateTranscript},
};
use echo_infra::{
    aws::sqs::{ProcessingMessage, Queue},
    recording_storage::{RecordingStore, StorageError},
};
use tokio::{io::AsyncWriteExt, process::Command, time::timeout};
use uuid::Uuid;
use whisper_rs::{FullParams, SamplingStrategy, WhisperState};

use crate::{ProcessingState, create_file, error::WorkerError, tempdir::TempDir};

const SAMPLE_RATE: usize = 16_000;
const CHUNK_SECONDS: usize = 30;
const CHUNK_SAMPLES: usize = SAMPLE_RATE * CHUNK_SECONDS;

#[cfg(test)]
mod tests {
    /// Testing PCM checks that typically caught by ffmpeg conversion first
    use std::path::PathBuf;

    use super::read_chunk;
    use echo_db::recordings::FailureCode::{self};

    use crate::{error::WorkerError, tempdir::TempDir, transcribe::load_pcm_wav};

    fn spec(sample_rate: u32, channels: u16) -> hound::WavSpec {
        hound::WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        }
    }

    fn write_wav(dir: &PathBuf, name: &str, spec: hound::WavSpec, samples: &[i16]) -> PathBuf {
        let path = dir.join(name);

        let mut w = hound::WavWriter::create(&path, spec).unwrap();
        for &s in samples {
            w.write_sample(s).unwrap();
        }
        w.finalize().unwrap();
        path
    }

    fn write_bytes(dir: &PathBuf, name: &str, bytes: &[u8]) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, bytes).unwrap();

        path
    }

    #[test]
    fn load_missing_file() {
        let temp_dir = TempDir::unstable_new_sync();
        let empty_path = temp_dir.path_buf.join("empty.wav");

        assert!(matches!(
            load_pcm_wav(&empty_path),
            Err(WorkerError::InvalidRecording(FailureCode::MissingObject))
        ))
    }

    #[test]
    fn load_empty_file() {
        let temp_dir = TempDir::unstable_new_sync();
        let empty_path = temp_dir.path_buf.join("empty.wav");
        std::fs::write(&empty_path, []).unwrap();

        assert!(matches!(
            load_pcm_wav(&empty_path),
            Err(WorkerError::InvalidRecording(FailureCode::EmptyFile))
        ))
    }

    #[test]
    fn load_invalid_content_long_file() {
        let temp_dir = TempDir::unstable_new_sync();
        let corrupted_file_path =
            write_bytes(&temp_dir.path_buf, "corrupted.wav", b"somehting longer");

        assert!(matches!(
            load_pcm_wav(&corrupted_file_path),
            Err(WorkerError::InvalidRecording(FailureCode::PcmRejected))
        ))
    }

    #[test]
    fn load_invalid_content_short_file() {
        let temp_dir = TempDir::unstable_new_sync();
        let corrupted_file_path = write_bytes(
            &temp_dir.path_buf,
            "corrupted-short.wav",
            b"RIFF\x24\x00\x00\x00WAVE",
        );

        assert!(matches!(
            load_pcm_wav(&corrupted_file_path),
            Err(WorkerError::InvalidRecording(FailureCode::PcmRejected))
        ))
    }

    #[test]
    fn load_valid_header_only_file() {
        let temp_dir = TempDir::unstable_new_sync();
        let valid_header_file_path =
            write_wav(&temp_dir.path_buf, "valid_header.wav", spec(16_000, 1), &[]);

        assert!(matches!(
            load_pcm_wav(&valid_header_file_path),
            Err(WorkerError::InvalidRecording(FailureCode::EmptyFile))
        ))
    }

    #[test]
    fn load_incomplete_file() {
        let temp_dir = TempDir::unstable_new_sync();
        let incomplete_file_path = write_wav(
            &temp_dir.path_buf,
            "incomplete_file.wav",
            spec(16_000, 1),
            &[1; 1000],
        );
        let bytes = std::fs::read(&incomplete_file_path).unwrap();

        write_bytes(
            &temp_dir.path_buf,
            "incomplete_file.wav",
            &bytes[..bytes.len() - 20],
        );

        let mut reader = load_pcm_wav(&incomplete_file_path).unwrap();

        let result = reader.samples::<i16>().collect::<Result<Vec<_>, _>>();

        assert!(result.is_err());
    }

    #[test]
    fn load_wrong_sample_rate() {
        let temp_dir = TempDir::unstable_new_sync();
        let sample_rate_file_path = write_wav(
            &temp_dir.path_buf,
            "sample_file.wav",
            spec(44_100, 1),
            &[0; 1000],
        );

        assert!(matches!(
            load_pcm_wav(&sample_rate_file_path),
            Err(WorkerError::InvalidRecording(
                FailureCode::UnsupportedAudioType
            ))
        ))
    }

    #[test]
    fn load_wrong_bit_depth() {
        let temp_dir = TempDir::unstable_new_sync();
        let sample_rate_file_path = write_wav(
            &temp_dir.path_buf,
            "sample_file.wav",
            hound::WavSpec {
                bits_per_sample: 24,
                ..spec(16_000, 1)
            },
            &[0; 1000],
        );

        assert!(matches!(
            load_pcm_wav(&sample_rate_file_path),
            Err(WorkerError::InvalidRecording(
                FailureCode::UnsupportedAudioType
            ))
        ))
    }

    #[test]
    fn load_wrong_sample_format() {
        let temp_dir = TempDir::unstable_new_sync();
        let wav_spec = hound::WavSpec {
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
            ..spec(16_000, 1)
        };

        let sample_rate_file_path = temp_dir.path_buf.join("sample_file.wav");

        let mut writer = hound::WavWriter::create(&sample_rate_file_path, wav_spec).unwrap();
        writer.write_sample(0.5_f32).unwrap();
        writer.finalize().unwrap();

        assert!(matches!(
            load_pcm_wav(&sample_rate_file_path),
            Err(WorkerError::InvalidRecording(
                FailureCode::UnsupportedAudioType
            ))
        ))
    }

    #[test]
    fn load_wrong_channels() {
        let temp_dir = TempDir::unstable_new_sync();
        let channels_file_path = write_wav(
            &temp_dir.path_buf,
            "channels_file.wav",
            spec(16_000, 2),
            &[0; 1000],
        );

        assert!(matches!(
            load_pcm_wav(&channels_file_path),
            Err(WorkerError::InvalidRecording(
                FailureCode::UnsupportedAudioType
            ))
        ))
    }

    #[test]
    fn load_valid_file() {
        let temp_dir = TempDir::unstable_new_sync();
        let file_path = write_wav(
            &temp_dir.path_buf,
            "valid.wav",
            spec(16_000, 1),
            &[0, i16::MAX, i16::MIN, 16_384],
        );

        let mut reader = load_pcm_wav(&file_path).unwrap();
        let mut buffer = vec![];
        let mut samples = reader.samples::<i16>();

        assert_eq!(read_chunk(&mut samples, &mut buffer, 2).unwrap(), 2);
        assert_eq!(buffer, [0.0, 32767.0 / 32768.0]);
        assert_eq!(read_chunk(&mut samples, &mut buffer, 2).unwrap(), 2);
        assert_eq!(buffer, [-1.0, 0.5]);

        assert_eq!(read_chunk(&mut samples, &mut buffer, 2).unwrap(), 0);
        assert!(buffer.is_empty());
    }
}

fn read_chunk(
    samples: &mut impl Iterator<Item = hound::Result<i16>>,
    buffer: &mut Vec<f32>,
    max_samples: usize,
) -> Result<usize, WorkerError> {
    buffer.clear();

    for sample in samples.take(max_samples) {
        let sample = sample.map_err(|_| WorkerError::InvalidRecording(FailureCode::PcmRejected))?;

        buffer.push(sample as f32 / 32768.0);
    }

    Ok(buffer.len())
}

/// Converts audio files to pcm, best for whisper flow
/// TODO: Currently shorter files that hound can not eval the header for are marked as io errs rather than rejects
async fn run_ffmpeg_pcm(input_path: &PathBuf, output_path: &PathBuf) -> Result<(), WorkerError> {
    let cmd = Command::new("ffmpeg")
        .arg("-nostdin")
        .arg("-hide_banner")
        .arg("-loglevel")
        .arg("error")
        .arg("-y")
        .arg("-i")
        .arg(input_path)
        .args([
            "-map",
            "0:a:0",
            "-ar",
            "16000",
            "-ac",
            "1",
            "-c:a",
            "pcm_s16le",
            "-f",
            "wav",
        ])
        .arg(output_path)
        .kill_on_drop(true)
        .output();

    // TODO: benchmark timeout to determine if 10 minutes is long enough for 1+ hour recordings
    let output = match timeout(Duration::from_secs(600), cmd).await {
        Err(_) => return Err(WorkerError::FFMpegTimeout),
        Ok(Err(_)) => return Err(WorkerError::FFMpegSpawn),
        Ok(Ok(output)) => output,
    };

    if !output.status.success() {
        tracing::warn!(
            "FFmpeg conversion failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        return Err(WorkerError::InvalidRecording(FailureCode::PcmRejected));
    }

    Ok(())
}

fn load_pcm_wav(path: &PathBuf) -> Result<hound::WavReader<BufReader<File>>, WorkerError> {
    let file = std::fs::File::open(path).map_err(|err| {
        if err.kind() == ErrorKind::NotFound {
            WorkerError::InvalidRecording(FailureCode::MissingObject)
        } else {
            WorkerError::IO(err)
        }
    })?;

    if file.metadata().map_err(|err| WorkerError::IO(err))?.len() == 0 {
        return Err(WorkerError::InvalidRecording(FailureCode::EmptyFile));
    }

    let reader = hound::WavReader::new(std::io::BufReader::new(file)).map_err(|err| match err {
        hound::Error::IoError(io_err) if matches!(io_err.kind(), ErrorKind::Other) => {
            WorkerError::InvalidRecording(FailureCode::PcmRejected)
        }
        hound::Error::IoError(io_err) => WorkerError::IO(io_err),

        _ => WorkerError::InvalidRecording(FailureCode::PcmRejected),
    })?;

    let spec = reader.spec();
    if spec.sample_rate != 16_000
        || spec.channels != 1
        || spec.bits_per_sample != 16
        || spec.sample_format != hound::SampleFormat::Int
    {
        return Err(WorkerError::InvalidRecording(
            FailureCode::UnsupportedAudioType,
        ));
    }

    if reader.len() == 0 {
        return Err(WorkerError::InvalidRecording(FailureCode::EmptyFile));
    }

    Ok(reader)
}

fn run_whisper(
    state: &mut WhisperState,
    mut reader: hound::WavReader<BufReader<File>>,
    lang: &str,
) -> Result<String, WorkerError> {
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_language(Some(lang)); // or Some("auto")
    params.set_n_threads(2);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_special(false);
    params.set_print_timestamps(false);

    //TODO: Decide if its worth treating each chunk independently
    // Move to using a small overlay or VAD to chunk on sliences/breaks in speech
    // params.set_no_context(true);

    let mut samples = reader.samples::<i16>();
    let mut buffer = Vec::with_capacity(CHUNK_SAMPLES);
    let mut out = String::new();
    let mut processed_samples = 0_u64;

    loop {
        let offset_secs = processed_samples as f64 / SAMPLE_RATE as f64;
        buffer.clear();

        let chunk_len = read_chunk(&mut samples, &mut buffer, CHUNK_SAMPLES)?;

        if chunk_len == 0 {
            break;
        }

        state
            .full(params.clone(), &buffer)
            .map_err(|err| WorkerError::Whisper(err))?;

        for i in 0..state.full_n_segments() {
            if let Some(seg) = state.get_segment(i) {
                // timestamps are in centiseconds
                out.push_str(&format!(
                    "[{:>6.2}s -> {:>6.2}s] {}\n",
                    offset_secs + seg.start_timestamp() as f64 / 100.0,
                    offset_secs + seg.end_timestamp() as f64 / 100.0,
                    seg
                ));
            }
        }

        processed_samples += buffer.len() as u64;
    }

    Ok(out)
}

pub async fn handle_processing(
    state: &ProcessingState<impl RecordingStore>,
    path: &PathBuf,
    recording: &Recording,
) -> Result<(String, Uuid), WorkerError> {
    let mut stream = state
        .storage
        .get_recording(recording.id)
        .await
        .map_err(|err| match err {
            StorageError::MissingObject => {
                WorkerError::InvalidRecording(FailureCode::MissingObject)
            }
            err => WorkerError::Storage(err),
        })?;

    let temp_child = TempDir::from_parent(&path.join(Uuid::new_v4().to_string())).await?;
    let input_path = temp_child.join(&recording.id.to_string());
    let mut file = create_file(&input_path).await?;

    tokio::io::copy(&mut stream, &mut file).await?;
    file.flush().await?;
    drop(file);

    let outpath_path = temp_child.join(&format!("{}.wav", recording.id));

    run_ffmpeg_pcm(&input_path, &outpath_path).await?;
    let reader = load_pcm_wav(&outpath_path)?;
    let mut context_state = state
        .context
        .create_state()
        .map_err(|err| WorkerError::Whisper(err))?;

    let output = tokio::task::spawn_blocking(move || run_whisper(&mut context_state, reader, "en"))
        .await
        .map_err(|err| WorkerError::TaskJoin(err))??;

    Ok((output, recording.user_id))
}

pub async fn process_and_persist(
    state: &ProcessingState<impl RecordingStore>,
    temp_storage: &TempDir,
    body: &ProcessingMessage,
    processing_claim_id: Uuid,
) -> Result<(), WorkerError> {
    let recording = match echo_db::recordings::claim_process(
        &state.pool,
        body.recording_id,
        processing_claim_id,
    )
    .await?
    {
        Some(result) => result,
        None => {
            let result = echo_db::recordings::find_by_server_id(&state.pool, body.recording_id)
                .await?
                .ok_or(WorkerError::MissingRecording)?;

            match result.status {
                RecordingStatus::Processing => return Err(WorkerError::ProcessingInProgress),
                RecordingStatus::Validating => return Err(WorkerError::ValidationInProgress),
                RecordingStatus::Completed
                | RecordingStatus::Invalid
                | RecordingStatus::Processed
                | RecordingStatus::Abort => {
                    return Err(WorkerError::ProcessingHandled);
                }
                RecordingStatus::Validated => echo_db::recordings::claim_process(
                    &state.pool,
                    body.recording_id,
                    processing_claim_id,
                )
                .await?
                .ok_or(WorkerError::ProcessingInProgress)?,
                _ => return Err(WorkerError::InvalidRecordingState),
            }
        }
    };

    let (transcript, user_id) =
        handle_processing(&state, &temp_storage.path_buf, &recording).await?;

    let mut tx = state.pool.begin().await?;

    echo_db::transcript::create(
        &mut *tx,
        CreateTranscript {
            recording_id: body.recording_id,
            user_id: user_id,
            content: transcript,
        },
    )
    .await?;

    echo_db::recordings::complete_processing(&mut *tx, body.recording_id, processing_claim_id)
        .await?;

    tx.commit().await?;

    Ok(())
}

pub async fn listen_transcribe(
    state: &ProcessingState<impl RecordingStore>,
    queues: &Queue,
) -> Result<(), WorkerError> {
    let temp_storage = TempDir::new().await?;

    tracing::info!("Listening to Process Queue");

    loop {
        let Some(message) = queues.receieve_process_message().await? else {
            continue;
        };

        let body: ProcessingMessage = match serde_json::from_str(message.body().unwrap_or("")) {
            Ok(msg) => msg,
            Err(_) => {
                queues.delete_processing(&message).await?;
                tracing::error!(
                  message_id = ?message.message_id(),
                  "invalid parsing processing message"
                );
                continue;
            }
        };
        let processing_id = Uuid::new_v4();
        let process_result =
            process_and_persist(&state, &temp_storage, &body, processing_id.clone()).await;

        match process_result {
            Ok(_) => {
                queues.delete_processing(&message).await?;
            }

            Err(err) => {
                tracing::error!(
                  recording_id = %&body.recording_id,
                  error = %err,
                  "Processing recording failed"
                );

                match err {
                    WorkerError::InvalidRecording(failure_code) => {
                        invalidate_processing(
                            &state.pool,
                            body.recording_id,
                            processing_id,
                            failure_code,
                        )
                        .await?;

                        queues.delete_processing(&message).await?;
                    }
                    WorkerError::InvalidRecordingState => {
                        // TODO: Add some type of recovering worker that will look
                        // unexpected recording state, might be recoverable
                        queues.move_message_dql(&message).await?;
                    }
                    WorkerError::ProcessingHandled | WorkerError::MissingRecording => {
                        queues.delete_processing(&message).await?;
                    }
                    WorkerError::ValidationInProgress | WorkerError::ProcessingInProgress => {
                        // Don't do anything and allow the message to be retried later
                    }

                    // Recording can be retried later. Dont delete sqs message
                    WorkerError::FFMpegTimeout => {
                        // TODO: This might not be retriable, needs benchmarking to tell if timeout is suitable.
                        // Currently it will retry on timeout.
                        echo_db::recordings::release_claim_process(
                            &state.pool,
                            body.recording_id,
                            processing_id,
                        )
                        .await?;
                    }

                    _ => {
                        echo_db::recordings::release_claim_process(
                            &state.pool,
                            body.recording_id,
                            processing_id,
                        )
                        .await?;
                    }
                }
            }
        }
    }
}
