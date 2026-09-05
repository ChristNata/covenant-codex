use serde::Deserialize;
use serde::Serialize;

use crate::numbers::NonnegativeInteger;
use crate::numbers::Version;
use crate::values::Digest;
use crate::values::Environment;
use crate::values::NonemptyString;
use crate::values::NonemptyVec;
use crate::values::Object;
use crate::values::present;
use crate::values::string_enum;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Envelope {
    version: Version,
    #[serde(deserialize_with = "string_enum")]
    pub(crate) kind: Kind,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub(crate) exec: Option<Object<ExecCall>>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub(crate) patch: Option<Object<PatchCall>>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Kind {
    Exec,
    Patch,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExecCall {
    program: NonemptyString,
    argv: Vec<String>,
    cwd: NonemptyString,
    env: Environment,
    sandbox: NonemptyString,
    network: bool,
    tty: bool,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PatchCall {
    paths: NonemptyVec<NonemptyString>,
    pub(crate) operations: NonemptyVec<Object<PatchOperation>>,
    permissions: Object<PatchPermissions>,
    pub(crate) resolved_identities: NonemptyVec<Object<ResolvedIdentity>>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PatchOperation {
    path: NonemptyString,
    #[serde(deserialize_with = "string_enum")]
    pub(crate) op: OperationKind,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub(crate) destination: Option<NonemptyString>,
    hunks: Vec<Object<Hunk>>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub(crate) pre_image_digest: Option<Digest>,
}

#[derive(Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum OperationKind {
    Add,
    Update,
    Delete,
    Move,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Hunk {
    old_start: NonnegativeInteger,
    old_lines: NonnegativeInteger,
    new_start: NonnegativeInteger,
    new_lines: NonnegativeInteger,
    lines: Vec<Object<HunkLine>>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct HunkLine {
    #[serde(deserialize_with = "string_enum")]
    kind: HunkLineKind,
    text: String,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum HunkLineKind {
    Context,
    Add,
    Remove,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PatchPermissions {
    sandbox: NonemptyString,
    write_roots: Vec<NonemptyString>,
    network: bool,
    tty: bool,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ResolvedIdentity {
    path: NonemptyString,
    #[serde(deserialize_with = "string_enum")]
    pub(crate) kind: IdentityKind,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub(crate) target: Option<NonemptyString>,
    pub(crate) ancestor_identities: Vec<Object<AncestorIdentity>>,
    win32_normalized: NonemptyString,
    volume_serial: NonemptyString,
    file_index: NonemptyString,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub(crate) pre_image_digest: Option<Digest>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AncestorIdentity {
    path: NonemptyString,
    #[serde(deserialize_with = "string_enum")]
    pub(crate) kind: IdentityKind,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub(crate) target: Option<NonemptyString>,
    win32_normalized: NonemptyString,
    volume_serial: NonemptyString,
    file_index: NonemptyString,
}

#[derive(Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum IdentityKind {
    File,
    Directory,
    Symlink,
    Junction,
}
