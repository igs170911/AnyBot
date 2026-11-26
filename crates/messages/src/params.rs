use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub enum ParamValue {
    Int(i64),
    Text(String),
}

impl ParamValue {
    #[must_use]
    pub fn int(value: i64) -> Self {
        Self::Int(value)
    }

    #[must_use]
    pub fn text(value: &str) -> Self {
        Self::Text(value.to_owned())
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Params(BTreeMap<String, ParamValue>);

impl Params {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn single(key: &str, value: ParamValue) -> Self {
        let mut out = Self::new();
        out.0.insert(key.to_owned(), value);
        out
    }

    #[must_use]
    pub fn with_text(mut self, key: &str, value: &str) -> Self {
        self.0.insert(key.to_owned(), ParamValue::text(value));
        self
    }

    #[must_use]
    pub fn with_int(mut self, key: &str, value: i64) -> Self {
        self.0.insert(key.to_owned(), ParamValue::int(value));
        self
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[must_use]
    pub fn keys(&self) -> Vec<&str> {
        self.0.keys().map(String::as_str).collect()
    }

    #[must_use]
    pub fn get(&self, key: &str) -> Option<&ParamValue> {
        self.0.get(key)
    }

    #[must_use]
    pub fn entries(&self) -> BTreeMap<String, ParamValue> {
        self.0.clone()
    }

    #[must_use]
    pub fn to_json_value(&self) -> serde_json::Value {
        let mut map = serde_json::Map::new();
        for (key, value) in &self.0 {
            let raw = match value {
                ParamValue::Int(number) => serde_json::Value::Number((*number).into()),
                ParamValue::Text(text) => serde_json::Value::String(text.clone()),
            };
            map.insert(key.clone(), raw);
        }
        serde_json::Value::Object(map)
    }

    pub fn from_json_value(value: &serde_json::Value) -> Result<Self, ParamsError> {
        let map = value.as_object().ok_or_else(|| ParamsError {
            reason: "params must be a JSON object".to_owned(),
        })?;
        let mut out = Self::new();
        for (key, raw) in map {
            let parsed = if let Some(number) = raw.as_i64() {
                ParamValue::Int(number)
            } else if let Some(text) = raw.as_str() {
                ParamValue::Text(text.to_owned())
            } else {
                return Err(ParamsError {
                    reason: format!("param {key} must be an integer or a string"),
                });
            };
            out.0.insert(key.clone(), parsed);
        }
        Ok(out)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParamsError {
    pub reason: String,
}

impl std::fmt::Display for ParamsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.reason)
    }
}

impl std::error::Error for ParamsError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_params_builder_keeps_keys_sorted_and_typed() {
        let p = Params::new()
            .with_int("port", 31_313)
            .with_text("db_host", "localhost");
        assert_eq!(p.keys(), vec!["db_host", "port"]);
        assert_eq!(p.get("port"), Some(&ParamValue::Int(31_313)));
        assert_eq!(p.get("db_host"), Some(&ParamValue::text("localhost")));
        assert!(!p.is_empty());
    }

    #[test]
    fn test_params_json_roundtrip_preserves_int_and_text() {
        let p = Params::single("version", ParamValue::text("0.1.0")).with_int("count", 3);
        let json = p.to_json_value();
        assert_eq!(json.get("version"), Some(&serde_json::json!("0.1.0")));
        assert_eq!(json.get("count"), Some(&serde_json::json!(3)));
        let back = Params::from_json_value(&json).unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn test_params_from_json_rejects_non_object_and_nested_values() {
        let bad = serde_json::json!([1, 2]);
        assert!(Params::from_json_value(&bad).is_err());
        let nested = serde_json::json!({ "a": { "b": 1 } });
        assert!(Params::from_json_value(&nested).is_err());
    }
}
