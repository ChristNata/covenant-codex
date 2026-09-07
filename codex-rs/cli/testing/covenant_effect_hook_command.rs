//! Private Windows SessionStart marker command for the owned effect fixture.
use super::FixtureFailure;
use super::MarkerExpectation;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

const SCRIPT: &str = r#"param([string]$Marker, [string]$Nonce, [string]$ExpectedCwd, [string]$ExpectedModel)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
function Refuse-HookInput {
    $stream = [Console]::OpenStandardError()
    try {
        $bytes = [Text.Encoding]::ASCII.GetBytes("covenant-hook-refused-v1`n")
        $stream.Write($bytes, 0, $bytes.Length)
        $stream.Flush()
    } finally { $stream.Dispose() }
    exit 73
}
$utf8 = [Text.UTF8Encoding]::new($false, $true)
$stdin = [Console]::OpenStandardInput()
$reader = [IO.StreamReader]::new($stdin, $utf8, $false, 1024, $true)
$characters = New-Object char[] 8193
$used = 0
try {
    while ($used -lt $characters.Length) {
        $count = $reader.Read($characters, $used, $characters.Length - $used)
        if ($count -eq 0) { break }
        $used += $count
    }
} finally { $reader.Dispose(); $stdin.Dispose() }
if ($used -gt 8192) { Refuse-HookInput }
$text = [string]::new($characters, 0, $used)
$timeout = [TimeSpan]::FromMilliseconds(100)
$options = [Text.RegularExpressions.RegexOptions]::CultureInvariant
$start = [regex]::Match($text, '\A[ \t\r\n]*\{', $options, $timeout)
if (-not $start.Success) { Refuse-HookInput }
# Only JSON strings or null are valid values in this closed, flat hook input.
$quoted = '"(?:[^"\\\x00-\x1f]|\\(?:["\\/bfnrt]|u[0-9a-fA-F]{4}))*"'
$pattern = '\G[ \t\r\n]*(?<key>' + $quoted + ')[ \t\r\n]*:[ \t\r\n]*(?<value>' +
    $quoted + '|null)[ \t\r\n]*(?<end>[,}])'
$member = [regex]::new($pattern, $options, $timeout)
$names = @('session_id', 'transcript_path', 'cwd', 'hook_event_name', 'model', 'permission_mode', 'source')
$values = [ordered]@{}
$position = $start.Length
$closed = $false
while ($values.Count -lt 7) {
    $match = $member.Match($text, $position)
    if (-not $match.Success) { Refuse-HookInput }
    $raw = $match.Groups['key'].Value
    $name = [regex]::Unescape($raw.Substring(1, $raw.Length - 2))
    if ($names -cnotcontains $name -or $values.Contains($name)) { Refuse-HookInput }
    $raw = $match.Groups['value'].Value
    $value = $null
    if ($raw -cne 'null') {
        $value = [regex]::Unescape($raw.Substring(1, $raw.Length - 2))
        try { $null = $utf8.GetByteCount($value) }
        catch [Text.EncoderFallbackException] { Refuse-HookInput }
    } elseif ($name -cne 'transcript_path') { Refuse-HookInput }
    $values.Add($name, $value)
    $position = $match.Index + $match.Length
    if ($match.Groups['end'].Value -ceq '}') { $closed = $true; break }
}
if (-not $closed -or $values.Count -ne 7 -or
    -not [regex]::IsMatch($text.Substring($position), '\A[ \t\r\n]*\z', $options, $timeout)) {
    Refuse-HookInput
}
if ([string]::IsNullOrEmpty($values['session_id']) -or
    $values['cwd'] -cne $ExpectedCwd -or $values['model'] -cne $ExpectedModel -or
    $values['hook_event_name'] -cne 'SessionStart' -or $values['source'] -cne 'startup') {
    Refuse-HookInput
}
$record = [ordered]@{nonce=$Nonce; pid=[uint32]$PID; input=$values}
$json = ConvertTo-Json -InputObject $record -Compress -Depth 4 -ErrorAction Stop
$bytes = $utf8.GetBytes($json)
if ($bytes.Length -gt 16384) { Refuse-HookInput }
try { $file = [IO.File]::Open($Marker, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::None) }
catch [IO.IOException] {
    $cause = $_.Exception.GetBaseException()
    # Exact HRESULT_FROM_WIN32(ERROR_FILE_EXISTS / ERROR_ALREADY_EXISTS) only.
    if ($cause -is [IO.IOException] -and ($cause.HResult -eq -2147024816 -or $cause.HResult -eq -2147024713)) {
        Refuse-HookInput
    }
    throw
}
try { $file.Write($bytes, 0, $bytes.Length); $file.Flush($true) }
finally { $file.Dispose() }
exit 0
"#;

pub(super) struct HookCommand {
    command_line: String,
}

fn literal(value: &str) -> Result<String, FixtureFailure> {
    if value.encode_utf16().count() > 8192 || value.chars().any(char::is_control) {
        return Err(FixtureFailure::Limit);
    }
    Ok(format!("'{}'", value.replace('\'', "''")))
}

fn owned_path(path: &Path) -> Result<&str, FixtureFailure> {
    if !path.is_absolute() || path.file_name().is_none() || !path.parent().is_some_and(Path::is_dir)
    {
        return Err(FixtureFailure::Marker);
    }
    path.to_str().ok_or(FixtureFailure::Marker)
}

impl HookCommand {
    pub(super) fn prepare(
        script: &Path,
        marker: &Path,
        expected: &MarkerExpectation,
    ) -> Result<Self, FixtureFailure> {
        if script == marker || expected.nonce.is_empty() || expected.model.is_empty() {
            return Err(FixtureFailure::Marker);
        }
        let script_argument = literal(owned_path(script)?)?;
        let marker_argument = literal(owned_path(marker)?)?;
        let nonce = literal(&expected.nonce)?;
        let cwd = literal(&expected.cwd)?;
        let model = literal(&expected.model)?;
        let command_line = format!(
            "$ErrorActionPreference = 'Stop'; Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass -Force -ErrorAction Stop; & {script_argument} -Marker {marker_argument} -Nonce {nonce} -ExpectedCwd {cwd} -ExpectedModel {model}; exit $LASTEXITCODE"
        );
        if command_line.encode_utf16().count() > 8192 {
            return Err(FixtureFailure::Limit);
        }
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(script)
            .map_err(|_| FixtureFailure::Marker)?;
        file.write_all(&[0xef, 0xbb, 0xbf])
            .map_err(|_| FixtureFailure::Marker)?;
        file.write_all(SCRIPT.as_bytes())
            .map_err(|_| FixtureFailure::Marker)?;
        file.flush().map_err(|_| FixtureFailure::Marker)?;
        file.sync_all().map_err(|_| FixtureFailure::Marker)?;
        drop(file);
        Ok(Self { command_line })
    }

    pub(super) fn command_line(&self) -> &str {
        &self.command_line
    }
}
