use crate::config::DotnetToolConfig;
use crate::metadata::{ChannelReleases, ReleasesIndex, SdkFile};
use extism_pdk::*;
use proto_pdk::*;
use schematic::SchemaBuilder;
use std::collections::{BTreeSet, HashMap};

static NAME: &str = ".NET SDK";

#[host_fn]
extern "ExtismHost" {
    fn exec_command(input: Json<ExecCommandInput>) -> Json<ExecCommandOutput>;
}

#[plugin_fn]
pub fn register_tool(Json(_): Json<RegisterToolInput>) -> FnResult<Json<RegisterToolOutput>> {
    Ok(Json(RegisterToolOutput {
        name: NAME.into(),
        type_of: PluginType::Language,
        minimum_proto_version: Some(Version::new(0, 46, 0)),
        plugin_version: Version::parse(env!("CARGO_PKG_VERSION")).ok(),
        ..RegisterToolOutput::default()
    }))
}

#[plugin_fn]
pub fn define_tool_config(_: ()) -> FnResult<Json<DefineToolConfigOutput>> {
    Ok(Json(DefineToolConfigOutput {
        schema: SchemaBuilder::build_root::<DotnetToolConfig>(),
    }))
}

#[plugin_fn]
pub fn detect_version_files(_: ()) -> FnResult<Json<DetectVersionOutput>> {
    Ok(Json(DetectVersionOutput {
        files: vec!["global.json".into()],
        ignore: vec![],
    }))
}

#[plugin_fn]
pub fn parse_version_file(
    Json(input): Json<ParseVersionFileInput>,
) -> FnResult<Json<ParseVersionFileOutput>> {
    if input.file != "global.json" {
        return Ok(Json(ParseVersionFileOutput { version: None }));
    }

    let payload: json::Value = json::from_str(&input.content)?;
    let sdk_version = payload
        .get("sdk")
        .and_then(|sdk| sdk.get("version"))
        .and_then(|version| version.as_str());

    Ok(Json(ParseVersionFileOutput {
        version: sdk_version
            .map(UnresolvedVersionSpec::parse)
            .transpose()?,
    }))
}

#[plugin_fn]
pub fn load_versions(Json(_): Json<LoadVersionsInput>) -> FnResult<Json<LoadVersionsOutput>> {
    let config = get_tool_config::<DotnetToolConfig>()?;
    let index: ReleasesIndex = fetch_json(&config.metadata_index_url)?;

    let mut channels = index
        .releases_index
        .into_iter()
        .filter(|channel| channel.latest_sdk.is_some())
        .filter(|channel| {
            config.include_eol_channels || channel.support_phase.as_deref().unwrap_or("") != "eol"
        })
        .collect::<Vec<_>>();

    channels.sort_by(|a, b| parse_channel_key(&b.channel_version).cmp(&parse_channel_key(&a.channel_version)));

    let mut versions = BTreeSet::new();
    let mut output = LoadVersionsOutput::default();
    let mut latest_sdk = None::<UnresolvedVersionSpec>;

    for channel in &channels {
        if let Some(sdk) = &channel.latest_sdk {
            let unresolved = UnresolvedVersionSpec::parse(sdk)?;

            if latest_sdk.is_none() {
                latest_sdk = Some(unresolved.clone());
                output.latest = Some(unresolved.clone());
                output.aliases.insert("latest".into(), unresolved.clone());
            }

            if channel.release_type.as_deref() == Some("lts")
                && !output.aliases.contains_key("lts")
            {
                output.aliases.insert("lts".into(), unresolved.clone());
            }

            if channel.release_type.as_deref() == Some("sts")
                && !output.aliases.contains_key("sts")
            {
                output.aliases.insert("sts".into(), unresolved.clone());
            }
        }

        let channel_releases: ChannelReleases = fetch_json(&channel.releases_json)?;
        for release in channel_releases.releases {
            if let Some(sdk) = release.sdk {
                versions.insert(sdk.version);
            }
        }
    }

    for version in sort_versions_desc(versions)? {
        let unresolved = UnresolvedVersionSpec::parse(&version)?;
        output.versions.push(unresolved.to_resolved_spec());
    }

    if output.aliases.contains_key("lts") && !output.aliases.contains_key("stable") {
        let lts = output.aliases.get("lts").cloned().unwrap();
        output.aliases.insert("stable".into(), lts);
    }

    if let Some(latest) = latest_sdk {
        // Support "current" as a convenience alias.
        output.aliases.insert("current".into(), latest);
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn resolve_version(
    Json(input): Json<ResolveVersionInput>,
) -> FnResult<Json<ResolveVersionOutput>> {
    let mut output = ResolveVersionOutput::default();

    if let UnresolvedVersionSpec::Alias(alias) = input.initial {
        match alias.as_str() {
            "stable" => output.candidate = UnresolvedVersionSpec::parse("lts").ok(),
            "current" => output.candidate = UnresolvedVersionSpec::parse("latest").ok(),
            _ => {}
        }
    }

    Ok(Json(output))
}

#[plugin_fn]
pub fn download_prebuilt(
    Json(input): Json<DownloadPrebuiltInput>,
) -> FnResult<Json<DownloadPrebuiltOutput>> {
    let env = get_host_environment()?;

    check_supported_os_and_arch(
        NAME,
        &env,
        permutations! [
            HostOS::Linux => [HostArch::X64, HostArch::Arm64, HostArch::Arm],
            HostOS::MacOS => [HostArch::X64, HostArch::Arm64],
            HostOS::Windows => [HostArch::X64, HostArch::Arm64],
        ],
    )?;

    let version = input
        .context
        .version
        .as_version()
        .ok_or_else(|| plugin_err!("Version alias must resolve to a concrete semantic version."))?;

    let channel = format!("{}.{}", version.major, version.minor);
    let config = get_tool_config::<DotnetToolConfig>()?;
    let index: ReleasesIndex = fetch_json(&config.metadata_index_url)?;
    let channel_entry = index
        .releases_index
        .iter()
        .find(|entry| entry.channel_version == channel)
        .ok_or_else(|| plugin_err!("Could not find release channel <id>{channel}</id> in metadata index."))?;

    let channel_releases: ChannelReleases = fetch_json(&channel_entry.releases_json)?;
    let rid = map_rid(&env);
    let extension = if env.os.is_windows() { ".zip" } else { ".tar.gz" };
    let target_version = version.to_string();

    for release in channel_releases.releases {
        let Some(sdk) = release.sdk else {
            continue;
        };

        if sdk.version != target_version {
            continue;
        }

        let file = pick_file_for_rid(&sdk.files, rid, extension).ok_or_else(|| {
            plugin_err!(
                "No SDK archive found for RID <id>{rid}</id> and extension <id>{extension}</id>."
            )
        })?;

        return Ok(Json(DownloadPrebuiltOutput {
            download_url: file.url.clone(),
            download_name: Some(download_name_from_url(&file.url)),
            checksum_url: Some(format!(
                "https://builds.dotnet.microsoft.com/dotnet/checksums/{}-sha.txt",
                release.release_version
            )),
            ..DownloadPrebuiltOutput::default()
        }));
    }

    Err(plugin_err!(
        "Could not find .NET SDK version <id>{target_version}</id> in channel <id>{channel}</id>."
    )
    .into())
}

#[plugin_fn]
pub fn locate_executables(
    Json(_): Json<LocateExecutablesInput>,
) -> FnResult<Json<LocateExecutablesOutput>> {
    let env = get_host_environment()?;

    Ok(Json(LocateExecutablesOutput {
        exes: HashMap::from_iter([(
            "dotnet".into(),
            ExecutableConfig::new_primary(env.os.get_exe_name("dotnet")),
        )]),
        globals_lookup_dirs: vec!["$HOME/.dotnet/tools".into(), "$DOTNET_ROOT/tools".into()],
        ..LocateExecutablesOutput::default()
    }))
}

fn map_rid(env: &HostEnvironment) -> &'static str {
    match (env.os, env.arch, env.libc) {
        (HostOS::Linux, HostArch::Arm, _) => "linux-arm",
        (HostOS::Linux, HostArch::Arm64, HostLibc::Musl) => "linux-musl-arm64",
        (HostOS::Linux, HostArch::Arm64, _) => "linux-arm64",
        (HostOS::Linux, HostArch::X64, HostLibc::Musl) => "linux-musl-x64",
        (HostOS::Linux, HostArch::X64, _) => "linux-x64",
        (HostOS::MacOS, HostArch::Arm64, _) => "osx-arm64",
        (HostOS::MacOS, HostArch::X64, _) => "osx-x64",
        (HostOS::Windows, HostArch::Arm64, _) => "win-arm64",
        (HostOS::Windows, HostArch::X64, _) => "win-x64",
        _ => unreachable!(),
    }
}

fn pick_file_for_rid<'a>(files: &'a [SdkFile], rid: &str, extension: &str) -> Option<&'a SdkFile> {
    files
        .iter()
        .find(|file| file.rid == rid && file.name.ends_with(extension))
}

fn parse_channel_key(channel: &str) -> (u64, u64) {
    let mut parts = channel.split('.');
    let major = parts.next().and_then(|value| value.parse().ok()).unwrap_or(0);
    let minor = parts.next().and_then(|value| value.parse().ok()).unwrap_or(0);
    (major, minor)
}

fn sort_versions_desc(versions: BTreeSet<String>) -> FnResult<Vec<String>> {
    let mut parsed = versions
        .into_iter()
        .map(|value| Version::parse(&value).map(|version| (version, value)))
        .collect::<Result<Vec<_>, _>>()?;

    parsed.sort_by(|a, b| b.0.cmp(&a.0));

    Ok(parsed.into_iter().map(|(_, raw)| raw).collect())
}

fn download_name_from_url(url: &str) -> String {
    url.rsplit('/').next().unwrap_or("dotnet-sdk").to_owned()
}
