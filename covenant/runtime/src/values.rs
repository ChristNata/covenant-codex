use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::fmt;
use std::marker::PhantomData;
use std::ops::Deref;

use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::de::Error;
use serde::de::IntoDeserializer;
use serde::de::MapAccess;
use serde::de::Visitor;
use serde::de::value::MapAccessDeserializer;

// Struct derives also accept positional sequences. Every schema object enters
// through this map-only wrapper while retaining its typed field validation.
#[derive(Serialize)]
#[serde(transparent)]
pub(crate) struct Object<T>(T);

impl<T> Deref for Object<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Object<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ObjectVisitor<T>(PhantomData<T>);

        impl<'de, T: Deserialize<'de>> Visitor<'de> for ObjectVisitor<T> {
            type Value = Object<T>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a JSON object")
            }

            fn visit_map<M: MapAccess<'de>>(self, map: M) -> Result<Self::Value, M::Error> {
                T::deserialize(MapAccessDeserializer::new(map)).map(Object)
            }
        }

        deserializer.deserialize_map(ObjectVisitor(PhantomData))
    }
}

// Unit-enum derives also accept externally tagged maps; first require a string.
pub(crate) fn string_enum<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(String::deserialize(deserializer)?.into_deserializer())
}

#[derive(Serialize)]
#[serde(transparent)]
pub(crate) struct NonemptyString(String);

impl<'de> Deserialize<'de> for NonemptyString {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        if value.is_empty() {
            return Err(D::Error::custom("expected a nonempty string"));
        }
        Ok(Self(value))
    }
}

#[derive(Serialize)]
#[serde(transparent)]
pub(crate) struct Digest(String);

impl<'de> Deserialize<'de> for Digest {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        if value.len() != 64
            || !value
                .bytes()
                .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
        {
            return Err(D::Error::custom("expected a lowercase SHA-256 digest"));
        }
        Ok(Self(value))
    }
}

#[derive(Serialize)]
#[serde(transparent)]
pub(crate) struct NonemptyVec<T>(pub(crate) Vec<T>);

impl<'de, T: Deserialize<'de>> Deserialize<'de> for NonemptyVec<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let values = Vec::<T>::deserialize(deserializer)?;
        if values.is_empty() {
            return Err(D::Error::custom("expected a nonempty array"));
        }
        Ok(Self(values))
    }
}

// Missing fields default to None, but a present value must deserialize as T;
// unlike Option<T>'s decoder, explicit JSON null is not treated as absence.
pub(crate) fn present<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

#[derive(Serialize)]
#[serde(transparent)]
pub(crate) struct Environment(BTreeMap<String, String>);

impl<'de> Deserialize<'de> for Environment {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct EnvironmentVisitor;

        impl<'de> Visitor<'de> for EnvironmentVisitor {
            type Value = Environment;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("an environment object with unique string keys")
            }

            fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Environment, M::Error> {
                let mut values = BTreeMap::new();
                while let Some(key) = map.next_key::<String>()? {
                    match values.entry(key) {
                        Entry::Vacant(entry) => {
                            entry.insert(map.next_value::<String>()?);
                        }
                        Entry::Occupied(_) => {
                            return Err(M::Error::custom("duplicate environment key"));
                        }
                    }
                }
                Ok(Environment(values))
            }
        }

        deserializer.deserialize_map(EnvironmentVisitor)
    }
}
