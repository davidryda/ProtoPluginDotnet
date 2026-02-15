#[derive(Debug, schematic::Schematic, serde::Deserialize, serde::Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct DotnetToolConfig {
    pub metadata_index_url: String,
    pub include_eol_channels: bool,
}

impl Default for DotnetToolConfig {
    fn default() -> Self {
        Self {
            metadata_index_url:
                "https://builds.dotnet.microsoft.com/dotnet/release-metadata/releases-index.json"
                    .into(),
            include_eol_channels: false,
        }
    }
}
