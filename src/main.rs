use std::env::args;
use std::error::Error;

use stream_download::process::{ProcessStreamParams, YtDlpCommand};
use stream_download::storage::temp::TempStorageProvider;
use stream_download::{Settings, StreamDownload};
use tracing::info;
use tracing::metadata::LevelFilter;
use tracing_subscriber::EnvFilter;
use youtube_dl::YoutubeDl;

use reqwest;
use serde::Deserialize;

use std::collections::HashMap;

#[derive(Debug, Deserialize)]
struct ListResponse {
    items: Vec<Video>,
}

#[derive(Debug, Deserialize)]
struct Video {
    id: HashMap<String, String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // let token = env!("YT_TOKEN");
    // let search_text = "justin johnson";
    // let list: ListResponse = reqwest::Client::new()
    //     .get("https://www.googleapis.com/youtube/v3/search")
    //     // .header("Authorization", format!("Bearer {token}"))
    //     .query(&[("part", "id,snippet")])
    //     .query(&[("q", search_text)])
    //     .query(&[("maxResults", "1")])
    //     .query(&[("key", token)])
    //     .query(&[("type", "video")])
    //     .send().await.unwrap().json().await.unwrap();

    // let video_id = list.items.first().unwrap().id.get("videoId").unwrap();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::default().add_directive(LevelFilter::INFO.into()))
        .with_line_number(true)
        .with_file(true)
        .init();

    // let url = args()
    //     .nth(1)
    // .unwrap_or_else(|| format!("https://www.youtube.com/watch?v={video_id}"));
    // .unwrap_or_else(|| format!("https://www.youtube.com/watch?v={video_id}"));
    // let url = "https://www.youtube.com/watch?v=L_XJ_s5IsQc".to_string();
    let url = "https://www.youtube.com/watch?v=dGEjbJkxFhs".to_string();

    let format = "m4a";

    info!("extracting video metadata - this may take a few seconds");
    let output = YoutubeDl::new(&url)
        .format(format)
        .extract_audio(true)
        .run_async()
        .await
        .expect("meta data")
        .into_single_video()
        .expect("to extract video metadata");
    info!("metadata extraction complete");

    let size = output.filesize.expect("file size") as u64;

    let cmd = YtDlpCommand::new(url).extract_audio(true).format(format);
    let reader = StreamDownload::new_process(
        ProcessStreamParams::new(cmd)?.content_length(size),
        TempStorageProvider::new(),
        // Disable cancel_on_drop to ensure no error messages from the process are lost.
        Settings::default().cancel_on_drop(false),
    )
    .await?;
    let reader_handle = reader.handle();
    let reader = Box::new(reader);

    let handle = tokio::task::spawn_blocking(move || {
        let (_stream, handle) = rodio::OutputStream::try_default()?;
        let sink = rodio::Sink::try_new(&handle)?;
        sink.append(rodio::Decoder::new(reader)?);
        sink.sleep_until_end();

        Ok::<_, Box<dyn Error + Send + Sync>>(())
    });
    handle.await?;
    // Wait for the spawned subprocess to terminate gracefully
    reader_handle.wait_for_completion().await;

    Ok(())
}
