use codex_protocol::openai_models::ModelsResponse;
use serde::Deserialize;
use serde::Deserializer;
use serde::de::Error as _;
use serde::de::MapAccess;
use serde::de::SeqAccess;
use serde::de::Visitor;
use serde_json::Map;
use serde_json::Number;
use serde_json::Value;
use std::fmt;
use std::io;

const MODEL_IDS: [&str; 4] = ["gpt-5.5", "gpt-5.4", "gpt-5.4-mini", "gpt-5.2"];
const CATALOG_LIMIT: usize = 256 * 1024;

/// Load the compiled Covenant catalog through its bounded transport validator.
pub fn covenant_model_catalog() -> io::Result<ModelsResponse> {
    decode(include_bytes!("../../../covenant/model-catalog.json"))
}

/// Resolve an exact approved identity, using the compiled default when absent.
pub fn covenant_selected_model(model: Option<&str>) -> io::Result<&'static str> {
    let requested = model.unwrap_or(MODEL_IDS[0]);
    MODEL_IDS
        .into_iter()
        .find(|candidate| *candidate == requested)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "Covenant model selection refused",
            )
        })
}

fn catalog_error() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "Covenant model catalog refused")
}

fn decode(bytes: &[u8]) -> io::Result<ModelsResponse> {
    if bytes.len() > CATALOG_LIMIT {
        return Err(catalog_error());
    }
    let UniqueJson(value) = serde_json::from_slice(bytes).map_err(|_| catalog_error())?;
    if !value
        .as_object()
        .is_some_and(|object| object.len() == 1 && object.contains_key("models"))
        || value["models"]
            .as_array()
            .is_none_or(|models| models.len() != MODEL_IDS.len())
    {
        return Err(catalog_error());
    }
    // The preliminary value checks transport/shape only. Decode the original
    // bytes to retain ModelsResponse's legacy/default and arbitrary-precision
    // semantics under dependency feature unification.
    let catalog: ModelsResponse = serde_json::from_slice(bytes).map_err(|_| catalog_error())?;
    let mut seen = [false; MODEL_IDS.len()];
    for model in &catalog.models {
        let Some(index) = MODEL_IDS
            .iter()
            .position(|identity| *identity == model.slug)
        else {
            return Err(catalog_error());
        };
        if seen[index] {
            return Err(catalog_error());
        }
        seen[index] = true;
    }
    Ok(catalog)
}

// serde_json's default recursion limit remains active while this visitor checks
// decoded keys per object, including metadata the typed catalog later ignores.
struct UniqueJson(Value);

impl<'de> Deserialize<'de> for UniqueJson {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(UniqueJsonVisitor)
    }
}

struct UniqueJsonVisitor;

impl<'de> Visitor<'de> for UniqueJsonVisitor {
    type Value = UniqueJson;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("JSON with unique object keys")
    }

    fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
        Ok(UniqueJson(Value::Null))
    }

    fn visit_bool<E: serde::de::Error>(self, value: bool) -> Result<Self::Value, E> {
        Ok(UniqueJson(Value::Bool(value)))
    }

    fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<Self::Value, E> {
        Ok(UniqueJson(Value::Number(value.into())))
    }

    fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<Self::Value, E> {
        Ok(UniqueJson(Value::Number(value.into())))
    }

    fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<Self::Value, E> {
        Number::from_f64(value)
            .map(|number| UniqueJson(Value::Number(number)))
            .ok_or_else(|| E::custom("invalid JSON number"))
    }

    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
        Ok(UniqueJson(Value::String(value.to_owned())))
    }

    fn visit_string<E: serde::de::Error>(self, value: String) -> Result<Self::Value, E> {
        Ok(UniqueJson(Value::String(value)))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
        let mut values = Vec::new();
        while let Some(UniqueJson(value)) = sequence.next_element()? {
            values.push(value);
        }
        Ok(UniqueJson(Value::Array(values)))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut object: A) -> Result<Self::Value, A::Error> {
        let mut values = Map::new();
        while let Some(key) = object.next_key::<String>()? {
            if values.contains_key(&key) {
                return Err(A::Error::custom("duplicate JSON key"));
            }
            let UniqueJson(value) = object.next_value()?;
            values.insert(key, value);
        }
        Ok(UniqueJson(Value::Object(values)))
    }
}

#[cfg(test)]
#[path = "covenant_catalog_tests.rs"]
mod tests;
