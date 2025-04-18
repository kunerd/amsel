use reqwest::Client;
use serde::Deserialize;

use crate::Error;

#[derive(Debug, Clone)]
pub struct Video {
    id: String,
    pub title: String,
}

#[derive(Debug, Deserialize)]
struct ListResponse {
    items: Vec<SearchResult>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct SearchResult {
    id: Id,
    snippet: Snippet,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all_fields = "camelCase")]
enum Id {
    #[serde(rename = "youtube#video")]
    Video { video_id: String },
    #[serde(rename = "youtube#channel")]
    Channel { channel_id: String },
    #[serde(rename = "youtube#playlist")]
    Playlist { playlist_id: String },
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
struct Snippet {
    title: String,
    description: String,
}

impl Video {
    pub async fn search(query: String) -> Result<Vec<Self>, Error> {
        // FIXME: load token on run time
        let token = env!("YT_TOKEN");

        let response = reqwest::Client::new()
            .get("https://www.googleapis.com/youtube/v3/search")
            // FIXME add oAuth
            // .header("Authorization", format!("Bearer {token}"))
            .query(&[("part", "id,snippet")])
            .query(&[("q", query)])
            .query(&[("maxResults", "25")])
            .query(&[("key", token)])
            .query(&[("type", "video")])
            .send()
            .await?;

        let list: ListResponse = response.json().await?;

        list.items
            .into_iter()
            .filter_map(|search| search.try_into().ok())
            .map(Ok)
            .collect()
    }
}

impl TryFrom<SearchResult> for Video {
    type Error = ();

    fn try_from(search: SearchResult) -> Result<Self, Self::Error> {
        let Id::Video { video_id: id } = search.id else {
            return Err(());
        };

        Ok(Self {
            id,
            title: search.snippet.title,
        })
    }
}

// pub async fn _playback() -> anyhow::Result<()> {

//     // let video_id = list.items.first().unwrap().id.get("videoId").unwrap();

//     tracing_subscriber::fmt()
//         .with_env_filter(EnvFilter::default().add_directive(LevelFilter::INFO.into()))
//         .with_line_number(true)
//         .with_file(true)
//         .init();

//     // let url = args()
//     //     .nth(1)
//     // .unwrap_or_else(|| format!("https://www.youtube.com/watch?v={video_id}"));
//     // .unwrap_or_else(|| format!("https://www.youtube.com/watch?v={video_id}"));
//     // let url = "https://www.youtube.com/watch?v=L_XJ_s5IsQc".to_string();
//     let url = "https://www.youtube.com/watch?v=dGEjbJkxFhs".to_string();

//     let format = "m4a";

//     info!("extracting video metadata - this may take a few seconds");
//     let output = YoutubeDl::new(&url)
//         .format(format)
//         .extract_audio(true)
//         .run_async()
//         .await
//         .expect("meta data")
//         .into_single_video()
//         .expect("to extract video metadata");
//     info!("metadata extraction complete");

//     let size = output.filesize.expect("file size") as u64;

//     let cmd = YtDlpCommand::new(url).extract_audio(true).format(format);
//     let reader = StreamDownload::new_process(
//         ProcessStreamParams::new(cmd)?.content_length(size),
//         TempStorageProvider::new(),
//         // Disable cancel_on_drop to ensure no error messages from the process are lost.
//         Settings::default().cancel_on_drop(false),
//     )
//     .await?;
//     let reader_handle = reader.handle();
//     let reader = Box::new(reader);

//     let handle = tokio::task::spawn_blocking(move || {
//         let (_stream, handle) = rodio::OutputStream::try_default()?;
//         let sink = rodio::Sink::try_new(&handle)?;
//         sink.append(rodio::Decoder::new(reader)?);
//         sink.sleep_until_end();

//         Ok::<_, Box<dyn Error + Send + Sync>>(())
//     });
//     handle.await?;
//     // Wait for the spawned subprocess to terminate gracefully
//     reader_handle.wait_for_completion().await;

//     Ok(())
// }
//
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_id() {
        let json = r#"
      {
        "kind": "youtube#video",
        "videoId": "1pW_j_eJIlo"
      }"#;

        let id: Id = serde_json::from_str(json).unwrap();
        assert_eq!(
            id,
            Id::Video {
                video_id: "1pW_j_eJIlo".to_string()
            }
        );
    }

    #[test]
    fn parse_snippet() {
        let json = r#"{
            "publishedAt": "2025-04-16T19:12:18Z",
            "channelId": "UCzH2vVrSpjwHNM0U3jJM0lQ",
            "title": "Dark Blues Slide Guitar • &quot;Black Moon&quot;",
            "description": "Unlock your guitar potential with exclusive lessons and tips at http://JustinJohnsonGuitar.com — from easy-to-follow basics to ...",
            "thumbnails": {
              "default": {
                "url": "https://i.ytimg.com/vi/1pW_j_eJIlo/default.jpg",
                "width": 120,
                "height": 90
              },
              "medium": {
                "url": "https://i.ytimg.com/vi/1pW_j_eJIlo/mqdefault.jpg",
                "width": 320,
                "height": 180
              },
              "high": {
                "url": "https://i.ytimg.com/vi/1pW_j_eJIlo/hqdefault.jpg",
                "width": 480,
                "height": 360
              }
            },
            "channelTitle": "Justin Johnson",
            "liveBroadcastContent": "none",
            "publishTime": "2025-04-16T19:12:18Z"
          }"#;

        let snippet: Snippet = serde_json::from_str(json).unwrap();
        assert_eq!(
            snippet,
            Snippet {
                    title: "Dark Blues Slide Guitar • &quot;Black Moon&quot;".to_string(),
                    description: "Unlock your guitar potential with exclusive lessons and tips at http://JustinJohnsonGuitar.com — from easy-to-follow basics to ...".to_string()
                }
        );
    }

    #[test]
    fn parse_search_result() {
        let json = r#"{
      "kind": "youtube#searchListResponse",
      "etag": "zKTHRvvYczZrAz9JayAYpOlPSws",
      "nextPageToken": "CBkQAA",
      "regionCode": "DE",
      "pageInfo": {
        "totalResults": 1000000,
        "resultsPerPage": 25
      },
      "items": [
        {
          "kind": "youtube#searchResult",
          "etag": "yJ-DyL6UtoYLkLxUZl1NriJnWYk",
          "id": {
            "kind": "youtube#video",
            "videoId": "1pW_j_eJIlo"
          },
          "snippet": {
            "publishedAt": "2025-04-16T19:12:18Z",
            "channelId": "UCzH2vVrSpjwHNM0U3jJM0lQ",
            "title": "Dark Blues Slide Guitar • &quot;Black Moon&quot;",
            "description": "Unlock your guitar potential with exclusive lessons and tips at http://JustinJohnsonGuitar.com — from easy-to-follow basics to ...",
            "thumbnails": {
              "default": {
                "url": "https://i.ytimg.com/vi/1pW_j_eJIlo/default.jpg",
                "width": 120,
                "height": 90
              },
              "medium": {
                "url": "https://i.ytimg.com/vi/1pW_j_eJIlo/mqdefault.jpg",
                "width": 320,
                "height": 180
              },
              "high": {
                "url": "https://i.ytimg.com/vi/1pW_j_eJIlo/hqdefault.jpg",
                "width": 480,
                "height": 360
              }
            },
            "channelTitle": "Justin Johnson",
            "liveBroadcastContent": "none",
            "publishTime": "2025-04-16T19:12:18Z"
          }
        }
      ]
    }"#;

        let result: ListResponse = serde_json::from_str(json).unwrap();
        assert_eq!(
            result.items[0],
            SearchResult {
                id: Id::Video {
                    video_id: "1pW_j_eJIlo".to_string()
                },
                snippet: Snippet {
                    title: "Dark Blues Slide Guitar • &quot;Black Moon&quot;".to_string(),
                    description: "Unlock your guitar potential with exclusive lessons and tips at http://JustinJohnsonGuitar.com — from easy-to-follow basics to ...".to_string()
                }
            }
        );
    }
}
