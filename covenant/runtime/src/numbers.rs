use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::de::Error;
use serde_json::Number;
use serde_json::value::RawValue;

#[derive(Serialize)]
#[serde(transparent)]
pub(crate) struct NonnegativeInteger(Number);

#[derive(Serialize)]
#[serde(transparent)]
pub(crate) struct Version(Number);

impl<'de> Deserialize<'de> for NonnegativeInteger {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        integer(deserializer).map(|(number, _)| Self(number))
    }
}

impl<'de> Deserialize<'de> for Version {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let (number, is_one) = integer(deserializer)?;
        if !is_one {
            return Err(D::Error::custom("unsupported schema version"));
        }
        Ok(Self(number))
    }
}

// JSON Schema integers have no machine-width limit and may use decimal/exponent
// notation. Compare the decimal scale without rounding or expanding the exponent.
fn integer<'de, D: Deserializer<'de>>(deserializer: D) -> Result<(Number, bool), D::Error> {
    // Capture the actual token: Number's arbitrary-precision Deserialize also
    // accepts its internal map representation, which is not a JSON number.
    let raw = Box::<RawValue>::deserialize(deserializer)?;
    let number = raw
        .get()
        .parse::<Number>()
        .map_err(|_| D::Error::custom("expected a JSON number"))?;
    let text = number.as_str();
    let negative = text.starts_with('-');
    let unsigned = text.strip_prefix('-').unwrap_or(text);
    let (significand, exponent) = unsigned.split_once(['e', 'E']).unwrap_or((unsigned, "0"));
    let mut digits = significand
        .bytes()
        .filter(|digit| *digit != b'.')
        .skip_while(|digit| *digit == b'0');
    let Some(first) = digits.next() else {
        return Ok((number, false));
    };
    if negative {
        return Err(D::Error::custom("expected a nonnegative integer"));
    }
    let unit_significand = first == b'1' && digits.all(|digit| digit == b'0');
    let fraction_len = significand
        .split_once('.')
        .map_or(/*default*/ 0, |(_, fraction)| fraction.len());
    let trailing_zeros = significand
        .bytes()
        .rev()
        .filter(|digit| *digit != b'.')
        .take_while(|digit| *digit == b'0')
        .count();
    let required_exponent = fraction_len as i128 - trailing_zeros as i128;
    // Only the ordering against an input-length-bounded scale matters. Exponents
    // outside i128 cannot equal that scale, so saturation preserves the comparison.
    let exponent = exponent.parse::<i128>().unwrap_or_else(|_| {
        if exponent.starts_with('-') {
            i128::MIN
        } else {
            i128::MAX
        }
    });
    if exponent < required_exponent {
        return Err(D::Error::custom("expected an integer"));
    }
    let is_one = unit_significand && exponent == required_exponent;
    Ok((number, is_one))
}
