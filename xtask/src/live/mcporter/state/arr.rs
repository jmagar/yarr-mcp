use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

use super::{assert_object_field_eq, expect_success};
use crate::live::mcporter::{call_tool, report};

pub(super) fn run_tag_lifecycle(
    report: &mut report::Report,
    mcp_url: &str,
    service: &str,
    api_prefix: &str,
) -> Result<()> {
    let label = format!("rustarr-live-mcporter-{service}-{}", std::process::id());
    cleanup_matching_tags(mcp_url, service, api_prefix, "rustarr-live-mcporter-")?;

    let create_args = json!({
        "action": "api_post",
        "path": format!("{api_prefix}/tag"),
        "body": { "label": label },
        "confirm": true,
    });
    let created = expect_success(
        service,
        "api_post tag create",
        call_tool(mcp_url, service, &create_args)?,
    )?;
    let tag_id = created.get("id").and_then(Value::as_i64).ok_or_else(|| {
        anyhow::anyhow!("{service} tag create did not return numeric id: {created}")
    })?;
    assert_object_field_eq(&created, "label", &label)
        .with_context(|| format!("{service} tag create did not echo label"))?;

    let list_path = format!("{api_prefix}/tag");
    assert_tag_present(mcp_url, service, &list_path, &label)?;

    let updated_label = format!("{label}-updated");
    let put_args = json!({
        "action": "api_put",
        "path": format!("{api_prefix}/tag/{tag_id}"),
        "body": { "id": tag_id, "label": updated_label },
        "confirm": true,
    });
    let updated = expect_success(
        service,
        "api_put tag update",
        call_tool(mcp_url, service, &put_args)?,
    )?;
    assert_object_field_eq(&updated, "label", &updated_label)
        .with_context(|| format!("{service} tag update did not echo updated label"))?;
    assert_tag_present(mcp_url, service, &list_path, &updated_label)?;

    let delete_args = json!({
        "action": "api_delete",
        "path": format!("{api_prefix}/tag/{tag_id}"),
        "confirm": true,
    });
    let _ = expect_success(
        service,
        "api_delete tag delete",
        call_tool(mcp_url, service, &delete_args)?,
    )?;
    assert_tag_absent(mcp_url, service, &list_path, &updated_label)?;

    report.pass(
        format!("mcporter confirmed write lifecycle {service} tag"),
        "api_post/api_put/api_delete changed observable state and cleaned up",
    );
    Ok(())
}

fn cleanup_matching_tags(
    mcp_url: &str,
    service: &str,
    api_prefix: &str,
    label_prefix: &str,
) -> Result<()> {
    let list_path = format!("{api_prefix}/tag");
    let tags = expect_success(
        service,
        "api_get tag cleanup list",
        call_tool(
            mcp_url,
            service,
            &json!({ "action": "api_get", "path": list_path }),
        )?,
    )?;
    for tag in tags.as_array().into_iter().flatten() {
        let Some(label) = tag.get("label").and_then(Value::as_str) else {
            continue;
        };
        let Some(id) = tag.get("id").and_then(Value::as_i64) else {
            continue;
        };
        if label.starts_with(label_prefix) {
            let _ = call_tool(
                mcp_url,
                service,
                &json!({
                    "action": "api_delete",
                    "path": format!("{api_prefix}/tag/{id}"),
                    "confirm": true,
                }),
            )?;
        }
    }
    Ok(())
}

fn assert_tag_present(mcp_url: &str, service: &str, list_path: &str, label: &str) -> Result<()> {
    let tags = expect_success(
        service,
        "api_get tag present",
        call_tool(
            mcp_url,
            service,
            &json!({ "action": "api_get", "path": list_path }),
        )?,
    )?;
    if tag_labels(&tags).any(|candidate| candidate == label) {
        return Ok(());
    }
    bail!("{service} tag list did not contain `{label}` after confirmed write: {tags}");
}

fn assert_tag_absent(mcp_url: &str, service: &str, list_path: &str, label: &str) -> Result<()> {
    let tags = expect_success(
        service,
        "api_get tag absent",
        call_tool(
            mcp_url,
            service,
            &json!({ "action": "api_get", "path": list_path }),
        )?,
    )?;
    if tag_labels(&tags).any(|candidate| candidate == label) {
        bail!("{service} tag list still contained `{label}` after confirmed delete: {tags}");
    }
    Ok(())
}

fn tag_labels(value: &Value) -> impl Iterator<Item = &str> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|tag| tag.get("label").and_then(Value::as_str))
}

pub(super) fn run_arr_item_lifecycle(
    report: &mut report::Report,
    mcp_url: &str,
    service: &str,
    api_prefix: &str,
    term: &str,
    root_folder: &str,
) -> Result<()> {
    ensure_root_folder(mcp_url, service, api_prefix, root_folder)?;
    cleanup_arr_title(mcp_url, service, term)?;

    let added = expect_success(
        service,
        "add",
        call_tool(
            mcp_url,
            service,
            &json!({
                "action": "add",
                "term": term,
                "quality_profile": "Any",
                "root_folder": root_folder,
                "confirm": true,
            }),
        )?,
    )?;
    let item_id = added
        .get("id")
        .and_then(Value::as_i64)
        .ok_or_else(|| anyhow::anyhow!("{service} add did not return numeric id: {added}"))?;
    assert_arr_item_present(mcp_url, service, item_id, Some(term), None)?;

    expect_arr_editor_write(mcp_url, service, "unmonitor", item_id)?;
    assert_arr_item_present(mcp_url, service, item_id, Some(term), Some(false))?;

    expect_arr_editor_write(mcp_url, service, "monitor", item_id)?;
    assert_arr_item_present(mcp_url, service, item_id, Some(term), Some(true))?;

    expect_arr_editor_write_with_args(
        mcp_url,
        service,
        "set_quality",
        json!({ "to": "Any", "ids": [item_id], "confirm": true }),
    )?;
    assert_arr_item_present(mcp_url, service, item_id, Some(term), Some(true))?;

    for action in ["search", "refresh"] {
        let value = expect_success(
            service,
            action,
            call_tool(
                mcp_url,
                service,
                &json!({ "action": action, "ids": [item_id], "confirm": true }),
            )?,
        )?;
        if value.get("async").and_then(Value::as_bool) != Some(true) {
            bail!("{service} {action} did not report async=true: {value}");
        }
    }

    let deleted = expect_success(
        service,
        "delete",
        call_tool(
            mcp_url,
            service,
            &json!({
                "action": "delete",
                "id": item_id.to_string(),
                "delete_files": false,
                "confirm": true,
            }),
        )?,
    )?;
    if deleted.get("deleted").and_then(Value::as_i64) != Some(item_id) {
        bail!("{service} delete did not echo deleted id {item_id}: {deleted}");
    }
    assert_arr_item_absent(mcp_url, service, item_id)?;

    report.pass(
        format!("mcporter confirmed arr item lifecycle {service}"),
        "add/monitor/unmonitor/set_quality/search/refresh/delete changed observable item state and cleaned up",
    );
    Ok(())
}

fn ensure_root_folder(
    mcp_url: &str,
    service: &str,
    api_prefix: &str,
    root_folder: &str,
) -> Result<()> {
    let list_path = format!("{api_prefix}/rootfolder");
    let roots = expect_success(
        service,
        "api_get rootfolder",
        call_tool(
            mcp_url,
            service,
            &json!({ "action": "api_get", "path": list_path }),
        )?,
    )?;
    if root_folder_paths(&roots).any(|candidate| candidate == root_folder) {
        return Ok(());
    }

    let created = expect_success(
        service,
        "api_post rootfolder",
        call_tool(
            mcp_url,
            service,
            &json!({
                "action": "api_post",
                "path": format!("{api_prefix}/rootfolder"),
                "body": { "path": root_folder },
                "confirm": true,
            }),
        )?,
    )?;
    assert_object_field_eq(&created, "path", root_folder)
        .with_context(|| format!("{service} rootfolder create did not echo path"))?;
    Ok(())
}

fn root_folder_paths(value: &Value) -> impl Iterator<Item = &str> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|root| root.get("path").and_then(Value::as_str))
}

fn cleanup_arr_title(mcp_url: &str, service: &str, title: &str) -> Result<()> {
    let items = arr_list(mcp_url, service)?;
    for item in items.as_array().into_iter().flatten() {
        let Some(candidate_title) = item.get("title").and_then(Value::as_str) else {
            continue;
        };
        let Some(id) = item.get("id").and_then(Value::as_i64) else {
            continue;
        };
        if candidate_title.eq_ignore_ascii_case(title) {
            let _ = call_tool(
                mcp_url,
                service,
                &json!({
                    "action": "delete",
                    "id": id.to_string(),
                    "delete_files": false,
                    "confirm": true,
                }),
            )?;
        }
    }
    Ok(())
}

fn expect_arr_editor_write(mcp_url: &str, service: &str, action: &str, id: i64) -> Result<Value> {
    expect_arr_editor_write_with_args(
        mcp_url,
        service,
        action,
        json!({ "ids": [id], "confirm": true }),
    )
}

fn expect_arr_editor_write_with_args(
    mcp_url: &str,
    service: &str,
    action: &str,
    mut args: Value,
) -> Result<Value> {
    args["action"] = json!(action);
    let value = expect_success(service, action, call_tool(mcp_url, service, &args)?)?;
    if value.get("changed").and_then(Value::as_i64).is_none()
        && value
            .get("upstream_count")
            .and_then(Value::as_i64)
            .is_none()
    {
        bail!("{service} {action} did not return an editor mutation summary: {value}");
    }
    Ok(value)
}

fn assert_arr_item_present(
    mcp_url: &str,
    service: &str,
    id: i64,
    title: Option<&str>,
    monitored: Option<bool>,
) -> Result<()> {
    let items = arr_list(mcp_url, service)?;
    let item = items
        .as_array()
        .into_iter()
        .flatten()
        .find(|item| item.get("id").and_then(Value::as_i64) == Some(id))
        .ok_or_else(|| anyhow::anyhow!("{service} list did not contain id {id}: {items}"))?;
    if let Some(title) = title {
        let actual = item.get("title").and_then(Value::as_str).unwrap_or("");
        if actual != title {
            bail!("{service} item {id} title mismatch: expected {title}, got {actual}");
        }
    }
    if let Some(monitored) = monitored
        && item.get("monitored").and_then(Value::as_bool) != Some(monitored)
    {
        bail!("{service} item {id} monitored state mismatch, expected {monitored}: {item}");
    }
    Ok(())
}

fn assert_arr_item_absent(mcp_url: &str, service: &str, id: i64) -> Result<()> {
    let items = arr_list(mcp_url, service)?;
    if items
        .as_array()
        .into_iter()
        .flatten()
        .any(|item| item.get("id").and_then(Value::as_i64) == Some(id))
    {
        bail!("{service} list still contained id {id} after delete: {items}");
    }
    Ok(())
}

fn arr_list(mcp_url: &str, service: &str) -> Result<Value> {
    expect_success(
        service,
        "list",
        call_tool(mcp_url, service, &json!({ "action": "list" }))?,
    )
}
