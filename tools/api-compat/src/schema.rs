use serde_json::Value;

#[must_use = "the parsed document is compared with the other document"]
pub fn load(text: &str) -> Result<Value, String> {
    let doc: Value =
        yaml_serde::from_str(text).map_err(|e| format!("cannot parse the yaml document: {e}"))?;
    Ok(doc)
}

#[must_use]
pub fn breaking(baseline_text: &str, current_text: &str) -> Vec<String> {
    let (Ok(baseline), Ok(current)) = (load(baseline_text), load(current_text)) else {
        return vec![
            "the protocol document cannot be parsed; the gate cannot compare it".to_owned(),
        ];
    };
    let mut out = Vec::new();
    walk("", &baseline, &current, &mut out);
    out
}

fn walk(path: &str, baseline: &Value, current: &Value, out: &mut Vec<String>) {
    match (baseline, current) {
        (Value::Object(a), Value::Object(b)) => {
            for (key, old) in a {
                match b.get(key) {
                    None => out.push(format!("{path}/{key}: removed")),
                    Some(new) => walk(&format!("{path}/{key}"), old, new, out),
                }
            }
            for (key, new) in b {
                if !a.contains_key(key) && key == "required" {
                    out.push(format!("{path}/{key}: added"));
                }
                let _: &Value = new;
            }
        }
        (Value::Array(a), Value::Array(b)) => {
            if b.len() < a.len() {
                out.push(format!("{path}: array shrank"));
            }
        }
        (a, b) => {
            if a != b {
                out.push(format!("{path}: value changed"));
            }
        }
    }
}
