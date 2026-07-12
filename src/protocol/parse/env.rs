use crate::protocol::Action;
use serde_json::Value;

pub(super) fn parse_env(raw: &Value, _id: &str, type_str: &str) -> anyhow::Result<Action> {
    Ok(match type_str {
        "env.get" => Action::EnvGet {
            name: raw["name"].as_str().map(String::from),
        },
        "env.set" => {
            let name = raw["name"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("env.set requires 'name'"))?
                .to_string();
            validate_name(&name, "env.set")?;
            let value = raw["value"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("env.set requires 'value'"))?
                .to_string();
            Action::EnvSet { name, value }
        }
        "env.persist" => Action::EnvPersist {
            vars: parse_vars(raw, "env.persist")?,
        },
        "env.unset" => {
            let names = raw["names"]
                .as_array()
                .ok_or_else(|| anyhow::anyhow!("env.unset requires 'names' array"))?
                .iter()
                .map(|v| {
                    v.as_str()
                        .map(String::from)
                        .ok_or_else(|| anyhow::anyhow!("env.unset: every name must be a string"))
                })
                .collect::<anyhow::Result<Vec<String>>>()?;
            for n in &names {
                validate_name(n, "env.unset")?;
            }
            Action::EnvUnset { names }
        }
        "env.list_persisted" => Action::EnvListPersisted,
        // Unknown env.* action
        other => anyhow::bail!("no env parser for {:?}", other),
    })
}

/// Validate a single env var name: non-empty, no `=`, no NUL.
fn validate_name(name: &str, action: &str) -> anyhow::Result<()> {
    if name.is_empty() {
        anyhow::bail!("{}: name is empty", action);
    }
    if name.contains('=') {
        anyhow::bail!(
            "{}: invalid name {:?} — contains '='; use a variable name, not an assignment",
            action,
            name
        );
    }
    if name.contains('\0') {
        anyhow::bail!("{}: invalid name {:?} — contains a NUL byte", action, name);
    }
    Ok(())
}

/// Parse the `vars` field of an env.persist request. Accepts the same
/// shapes as `locale.set`:
///
/// - `"vars": {"LANG": "en_US.UTF-8", "LC_TIME": "en_DK.UTF-8"}`  (object)
/// - `"vars": [["LANG","en_US.UTF-8"], ...]`  (array of pairs)
///
/// Returns an empty Vec if `vars` is missing or null.
fn parse_vars(raw: &Value, action: &str) -> anyhow::Result<Vec<(String, String)>> {
    let Some(v) = raw.get("vars") else {
        return Ok(Vec::new());
    };
    if v.is_null() {
        return Ok(Vec::new());
    }
    if let Some(obj) = v.as_object() {
        let mut out = Vec::with_capacity(obj.len());
        for (k, val) in obj {
            let s = val
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("{}: var {:?} must be a string", action, k))?;
            out.push((k.clone(), s.to_string()));
        }
        return Ok(out);
    }
    if let Some(arr) = v.as_array() {
        let mut out = Vec::with_capacity(arr.len());
        for (i, item) in arr.iter().enumerate() {
            let pair = item
                .as_array()
                .ok_or_else(|| anyhow::anyhow!("{}: vars[{}] must be [name, value]", action, i))?;
            if pair.len() != 2 {
                anyhow::bail!("{}: vars[{}] must be exactly [name, value]", action, i);
            }
            let name = pair[0]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("{}: vars[{}][0] must be a string", action, i))?;
            let value = pair[1]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("{}: vars[{}][1] must be a string", action, i))?;
            out.push((name.to_string(), value.to_string()));
        }
        return Ok(out);
    }
    anyhow::bail!("{}: 'vars' must be an object or array of pairs", action)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_env_get() {
        let v = json!({"name": "PATH"});
        let a = parse_env(&v, "1", "env.get").unwrap();
        match a {
            Action::EnvGet { name: Some(n) } if n == "PATH" => {}
            other => panic!("unexpected: {:?}", other),
        }
    }

    #[test]
    fn parse_env_set_validates_name() {
        let v = json!({"name": "FOO=BAR", "value": "x"});
        let err = parse_env(&v, "1", "env.set").unwrap_err().to_string();
        assert!(err.contains("="), "got: {}", err);
    }

    #[test]
    fn parse_env_persist_accepts_object() {
        let v = json!({"vars": {"LANG": "en_US.UTF-8", "LC_TIME": "en_DK.UTF-8"}});
        let a = parse_env(&v, "1", "env.persist").unwrap();
        match a {
            Action::EnvPersist { vars } => {
                assert_eq!(vars.len(), 2);
                assert!(vars.contains(&("LANG".into(), "en_US.UTF-8".into())));
            }
            other => panic!("unexpected: {:?}", other),
        }
    }

    #[test]
    fn parse_env_persist_accepts_array_of_pairs() {
        let v = json!({"vars": [["LANG", "C"], ["EDITOR", "nvim"]]});
        let a = parse_env(&v, "1", "env.persist").unwrap();
        match a {
            Action::EnvPersist { vars } => {
                assert_eq!(vars.len(), 2);
            }
            other => panic!("unexpected: {:?}", other),
        }
    }

    #[test]
    fn parse_env_unset_requires_names_array() {
        let v = json!({});
        let err = parse_env(&v, "1", "env.unset").unwrap_err().to_string();
        assert!(err.contains("'names' array"), "got: {}", err);
    }

    #[test]
    fn parse_env_unset_rejects_invalid_names() {
        let v = json!({"names": ["FOO=BAR"]});
        let err = parse_env(&v, "1", "env.unset").unwrap_err().to_string();
        assert!(err.contains("="), "got: {}", err);
    }

    #[test]
    fn parse_env_list_persisted() {
        let v = json!({});
        let a = parse_env(&v, "1", "env.list_persisted").unwrap();
        assert!(matches!(a, Action::EnvListPersisted));
    }

    #[test]
    fn validate_name_rejects_equals() {
        assert!(validate_name("FOO=BAR", "test").is_err());
    }

    #[test]
    fn validate_name_rejects_empty() {
        assert!(validate_name("", "test").is_err());
    }

    #[test]
    fn validate_name_rejects_nul() {
        assert!(validate_name("FOO\0BAR", "test").is_err());
    }

    #[test]
    fn validate_name_accepts_underscores_and_digits() {
        assert!(validate_name("MY_VAR_42", "test").is_ok());
    }
}
