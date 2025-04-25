use chrono::{Duration, TimeDelta};
use serde::{Deserialize, Deserializer};

use crate::Error;

#[derive(Debug, Clone)]
pub struct Video {
    pub id: String,
    pub title: String,
    pub duration: Duration,
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

        let ids: Vec<_> = list
            .items
            .into_iter()
            .filter_map(|search| match search.id {
                Id::Video { video_id } => Some(video_id),
                _ => None,
            })
            // .filter_map(|search| search.try_into().ok())
            // .map(Ok)
            .collect();

        let videos_resource: VideosResource = reqwest::Client::new()
            .get("https://www.googleapis.com/youtube/v3/videos")
            // FIXME add oAuth
            // .header("Authorization", format!("Bearer {token}"))
            // .query(&[("part", "id,snippet,fileDetails")])
            .query(&[("part", "id,snippet,statistics,contentDetails")])
            .query(&[("id", ids.join(","))])
            .query(&[("maxResults", "25")])
            .query(&[("key", token)])
            .send()
            .await?
            .json()
            .await?;

        Ok(videos_resource
            .items
            .into_iter()
            .map(|resource| Video {
                id: resource.id,
                title: resource.snippet.title,
                duration: resource.content_details.duration,
            })
            .collect())
    }
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
#[serde(rename_all = "camelCase")]
struct VideosResource {
    items: Vec<VideoResource>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct VideoResource {
    id: String,
    snippet: Snippet,
    content_details: ContentDetails,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct ContentDetails {
    #[serde(deserialize_with = "deserialize_iso8601_duration")]
    duration: TimeDelta,
}

pub fn deserialize_iso8601_duration<'de, D>(deserializer: D) -> Result<TimeDelta, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let duration = iso8601_duration::Duration::parse(&s).unwrap();
    Ok(duration.to_chrono().unwrap())
}
// impl Playlist {
//     pub async fn from_ids(ids: Vec<String>) -> Self {
//         let token = env!("YT_TOKEN");

//         let response = reqwest::Client::new()
//             .get("https://www.googleapis.com/youtube/v3/playlists")
//             .query(&[("part", "id,snippet")])
//             .query(&[("id", id.into_iter().join(","))])
//             .query(&[("key", token)])
//             .send()
//             .await?;

//     }
// }

// impl TryFrom<SearchResult> for Video {
//     type Error = ();

//     fn try_from(search: SearchResult) -> Result<Self, Self::Error> {
//         let Id::Video { video_id: id } = search.id else {
//             return Err(());
//         };

//         Ok(Self {
//             id,
//             title: search.snippet.title,
//         })
//     }
// }

// impl TryFrom<SearchResult> for Resource {
//     type Error;

//     fn try_from(search: SearchResult) -> Result<Self, Self::Error> {
//         let result = match search.id {
//             Id::Video { video_id } => Resource::Video {
//                 id: video_id,
//                 title: search.snippet.title,
//             },
//             Id::Channel { channel_id } => return Err(()),
//             Id::Playlist { playlist_id } => Resource::Playlist { id: playlist_id, title: search.snippet.title, videos: () },
//         }
//     }
// }

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

    #[test]
    fn parse_videos_resource() {
        let json = r#"
{
  "kind": "youtube#videoListResponse",
  "etag": "1w5F3FCMpByyYWkU-1dOE20vDW4",
  "items": [
    {
      "kind": "youtube#video",
      "etag": "eBoa1A_qEhFmkvL88y0HKyxwQAU",
      "id": "FUvxRjYqjEQ",
      "snippet": {
        "publishedAt": "2014-09-11T17:03:26Z",
        "channelId": "UC1-evqmLMusdbdD65Sz0-5Q",
        "title": "The Allman Brothers Band - Whipping Post - 9/23/1970 - Fillmore East (Official)",
        "description": "The Allman Brothers Band - Whipping Post\nRecorded Live: 9/23/1970 - Fillmore East - New York, NY\nMore The Allman Brothers Band at Music Vault: http://www.musicvault.com\nSubscribe to Music Vault on YouTube: http://goo.gl/DUzpUF\n\nPersonnel: \nGregg Allman - organ, vocals\nDuane Allman - guitar, vocals\nDickey Betts - guitar, vocals\nBerry Oakley - bass, vocals\nButch Trucks - drums\nJai Johanny Johanson - drums\nTom Doucette - harp\n\nSummary: \nOn this date, Bill Graham assembled a stellar roster of bands to participate in the filming of a television special called Welcome To The Fillmore East for broadcast on educational channels. Short sets were filmed by the Byrds, the Elvin Bishop Group, Sha-Na-Na, Van Morrison, and the Allman Brothers Band, as well as behind-the-scenes footage of Bill Graham and the Fillmore East staff at work. \n\nThe Allman Brothers performance is nothing short of spectacular and features the original lineup that included Duane Allman and Berry Oakley. Recorded six months prior to the legendary Live At Fillmore East double album set, this performance captures the Allman Brothers when they were a relatively new band, full of youthful passion and performing what would become classic original material when it was fresh and new.\n\nFollowing Bill Graham's introduction, they kick things off with a tight performance of \"Don't Keep Me Wonderin',\" which features the band's friend, Tom Doucette, blowing harp over the group's trademark sound. Gregg's vocal is barely audible, but it's obvious the group is full of fire. \"Dreams,\" which follows, slows things down a bit and the group establishes a relaxed groove that showcases their trademark sound, blending elements that would eventually come to define \"Southern Rock.\"\n\nThey hit their stride on the next number, Dickey Betts' \"In Memory Of Elizabeth Reed.\" Here, the dual guitar attack of Allman and Betts is astounding. The two guitarists intertwine and synchronize in a manner nothing short of telepathic, creating a melting pot seasoned with elements of jazz, rock, country, and blues into a style utterly their own. The set ends with a ferocious take of \"Whipping Post\" that features outstanding melodic bass playing from Berry Oakley, with both Duane Allman and Dickey Betts soaring over the propulsive rhythm section. Shorter than the expansive versions that would develop in coming months, this is all the more fascinating for it, as they compress an incredible amount of energy into the time allotted. \n\nTime constrictions and vocal microphone malfunctions aside, this is still a fascinating performance. This original lineup of the band was certainly one of the most innovative and captivating bands to ever play the Fillmore.",
        "thumbnails": {
          "default": {
            "url": "https://i.ytimg.com/vi/FUvxRjYqjEQ/default.jpg",
            "width": 120,
            "height": 90
          },
          "medium": {
            "url": "https://i.ytimg.com/vi/FUvxRjYqjEQ/mqdefault.jpg",
            "width": 320,
            "height": 180
          },
          "high": {
            "url": "https://i.ytimg.com/vi/FUvxRjYqjEQ/hqdefault.jpg",
            "width": 480,
            "height": 360
          },
          "standard": {
            "url": "https://i.ytimg.com/vi/FUvxRjYqjEQ/sddefault.jpg",
            "width": 640,
            "height": 480
          }
        },
        "channelTitle": "Allman Brothers on MV",
        "tags": [
          "The Allman Brothers Band",
          "Bill Graham",
          "live music",
          "music vault",
          "New York",
          "Fillmore East",
          "Idlewild South Tour",
          "Whipping Post"
        ],
        "categoryId": "10",
        "liveBroadcastContent": "none",
        "localized": {
          "title": "The Allman Brothers Band - Whipping Post - 9/23/1970 - Fillmore East (Official)",
          "description": "The Allman Brothers Band - Whipping Post\nRecorded Live: 9/23/1970 - Fillmore East - New York, NY\nMore The Allman Brothers Band at Music Vault: http://www.musicvault.com\nSubscribe to Music Vault on YouTube: http://goo.gl/DUzpUF\n\nPersonnel: \nGregg Allman - organ, vocals\nDuane Allman - guitar, vocals\nDickey Betts - guitar, vocals\nBerry Oakley - bass, vocals\nButch Trucks - drums\nJai Johanny Johanson - drums\nTom Doucette - harp\n\nSummary: \nOn this date, Bill Graham assembled a stellar roster of bands to participate in the filming of a television special called Welcome To The Fillmore East for broadcast on educational channels. Short sets were filmed by the Byrds, the Elvin Bishop Group, Sha-Na-Na, Van Morrison, and the Allman Brothers Band, as well as behind-the-scenes footage of Bill Graham and the Fillmore East staff at work. \n\nThe Allman Brothers performance is nothing short of spectacular and features the original lineup that included Duane Allman and Berry Oakley. Recorded six months prior to the legendary Live At Fillmore East double album set, this performance captures the Allman Brothers when they were a relatively new band, full of youthful passion and performing what would become classic original material when it was fresh and new.\n\nFollowing Bill Graham's introduction, they kick things off with a tight performance of \"Don't Keep Me Wonderin',\" which features the band's friend, Tom Doucette, blowing harp over the group's trademark sound. Gregg's vocal is barely audible, but it's obvious the group is full of fire. \"Dreams,\" which follows, slows things down a bit and the group establishes a relaxed groove that showcases their trademark sound, blending elements that would eventually come to define \"Southern Rock.\"\n\nThey hit their stride on the next number, Dickey Betts' \"In Memory Of Elizabeth Reed.\" Here, the dual guitar attack of Allman and Betts is astounding. The two guitarists intertwine and synchronize in a manner nothing short of telepathic, creating a melting pot seasoned with elements of jazz, rock, country, and blues into a style utterly their own. The set ends with a ferocious take of \"Whipping Post\" that features outstanding melodic bass playing from Berry Oakley, with both Duane Allman and Dickey Betts soaring over the propulsive rhythm section. Shorter than the expansive versions that would develop in coming months, this is all the more fascinating for it, as they compress an incredible amount of energy into the time allotted. \n\nTime constrictions and vocal microphone malfunctions aside, this is still a fascinating performance. This original lineup of the band was certainly one of the most innovative and captivating bands to ever play the Fillmore."
        }
      },
      "contentDetails": {
        "duration": "PT11M23S",
        "dimension": "2d",
        "definition": "sd",
        "caption": "false",
        "licensedContent": true,
        "contentRating": {},
        "projection": "rectangular"
      },
      "statistics": {
        "viewCount": "15111123",
        "likeCount": "114163",
        "favoriteCount": "0",
        "commentCount": "12627"
      }
    }
  ],
  "pageInfo": {
    "totalResults": 1,
    "resultsPerPage": 1
  }
}
"#;

        let result: VideosResource = serde_json::from_str(json).unwrap();
        assert_eq!(result.items.len(), 1);
    }
}
