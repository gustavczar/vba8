//! All fixtures are SYNTHETIC, not Excel-generated and contain no executable VBA.
use std::{
    fs::{self, File},
    io::{Cursor, Read, Write},
    path::Path,
};
use vba8_core::{Status, inspect, recover};
#[test]
fn recovery_changes_only_root_project_marker_and_preserves_package() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("synthetic.xlsm");
    let dst = d.path().join("copy.xlsm");
    let text = format!(
        "ID=\"SYNTHETIC\"\r\ndPb=\"{}\"\r\n",
        encoded(b"synthetic\0", 6)
    );
    archive(&src, Some(text.as_bytes()));
    let original = fs::read(&src).unwrap();
    let report = recover(&src, &dst).unwrap();
    assert!(report.verified);
    assert_eq!(inspect(&dst).unwrap().status, Status::AlreadyRecovered);
    assert_eq!(fs::read(&src).unwrap(), original);
    let mut a = zip::ZipArchive::new(File::open(&src).unwrap()).unwrap();
    let mut b = zip::ZipArchive::new(File::open(&dst).unwrap()).unwrap();
    assert_eq!(a.comment(), b.comment());
    assert_eq!(a.len(), b.len());
    for i in 0..a.len() {
        let mut x = a.by_index(i).unwrap();
        let mut y = b.by_index(i).unwrap();
        assert_eq!(x.name(), y.name());
        assert_eq!(x.comment(), y.comment());
        assert_eq!(x.unix_mode(), y.unix_mode());
        assert_eq!(x.last_modified(), y.last_modified());
        let (mut xb, mut yb) = (Vec::new(), Vec::new());
        x.read_to_end(&mut xb).unwrap();
        y.read_to_end(&mut yb).unwrap();
        if x.name() != "xl/vbaProject.bin" {
            assert_eq!(xb, yb)
        } else {
            assert_ne!(xb, yb);
            let mut c = cfb::CompoundFile::open(Cursor::new(yb)).unwrap();
            let mut p = Vec::new();
            c.open_stream("/PROJECT")
                .unwrap()
                .read_to_end(&mut p)
                .unwrap();
            assert_eq!(
                p,
                String::from_utf8(text.as_bytes().to_vec())
                    .unwrap()
                    .replace("dPb=", "dPx=")
                    .as_bytes()
            );
            let mut m = Vec::new();
            c.open_stream("/VBA/Module1")
                .unwrap()
                .read_to_end(&mut m)
                .unwrap();
            assert_eq!(m, b"SYNTHETIC MODULE: DPB=\"DECOY\"");
        }
    }
}
/// Requires independently downloaded, inert public fixtures; never executes Office or macros.
#[test]
#[ignore = "requires VBA8_FIXTURE_DIR and VBA8_NEGATIVE_FIXTURE; see docs/TESTING.md"]
fn real_protected_workbooks_recover_without_touching_source() {
    let fixture_dir = std::env::var("VBA8_FIXTURE_DIR")
        .expect("Set VBA8_FIXTURE_DIR to the public-protected fixture directory");
    let dir = std::path::Path::new(&fixture_dir);
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(dir.join("manifest.json")).unwrap()).unwrap();
    let paths: Vec<_> = manifest["fixtures"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            dir.join(
                std::path::Path::new(f["local_file"].as_str().unwrap())
                    .file_name()
                    .unwrap(),
            )
        })
        .collect();
    assert_eq!(paths.len(), 3);
    for src in paths {
        let before = fs::read(&src).unwrap();
        let i = inspect(&src).unwrap();
        assert_eq!(i.status, Status::Protected, "{}", src.display());
        assert!(i.can_recover);
        let d = tempfile::tempdir().unwrap();
        let dst = d.path().join("recovered.xlsm");
        let report = recover(&src, &dst).unwrap();
        assert!(report.verified);
        assert_eq!(inspect(&dst).unwrap().status, Status::AlreadyRecovered);
        assert_eq!(fs::read(&src).unwrap(), before);
        println!(
            "REAL PASS {} => {}",
            src.display(),
            serde_json::to_string(&report).unwrap()
        );
    }
    let negative = std::env::var("VBA8_NEGATIVE_FIXTURE")
        .expect("Set VBA8_NEGATIVE_FIXTURE to an unprotected XLSM control");
    assert_eq!(
        inspect(Path::new(&negative)).unwrap().status,
        Status::Unprotected
    );
}
#[test]
fn rejects_invalid_29_byte_payload_and_oversized_project() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("synthetic.xlsm");
    let text = format!("DPB=\"{}\"", encoded(&[0u8; 29], 0));
    archive(&p, Some(text.as_bytes()));
    assert!(inspect(&p).is_err());
    archive(&p, Some(&vec![b'A'; 1024 * 1024 + 1]));
    assert!(inspect(&p).is_err());
}
#[test]
fn public_dpb_vectors_agree_with_independent_research() {
    for (hex, want) in [
        ("5557E86E18E919E919E9", Status::Unprotected),
        (
            "FCFE5054B04FCD4FCDB03350CD27DCB79AACA3F42F5179FFF4B1A293D0B04861AA321BF5767C",
            Status::Protected,
        ),
    ] {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("synthetic-container.xlsm");
        let text = format!("DPB=\"{hex}\"\r\n");
        archive(&p, Some(text.as_bytes()));
        let i = inspect(&p).unwrap();
        assert_eq!(i.can_recover, want == Status::Protected);
        assert_eq!(i.status, want);
    }
}
#[test]
fn rejects_ambiguous_workbook_aliases() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("synthetic.xlsm");
    package(
        &p,
        vec![
            ("[Content_Types].xml", TYPES.to_vec()),
            ("xl/workbook.xml", BOOK.to_vec()),
            ("xl/vbaProject.bin", vba(None)),
            ("xl/vbaProject.bin ", vec![]),
        ],
    );
    assert!(inspect(&p).is_err());
}
#[test]
fn recovery_failures_leave_source_and_destination_untouched() {
    let d = tempfile::tempdir().unwrap();
    let src = d.path().join("synthetic.xlsm");
    let dst = d.path().join("copy.xlsm");
    for project in [
        None,
        Some(b"Name=\"synthetic\"".as_slice()),
        Some(b"DPx=\"1234\"".as_slice()),
        Some(b"DPB=\"invalid\"".as_slice()),
        Some(format!("DPB=\"{}\"", encoded(b"\0", 6)).as_bytes()),
    ] {
        archive(&src, project);
        let before = fs::read(&src).unwrap();
        assert!(recover(&src, &dst).is_err());
        assert_eq!(fs::read(&src).unwrap(), before);
        assert!(!dst.exists());
        assert_eq!(fs::read_dir(d.path()).unwrap().count(), 1);
    }
    let text = format!("DPB=\"{}\"", encoded(b"synthetic\0", 2));
    archive(&src, Some(text.as_bytes()));
    let original = fs::read(&src).unwrap();
    assert!(recover(&src, &src).is_err());
    let alias = d.path().join("alias.xlsm");
    fs::hard_link(&src, &alias).unwrap();
    assert!(recover(&src, &alias).is_err());
    assert_eq!(fs::read(&src).unwrap(), original);
    fs::write(&dst, b"keep existing destination").unwrap();
    assert!(recover(&src, &dst).is_err());
    assert_eq!(fs::read(&dst).unwrap(), b"keep existing destination");
    assert_eq!(fs::read(&src).unwrap(), original);
}
#[test]
fn rejects_limits_before_loading_large_entries() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("synthetic.xlsm");
    File::create(&p)
        .unwrap()
        .set_len(vba8_core::MAX_FILE_SIZE + 1)
        .unwrap();
    assert!(inspect(&p).unwrap_err().to_string().contains("1 GiB"));
    archive(&p, None);
    let mut bytes = fs::read(&p).unwrap();
    let pos = bytes.windows(4).position(|x| x == b"PK\x01\x02").unwrap();
    let mut start = pos;
    loop {
        if &bytes[start..start + 4] != b"PK\x01\x02" {
            panic!("VBA entry not found")
        };
        let n = u16::from_le_bytes(bytes[start + 28..start + 30].try_into().unwrap()) as usize;
        let e = u16::from_le_bytes(bytes[start + 30..start + 32].try_into().unwrap()) as usize;
        let c = u16::from_le_bytes(bytes[start + 32..start + 34].try_into().unwrap()) as usize;
        if &bytes[start + 46..start + 46 + n] == b"xl/vbaProject.bin" {
            bytes[start + 24..start + 28]
                .copy_from_slice(&((vba8_core::MAX_VBA_SIZE + 1) as u32).to_le_bytes());
            break;
        }
        start += 46 + n + e + c;
    }
    fs::write(&p, bytes).unwrap();
    assert!(inspect(&p).unwrap_err().to_string().contains("100 MiB"));
}
#[test]
fn duplicate_zip_names_are_rejected_even_if_zip_reader_deduplicates() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("synthetic.xlsm");
    package(
        &p,
        vec![
            ("[Content_Types].xml", TYPES.to_vec()),
            ("xl/workbook.xml", BOOK.to_vec()),
            ("xl/duplicatexml", BOOK.to_vec()),
            ("xl/vbaProject.bin", vba(None)),
        ],
    );
    let mut bytes = fs::read(&p).unwrap();
    let old = b"xl/duplicatexml";
    let new = b"xl/workbook.xml";
    assert_eq!(old.len(), new.len());
    for start in (0..bytes.len() - old.len()).collect::<Vec<_>>() {
        if &bytes[start..start + old.len()] == old {
            bytes[start..start + old.len()].copy_from_slice(new);
        }
    }
    fs::write(&p, bytes).unwrap();
    assert!(inspect(&p).is_err());
}
use zip::{ZipWriter, write::SimpleFileOptions};
const TYPES: &[u8] = br#"<?xml version="1.0"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Override PartName="/xl/workbook.xml" ContentType="application/vnd.ms-excel.sheet.macroEnabled.main+xml"/><Default Extension="bin" ContentType="application/vnd.ms-office.vbaProject"/></Types>"#;
const BOOK: &[u8] =
    br#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"/>"#;
fn vba(project: Option<&[u8]>) -> Vec<u8> {
    let mut c = cfb::CompoundFile::create(Cursor::new(Vec::new())).unwrap();
    if let Some(p) = project {
        c.create_stream("/PROJECT").unwrap().write_all(p).unwrap();
    }
    c.create_storage("/VBA").unwrap();
    c.create_stream("/VBA/Module1")
        .unwrap()
        .write_all(b"SYNTHETIC MODULE: DPB=\"DECOY\"")
        .unwrap();
    c.into_inner().into_inner()
}
fn archive(path: &Path, project: Option<&[u8]>) {
    package(
        path,
        vec![
            ("[Content_Types].xml", TYPES.to_vec()),
            ("xl/workbook.xml", BOOK.to_vec()),
            ("xl/vbaProject.bin", vba(project)),
            ("xl/worksheets/sheet1.xml", b"<worksheet/>".to_vec()),
        ],
    );
}
fn package(path: &Path, entries: Vec<(&str, Vec<u8>)>) {
    let mut z = ZipWriter::new(File::create(path).unwrap());
    z.set_raw_comment(b"synthetic archive comment".to_vec().into());
    for (n, b) in entries {
        z.start_file(
            n,
            SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated)
                .unix_permissions(0o644),
        )
        .unwrap();
        z.write_all(&b).unwrap();
    }
    z.finish().unwrap();
}
#[test]
fn classifies_only_unambiguous_root_project_lines() {
    let protected = format!("dpb=\"{}\"\r\n", encoded(b"synthetic\0", 0));
    for (data, want) in [
        (Some(protected.as_bytes()), Status::Protected),
        (Some(b"DPx=\"1234\"\n".as_slice()), Status::AlreadyRecovered),
        (Some(b"Name=\"DPB=1234\"\r\n".as_slice()), Status::Unknown),
        (None, Status::MissingProject),
    ] {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("synthetic.xlsm");
        archive(&p, data);
        assert_eq!(inspect(&p).unwrap().status, want);
    }
    for bad in [
        b"DPB=\"1234\"\nDPB=\"5678\"\n".as_slice(),
        b"DPB=\"1234\"\nDPx=\"1234\"\n",
        b" DPB=\"1234\"\n",
        b"DPB=\"not hex\"\n",
        b"DPB=\"\"\n",
        b"DPB=\"12\"garbage\n",
        b"DPB=\"1234\"\rDPB=\"5678\"",
    ] {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("synthetic.xlsm");
        archive(&p, Some(bad));
        assert!(inspect(&p).is_err(), "accepted {bad:?}");
    }
}
#[test]
fn rejects_unsafe_or_non_xlsm_packages() {
    let cases = vec![
        vec![("xl/vbaProject.bin", vba(Some(b"DPB=\"1234\"")))],
        vec![
            ("[Content_Types].xml", b"<Types/>".to_vec()),
            ("xl/workbook.xml", BOOK.to_vec()),
            ("xl/vbaProject.bin", vba(None)),
        ],
        vec![
            ("[Content_Types].xml", TYPES.to_vec()),
            ("xl/workbook.xml", b"not xml".to_vec()),
            ("xl/vbaProject.bin", vba(None)),
        ],
        vec![
            ("[Content_Types].xml", TYPES.to_vec()),
            ("xl/workbook.xml", BOOK.to_vec()),
            ("../evil", vec![]),
        ],
        vec![
            ("[Content_Types].xml", TYPES.to_vec()),
            ("xl/workbook.xml", BOOK.to_vec()),
            ("xl/vbaProject.bin", vba(None)),
            ("xl/vbaProjectSignature.bin", vec![]),
        ],
        vec![
            ("[Content_Types].xml", TYPES.to_vec()),
            ("xl/workbook.xml", BOOK.to_vec()),
            ("_xmlsignatures/sig1.xml", vec![]),
        ],
        vec![
            ("[Content_Types].xml", TYPES.to_vec()),
            ("xl/workbook.xml", BOOK.to_vec()),
            ("xl/vbaProject.bin", b"invalid CFB".to_vec()),
        ],
    ];
    for entries in cases {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("synthetic.xlsm");
        package(&p, entries);
        assert!(inspect(&p).is_err(), "accepted {p:?}");
    }
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("synthetic.xlsx");
    archive(&p, Some(b"DPB=\"1234\""));
    assert!(inspect(&p).is_err());
    let p = d.path().join("synthetic.xlsm");
    package(
        &p,
        vec![
            ("[Content_Types].xml", TYPES.to_vec()),
            ("xl/workbook.xml", BOOK.to_vec()),
        ],
    );
    assert_eq!(inspect(&p).unwrap().status, Status::MissingVba);
}
// Independent synthetic encoder derived from MS-OVBA 2.4.3.2, not real passwords.
fn encoded(data: &[u8], seed: u8) -> String {
    let key = 0xA5u8;
    let mut out = vec![seed, seed ^ 2, seed ^ key];
    let (mut plain, mut enc1, mut enc2) = (key, seed ^ key, seed ^ 2);
    let ignored = (seed & 6) / 2;
    for byte in std::iter::repeat_n(0x37, ignored as usize)
        .chain((data.len() as u32).to_le_bytes())
        .chain(data.iter().copied())
    {
        let enc = byte ^ enc2.wrapping_add(plain);
        out.push(enc);
        enc2 = enc1;
        enc1 = enc;
        plain = byte;
    }
    out.iter().map(|b| format!("{b:02X}")).collect()
}
#[test]
fn decodes_password_semantics_and_can_recover() {
    for seed in [0, 2, 4, 6, 255] {
        for data in [b"\0".as_slice(), b"synthetic\0".as_slice()] {
            let d = tempfile::tempdir().unwrap();
            let p = d.path().join("synthetic.xlsm");
            let text = format!("DPB=\"{}\"\r\n", encoded(data, seed));
            archive(&p, Some(text.as_bytes()));
            let i = inspect(&p).unwrap();
            assert_eq!(
                i.status,
                if data == b"\0" {
                    Status::Unprotected
                } else {
                    Status::Protected
                }
            );
        }
    }
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("synthetic.xlsm");
    archive(&p, Some(b"DPB=\"1234\"\r\n"));
    assert!(inspect(&p).is_err());
}
#[test]
fn inspects_protected_project_without_modifying_source() {
    let d = tempfile::tempdir().unwrap();
    let p = d.path().join("synthetic.xlsm");
    let text = format!(
        "ID=\"SYNTHETIC\"\r\nDPB=\"{}\"\r\n",
        encoded(b"synthetic\0", 0)
    );
    archive(&p, Some(text.as_bytes()));
    let before = fs::read(&p).unwrap();
    let i = inspect(&p).unwrap();
    assert_eq!(i.status, Status::Protected);
    assert_eq!(fs::read(&p).unwrap(), before);
}
