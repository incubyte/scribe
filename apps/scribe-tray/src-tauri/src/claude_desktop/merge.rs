use std::path::Path;

use serde_json::{Map, Value};

use crate::mcp_config::{SCRIBE_SERVER_NAME, SERVERS_KEY, scribe_server_entry};

#[derive(Debug, PartialEq, Eq)]
pub enum MergeResult {
    Unchanged,
    Write(String),
}

#[derive(Debug, thiserror::Error)]
pub enum UnreadableConfig {
    #[error("not valid JSON")]
    InvalidJson,
    #[error("top level is not an object")]
    RootNotObject,
    #[error("mcpServers is not an object")]
    ServersNotObject,
}

pub fn merge_scribe_server(
    existing: Option<&str>,
    server_command: &Path,
) -> Result<MergeResult, UnreadableConfig> {
    let existing = existing.filter(|text| !text.trim().is_empty());
    let mut root = match existing {
        Some(text) => parse_root(text)?,
        None => Map::new(),
    };
    let entry = scribe_server_entry(server_command);
    let servers = servers_of(&mut root)?;
    if existing.is_some() && servers.get(SCRIBE_SERVER_NAME) == Some(&entry) {
        return Ok(MergeResult::Unchanged);
    }
    servers.insert(SCRIBE_SERVER_NAME.to_owned(), entry);
    Ok(MergeResult::Write(render(root)))
}

fn parse_root(existing: &str) -> Result<Map<String, Value>, UnreadableConfig> {
    match serde_json::from_str(existing) {
        Ok(Value::Object(root)) => Ok(root),
        Ok(_) => Err(UnreadableConfig::RootNotObject),
        Err(_) => Err(UnreadableConfig::InvalidJson),
    }
}

fn servers_of(root: &mut Map<String, Value>) -> Result<&mut Map<String, Value>, UnreadableConfig> {
    root.entry(SERVERS_KEY)
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .ok_or(UnreadableConfig::ServersNotObject)
}

fn render(root: Map<String, Value>) -> String {
    format!("{:#}\n", Value::Object(root))
}

#[must_use]
pub fn scribe_server_in(existing: &str) -> Option<Value> {
    let mut root = parse_root(existing).ok()?;
    servers_of(&mut root).ok()?.remove(SCRIBE_SERVER_NAME)
}
