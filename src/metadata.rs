use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ReleasesIndex {
    #[serde(rename = "releases-index")]
    pub releases_index: Vec<ReleaseChannel>,
}

#[derive(Debug, Deserialize)]
pub struct ReleaseChannel {
    #[serde(rename = "channel-version")]
    pub channel_version: String,

    #[serde(rename = "latest-sdk")]
    pub latest_sdk: Option<String>,

    #[serde(rename = "release-type")]
    pub release_type: Option<String>,

    #[serde(rename = "support-phase")]
    pub support_phase: Option<String>,

    #[serde(rename = "releases.json")]
    pub releases_json: String,
}

#[derive(Debug, Deserialize)]
pub struct ChannelReleases {
    pub releases: Vec<ReleaseEntry>,
}

#[derive(Debug, Deserialize)]
pub struct ReleaseEntry {
    #[serde(rename = "release-version")]
    pub release_version: String,
    pub sdk: Option<SdkEntry>,
}

#[derive(Debug, Deserialize)]
pub struct SdkEntry {
    pub version: String,
    #[serde(default)]
    pub files: Vec<SdkFile>,
}

#[derive(Debug, Deserialize)]
pub struct SdkFile {
    #[serde(default)]
    pub rid: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub url: String,
}
