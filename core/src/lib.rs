use serde::Serialize;
use std::path::Path;
#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Protected,
    AlreadyRecovered,
    Unprotected,
    MissingProject,
    MissingVba,
    Unknown,
}
#[derive(Debug, Serialize)]
pub struct Inspection {
    pub status: Status,
    pub can_recover: bool,
    pub filename: String,
    pub details: String,
    pub warnings: Vec<String>,
}
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct Error(pub String);
#[derive(Debug, Serialize)]
pub struct Report {
    pub status: String,
    pub filename: String,
    pub output: String,
    pub details: String,
    pub warnings: Vec<String>,
    pub verified: bool,
}
pub fn recover(src: &Path, dst: &Path) -> Result<Report, Error> {
    use std::io::{Seek, SeekFrom, Write};
    extension(dst)?;
    if dst.try_exists()? {
        if same_file::is_same_file(src, dst)? || src.canonicalize()? == dst.canonicalize()? {
            return Err(Error(
                "Source and destination are the same file (including hardlinks)".into(),
            ));
        }
        return Err(Error(
            "Destination already exists; refusing overwrite".into(),
        ));
    }
    let (mut z, bytes) = load(src)?;
    let bytes = bytes.ok_or_else(|| Error("No VBA project present".into()))?;
    let p = project(&bytes)?;
    let (status, offset) = classify(p.as_deref())?;
    if status != Status::Protected {
        return Err(Error(format!(
            "No recoverable password protection: {status:?}"
        )));
    }
    let offset = offset.ok_or_else(|| Error("Missing DPB offset".into()))?;
    let mut changed = cfb::CompoundFile::open(Cursor::new(bytes.clone()))?;
    {
        let mut stream = changed.open_stream("/PROJECT")?;
        stream.seek(SeekFrom::Start(offset as u64))?;
        stream.write_all(b"x")?;
        stream.flush()?;
    }
    changed.flush()?;
    let changed = changed.into_inner().into_inner();
    verify_vba(&bytes, &changed, offset)?;
    let parent = dst
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    {
        let mut writer = zip::ZipWriter::new(temp.as_file_mut());
        writer.set_raw_comment(z.comment().into());
        for i in 0..z.len() {
            let f = z.by_index(i)?;
            if f.name() != VBA {
                writer.raw_copy_file(f)?;
            } else {
                let options = f.options();
                writer.start_file(VBA, options)?;
                writer.write_all(&changed)?;
            }
        }
        writer.finish()?;
    }
    restore_zip_attributes(src, &mut z, temp.path())?;
    temp.as_file().sync_all()?;
    verify_output(&mut z, temp.path(), &bytes, &changed, offset)?;
    temp.persist_noclobber(dst).map_err(|e| {
        Error(format!(
            "Could not publish output without overwrite: {}",
            e.error
        ))
    })?;
    Ok(Report{status:"recovered".into(),filename:src.file_name().unwrap_or_default().to_string_lossy().into_owned(),output:dst.to_string_lossy().into_owned(),details:"Created verified copy; changed only DPB marker in root PROJECT to DPx; no macros executed".into(),warnings:vec!["VBA editor recovery is not password decryption. Open the copy only if you trust its macros; Excel may prompt to repair project protection. Compatibility has not been tested in Excel.".into()],verified:true})
}
fn verify_vba(before: &[u8], after: &[u8], offset: usize) -> Result<(), Error> {
    if before.len() != after.len() || before.iter().zip(after).filter(|(a, b)| a != b).count() != 1
    {
        return Err(Error(
            "VBA verification failed: expected exactly one changed byte".into(),
        ));
    }
    let mut expected = project(before)?.ok_or_else(|| Error("Missing PROJECT".into()))?;
    if offset >= expected.len() {
        return Err(Error("Invalid PROJECT offset".into()));
    }
    expected[offset] = b'x';
    if project(after)?.as_deref() != Some(expected.as_slice()) {
        return Err(Error("VBA verification failed: PROJECT mismatch".into()));
    }
    if classify(Some(&expected))?.0 != Status::AlreadyRecovered {
        return Err(Error("VBA verification failed: marker unchanged".into()));
    }
    Ok(())
}
// zip::raw_copy_file preserves compressed data but normalizes DOS attributes to Unix.
// Restore source creator platform and raw attributes in central records, without touching payloads.
fn restore_zip_attributes(
    src: &Path,
    source: &mut zip::ZipArchive<File>,
    path: &Path,
) -> Result<(), Error> {
    use std::io::{Seek, SeekFrom, Write};
    let mut input = File::open(src)?;
    let mut output_archive = zip::ZipArchive::new(File::open(path)?)?;
    let mut records = Vec::new();
    for i in 0..source.len() {
        let a = source.by_index(i)?;
        let b = output_archive.by_index(i)?;
        records.push((a.central_header_start(), b.central_header_start()));
    }
    drop(output_archive);
    let mut output = std::fs::OpenOptions::new().write(true).open(path)?;
    for (a, b) in records {
        for (offset, size) in [(4, 2), (38, 4)] {
            let mut value = vec![0; size];
            input.seek(SeekFrom::Start(a + offset))?;
            input.read_exact(&mut value)?;
            output.seek(SeekFrom::Start(b + offset))?;
            output.write_all(&value)?;
        }
    }
    output.flush()?;
    Ok(())
}
fn verify_output(
    source: &mut zip::ZipArchive<File>,
    path: &Path,
    before: &[u8],
    changed: &[u8],
    offset: usize,
) -> Result<(), Error> {
    let mut output = zip::ZipArchive::new(File::open(path)?)?;
    if source.len() != output.len() || source.comment() != output.comment() {
        return Err(Error("Output archive structure changed".into()));
    }
    for i in 0..source.len() {
        let mut a = source.by_index(i)?;
        let mut b = output.by_index(i)?;
        if a.name() != b.name()
            || a.compression() != b.compression()
            || a.comment() != b.comment()
            || a.last_modified() != b.last_modified()
            || a.unix_mode() != b.unix_mode()
        {
            return Err(Error(format!(
                "Output entry metadata changed: {} => {} compression {:?}/{:?} comment {:?}/{:?} time {:?}/{:?} mode {:?}/{:?}",
                a.name(),
                b.name(),
                a.compression(),
                b.compression(),
                a.comment(),
                b.comment(),
                a.last_modified(),
                b.last_modified(),
                a.unix_mode(),
                b.unix_mode()
            )));
        }
        if a.name() == VBA {
            let mut bytes = Vec::new();
            b.read_to_end(&mut bytes)?;
            if bytes != changed {
                return Err(Error("Output VBA mismatch".into()));
            }
            verify_vba(before, &bytes, offset)?;
        } else {
            let mut ab = [0u8; 65536];
            let mut bb = [0u8; 65536];
            loop {
                let an = read_chunk(&mut a, &mut ab)?;
                let bn = read_chunk(&mut b, &mut bb)?;
                if an != bn || ab[..an] != bb[..bn] {
                    return Err(Error("Non-VBA entry changed".into()));
                }
                if an == 0 {
                    break;
                }
            }
        }
    }
    Ok(())
}
fn read_chunk(reader: &mut impl std::io::Read, buf: &mut [u8]) -> Result<usize, Error> {
    let mut n = 0;
    while n < buf.len() {
        let got = reader.read(&mut buf[n..])?;
        if got == 0 {
            break;
        }
        n += got;
    }
    Ok(n)
}
use std::{
    fs::File,
    io::{Cursor, Read},
};
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self(e.to_string())
    }
}
impl From<zip::result::ZipError> for Error {
    fn from(e: zip::result::ZipError) -> Self {
        Self(e.to_string())
    }
}
fn project(bytes: &[u8]) -> Result<Option<Vec<u8>>, Error> {
    let mut c = cfb::CompoundFile::open(Cursor::new(bytes))?;
    if !c.exists("/PROJECT") {
        return Ok(None);
    }
    let mut p = Vec::new();
    let mut stream = c.open_stream("/PROJECT")?;
    if stream.len() > 1024 * 1024 {
        return Err(Error("PROJECT excede limite de 1 MiB".into()));
    }
    stream.read_to_end(&mut p)?;
    Ok(Some(p))
}
// MS-OVBA 2.4.3.2. Only the no-password sentinel is exposed; decoded secrets are not returned.
fn password_present(hex: &[u8]) -> Result<bool, Error> {
    let bytes: Vec<u8> = hex
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| {
            u8::from_str_radix(std::str::from_utf8(c).unwrap_or(""), 16)
                .map_err(|e| Error(e.to_string()))
        })
        .collect::<Result<_, _>>()?;
    if bytes.len() < 8 || bytes[0] ^ bytes[1] != 2 {
        return Err(Error("Invalid DPB encryption version/length".into()));
    }
    let seed = bytes[0];
    let ignored = ((seed & 6) / 2) as usize;
    let (mut prev_plain, mut prev_enc, mut prev2_enc) = (bytes[2] ^ seed, bytes[2], bytes[1]);
    let mut decoded = Vec::new();
    for &enc in &bytes[3..] {
        let plain = enc ^ prev2_enc.wrapping_add(prev_plain);
        decoded.push(plain);
        prev2_enc = prev_enc;
        prev_enc = enc;
        prev_plain = plain;
    }
    if decoded.len() < ignored + 4 {
        return Err(Error("Truncated DPB data".into()));
    }
    let len = u32::from_le_bytes(
        decoded[ignored..ignored + 4]
            .try_into()
            .map_err(|_| Error("DPB length".into()))?,
    ) as usize;
    let data = &decoded[ignored + 4..];
    if data.len() != len {
        return Err(Error("DPB declared data length mismatch".into()));
    }
    if data == [0] {
        return Ok(false);
    }
    let hash_candidate = len == 29 && data[0] == 0xFF && data[28] == 0 && !data[4..28].contains(&0);
    let legacy_candidate = len > 1 && data.last() == Some(&0) && !data[..len - 1].contains(&0);
    if hash_candidate || legacy_candidate {
        Ok(true)
    } else {
        Err(Error("Invalid DPB password data structure".into()))
    }
}
fn classify(p: Option<&[u8]>) -> Result<(Status, Option<usize>), Error> {
    let Some(p) = p else {
        return Ok((Status::MissingProject, None));
    };
    if p.iter()
        .any(|b| *b == 0 || (*b < 32 && !b"\r\n\t".contains(b)))
    {
        return Err(Error("Unreadable PROJECT text".into()));
    }
    let mut found = None;
    let mut offset = 0;
    for raw in p.split_inclusive(|b| *b == b'\n') {
        let line = raw.strip_suffix(b"\n").unwrap_or(raw);
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        let trimmed = line.trim_ascii_start();
        if trimmed.len() >= 3
            && (trimmed[..3].eq_ignore_ascii_case(b"DPB")
                || trimmed[..3].eq_ignore_ascii_case(b"DPX"))
        {
            if found.is_some()
                || trimmed.len() != line.len()
                || line.len() < 7
                || &line[3..5] != b"=\""
                || line.last() != Some(&b'\"')
            {
                return Err(Error(
                    "Ambiguous or malformed PROJECT protection line".into(),
                ));
            }
            let hex = &line[5..line.len() - 1];
            if hex.is_empty() || hex.len() % 2 != 0 || !hex.iter().all(u8::is_ascii_hexdigit) {
                return Err(Error(
                    "Invalid protection value; expected nonempty hex".into(),
                ));
            }
            found = Some((
                if line[..3].eq_ignore_ascii_case(b"DPB") {
                    if password_present(hex)? {
                        Status::Protected
                    } else {
                        Status::Unprotected
                    }
                } else {
                    Status::AlreadyRecovered
                },
                Some(offset + 2),
            ));
        }
        offset += raw.len();
    }
    Ok(found.unwrap_or((Status::Unknown, None)))
}
const VBA: &str = "xl/vbaProject.bin";
pub const MAX_FILE_SIZE: u64 = 1024 * 1024 * 1024;
pub const MAX_VBA_SIZE: u64 = 100 * 1024 * 1024;
const MAX_XML_SIZE: u64 = 10 * 1024 * 1024;
fn extension(path: &Path) -> Result<(), Error> {
    if !path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("xlsm"))
    {
        return Err(Error("Only .xlsm files are supported; legacy XLS and opening-password encryption are not supported".into()));
    }
    Ok(())
}
fn read_entry(z: &mut zip::ZipArchive<File>, name: &str, limit: u64) -> Result<Vec<u8>, Error> {
    let mut f = z.by_name(name)?;
    if f.size() > limit {
        return Err(Error(format!("Entry exceeds size limit: {name}")));
    }
    let mut bytes = Vec::new();
    (&mut f).take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(Error("Decompressed size limit exceeded".into()));
    }
    Ok(bytes)
}
fn validate_xml(
    bytes: &[u8],
    root: &[u8],
    namespace: &str,
    content_types: bool,
) -> Result<(), Error> {
    use quick_xml::{Reader, events::Event};
    let mut reader = Reader::from_reader(bytes);
    let mut depth = 0usize;
    let mut seen = false;
    let mut closed = false;
    let mut macro_type = false;
    loop {
        let event = reader
            .read_event()
            .map_err(|e| Error(format!("Invalid OOXML XML: {e}")))?;
        let empty = matches!(event, Event::Empty(_));
        match event {
            Event::Start(ref e) | Event::Empty(ref e) => {
                if depth == 0 {
                    if seen || closed || e.name().as_ref() != root {
                        return Err(Error("Invalid OOXML root".into()));
                    }
                    seen = true;
                    let mut ns = false;
                    for a in e.attributes() {
                        let a = a.map_err(|e| Error(e.to_string()))?;
                        if a.key.as_ref() == b"xmlns" && a.value.as_ref() == namespace.as_bytes() {
                            ns = true;
                        }
                    }
                    if !ns {
                        return Err(Error("Missing OOXML namespace".into()));
                    }
                }
                if content_types && e.name().as_ref() == b"Override" {
                    let mut part = false;
                    let mut ct = false;
                    for a in e.attributes() {
                        let a = a.map_err(|e| Error(e.to_string()))?;
                        if a.key.as_ref() == b"PartName" && a.value.as_ref() == b"/xl/workbook.xml"
                        {
                            part = true
                        }
                        if a.key.as_ref() == b"ContentType"
                            && a.value.as_ref()
                                == b"application/vnd.ms-excel.sheet.macroEnabled.main+xml"
                        {
                            ct = true
                        }
                    }
                    macro_type |= part && ct;
                }
                if empty {
                    if depth == 0 {
                        closed = true;
                    }
                } else {
                    depth += 1;
                }
                if depth > 256 {
                    return Err(Error("XML nesting limit exceeded".into()));
                }
            }
            Event::End(_) => {
                if depth == 0 {
                    return Err(Error("Invalid XML nesting".into()));
                }
                depth -= 1;
                if depth == 0 {
                    closed = true;
                }
            }
            Event::DocType(_) => return Err(Error("XML DTD is not supported".into())),
            Event::Text(t) => {
                if depth == 0 && !t.as_ref().iter().all(u8::is_ascii_whitespace) {
                    return Err(Error("Text outside OOXML root".into()));
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if !seen || !closed || depth != 0 || (content_types && !macro_type) {
        return Err(Error(
            "Missing or invalid XLSM OOXML content types/workbook".into(),
        ));
    }
    Ok(())
}
// zip 2.x deduplicates exact names internally. Scan physical central records as well.
fn validate_central(path: &Path, z: &zip::ZipArchive<File>) -> Result<(), Error> {
    use std::io::{Seek, SeekFrom};
    let mut f = File::open(path)?;
    f.seek(SeekFrom::Start(z.central_directory_start()))?;
    let mut count = 0;
    let mut names = std::collections::HashSet::new();
    loop {
        let mut sig = [0u8; 4];
        f.read_exact(&mut sig)?;
        if sig != *b"PK\x01\x02" {
            break;
        }
        let mut header = [0u8; 42];
        f.read_exact(&mut header)?;
        let n = u16::from_le_bytes([header[24], header[25]]) as usize;
        let e = u16::from_le_bytes([header[26], header[27]]) as u64;
        let c = u16::from_le_bytes([header[28], header[29]]) as u64;
        let mut name = vec![0; n];
        f.read_exact(&mut name)?;
        if !names.insert(name) {
            return Err(Error("Duplicate physical ZIP entry".into()));
        }
        f.seek(SeekFrom::Current((e + c) as i64))?;
        count += 1;
        if count > 10000 {
            return Err(Error("ZIP entry count exceeds limit".into()));
        }
    }
    if count != z.len() {
        return Err(Error("ZIP central directory count mismatch".into()));
    }
    Ok(())
}
fn load(path: &Path) -> Result<(zip::ZipArchive<File>, Option<Vec<u8>>), Error> {
    extension(path)?;
    let f = File::open(path)?;
    if f.metadata()?.len() > MAX_FILE_SIZE {
        return Err(Error("File exceeds 1 GiB limit".into()));
    }
    let mut z = zip::ZipArchive::new(f).map_err(|e| {
        Error(format!(
            "Not a readable OOXML ZIP (opening-password encryption and XLS are unsupported): {e}"
        ))
    })?;
    validate_central(path, &z)?;
    if z.len() > 10000 {
        return Err(Error("ZIP entry count exceeds limit".into()));
    }
    let mut names = std::collections::HashSet::new();
    let mut total = 0u64;
    for i in 0..z.len() {
        let f = z.by_index(i)?;
        let n = f.name();
        let lower = n.to_ascii_lowercase();
        if n.is_empty()
            || n.starts_with('/')
            || n.contains(['\\', ':', '\0'])
            || n.trim_end_matches('/')
                .split('/')
                .any(|p| p == ".." || p == "." || p.is_empty() || p.ends_with([' ', '.']))
            || !names.insert(lower.clone())
        {
            return Err(Error("Unsafe or duplicate ZIP entry".into()));
        }
        if lower
            .split('/')
            .any(|p| p == "_xmlsignatures" || p.starts_with("vbaprojectsignature"))
        {
            return Err(Error("Arquivo ou VBA assinado digitalmente: operação recusada para evitar alterar assinatura ou confiança".into()));
        }
        if f.is_symlink() || f.encrypted() {
            return Err(Error("ZIP symlinks/encryption unsupported".into()));
        }
        total = total
            .checked_add(f.size())
            .ok_or_else(|| Error("ZIP size overflow".into()))?;
        if total > MAX_FILE_SIZE {
            return Err(Error("ZIP expanded total exceeds 1 GiB limit".into()));
        }
        if n == VBA && f.size() > MAX_VBA_SIZE {
            return Err(Error("VBA exceeds 100 MiB limit".into()));
        }
    }
    let types = read_entry(&mut z, "[Content_Types].xml", MAX_XML_SIZE)?;
    validate_xml(
        &types,
        b"Types",
        "http://schemas.openxmlformats.org/package/2006/content-types",
        true,
    )?;
    let book = read_entry(&mut z, "xl/workbook.xml", MAX_XML_SIZE)?;
    validate_xml(
        &book,
        b"workbook",
        "http://schemas.openxmlformats.org/spreadsheetml/2006/main",
        false,
    )?;
    let bytes = if names.contains(&VBA.to_ascii_lowercase()) {
        Some(read_entry(&mut z, VBA, MAX_VBA_SIZE)?)
    } else {
        None
    };
    Ok((z, bytes))
}
pub fn inspect(path: &Path) -> Result<Inspection, Error> {
    let (_, bytes) = load(path)?;
    let status = if let Some(b) = bytes {
        let p = project(&b)?;
        classify(p.as_deref())?.0
    } else {
        Status::MissingVba
    };
    Ok(Inspection {
        can_recover: status == Status::Protected,
        status,
        filename: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        details: "VBA project inspected; no macros executed".into(),
        warnings: vec![],
    })
}
