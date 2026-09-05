use serde::Serialize;

use crate::values::Object;
use crate::wire::Envelope;
use crate::wire::IdentityKind;
use crate::wire::Kind;
use crate::wire::OperationKind;

/// An immutable request validated against the Decide v1 wire contract.
///
/// Construction is restricted to decoding. Serialization preserves the complete
/// typed payload; this type deliberately provides no secret-bearing Debug output.
#[derive(Serialize)]
#[serde(transparent)]
pub struct DecideV1(Object<Envelope>);

/// A decoding refusal that never retains or displays submitted content.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidDecide;

impl std::fmt::Display for InvalidDecide {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("invalid Decide v1 request")
    }
}

impl std::error::Error for InvalidDecide {}

impl DecideV1 {
    /// Decode a schema-valid request, additionally refusing duplicate object keys.
    pub fn decode(bytes: &[u8]) -> Result<Self, InvalidDecide> {
        let request: Object<Envelope> = serde_json::from_slice(bytes).map_err(|_| InvalidDecide)?;
        match request.kind {
            Kind::Exec => {
                if request.exec.is_none() || request.patch.is_some() {
                    return Err(InvalidDecide);
                }
            }
            Kind::Patch => {
                if request.exec.is_some() {
                    return Err(InvalidDecide);
                }
                let patch = request.patch.as_ref().ok_or(InvalidDecide)?;
                for operation in &patch.operations.0 {
                    if (operation.op == OperationKind::Move && operation.destination.is_none())
                        || (operation.op != OperationKind::Add
                            && operation.pre_image_digest.is_none())
                    {
                        return Err(InvalidDecide);
                    }
                }
                for identity in &patch.resolved_identities.0 {
                    if (identity.kind == IdentityKind::File && identity.pre_image_digest.is_none())
                        || (matches!(
                            identity.kind,
                            IdentityKind::Symlink | IdentityKind::Junction
                        ) && identity.target.is_none())
                    {
                        return Err(InvalidDecide);
                    }
                    for ancestor in &identity.ancestor_identities {
                        if matches!(
                            ancestor.kind,
                            IdentityKind::Symlink | IdentityKind::Junction
                        ) && ancestor.target.is_none()
                        {
                            return Err(InvalidDecide);
                        }
                    }
                }
            }
        }
        Ok(Self(request))
    }
}
