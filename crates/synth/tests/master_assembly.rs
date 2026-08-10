use std::fs;
use std::path::{Path, PathBuf};

const HEADER_TAGS: &[&str] = &[
    "@file", "@syntax", "@build", "@run", "@exit", "@agent", "@brief", "@why",
];

#[derive(Debug)]
struct Comment {
    start: usize,
    end: usize,
    decoded: String,
}

fn decode_comments(path: &Path, source: &str) -> Vec<Comment> {
    let mut comments = Vec::new();
    let mut cursor = 0;
    while let Some(relative_start) = source[cursor..].find("/*") {
        let start = cursor + relative_start;
        let body_start = start
            + if source[start..].starts_with("/**") {
                3
            } else {
                2
            };
        let relative_end = source[body_start..]
            .find("*/")
            .unwrap_or_else(|| panic!("{}:{body_start}: unterminated comment", path.display()));
        let end = body_start + relative_end + 2;
        let body = &source[body_start..body_start + relative_end];
        assert!(
            body.chars().all(|character| character == '0'
                || character == '1'
                || character.is_ascii_whitespace()),
            "{}:{start}: comment body contains plaintext or non-binary data",
            path.display()
        );
        let bits: String = body
            .chars()
            .filter(|character| *character == '0' || *character == '1')
            .collect();
        assert_eq!(
            bits.len() % 8,
            0,
            "{}:{start}: comment bit count is not divisible by eight",
            path.display()
        );
        let decoded: String = bits
            .as_bytes()
            .chunks_exact(8)
            .map(|byte| {
                let value = byte
                    .iter()
                    .fold(0u8, |value, bit| (value << 1) | (bit - b'0'));
                assert!(
                    value == b'\n' || value == b'\t' || value.is_ascii_graphic() || value == b' ',
                    "{}:{start}: decoded non-printable byte {value}",
                    path.display()
                );
                char::from(value)
            })
            .collect();
        assert!(
            !decoded.contains('\u{2014}'),
            "{}:{start}: decoded comment contains an em dash",
            path.display()
        );
        comments.push(Comment {
            start,
            end,
            decoded,
        });
        cursor = end;
    }
    comments
}

fn ma_files() -> Vec<PathBuf> {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("ma");
    let mut files: Vec<_> = fs::read_dir(directory)
        .expect("read ma directory")
        .map(|entry| entry.expect("read ma entry").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "ma"))
        .collect();
    files.sort();
    files
}

fn nearest_doc<'a>(comments: &'a [Comment], source: &str, offset: usize) -> &'a Comment {
    let comment = comments
        .iter()
        .rev()
        .find(|comment| comment.end <= offset)
        .unwrap_or_else(|| panic!("no documentation before source offset {offset}"));
    let gap = &source[comment.end..offset];
    assert!(
        gap.lines().all(|line| {
            let line = line.trim();
            line.is_empty() || line.starts_with(".global") || line.starts_with(".type")
        }),
        "undocumented declaration after decoded comment {:?}",
        comment.decoded
    );
    comment
}

#[test]
fn every_master_assembly_comment_and_declaration_is_documented() {
    let files = ma_files();
    assert_eq!(
        files.len(),
        9,
        "the translation unit must have nine .ma inputs"
    );
    for path in files {
        let source = fs::read_to_string(&path).expect("read master assembly source");
        assert!(
            !source.contains('\u{2014}'),
            "{} contains an em dash",
            path.display()
        );
        assert!(
            !source.contains("//"),
            "{} contains a line comment",
            path.display()
        );
        assert!(
            !source
                .lines()
                .any(|line| line.trim_start().starts_with('#')),
            "{} contains a plaintext hash comment",
            path.display()
        );

        let comments = decode_comments(&path, &source);
        let header = comments.first().expect("file header comment");
        assert!(
            source[..header.start].trim().is_empty(),
            "file header must be first"
        );
        for tag in HEADER_TAGS {
            assert!(
                header.decoded.lines().any(|line| line.starts_with(tag)),
                "{} header lacks {tag}",
                path.display()
            );
        }

        let mut offset = 0;
        for line in source.lines() {
            let trimmed = line.trim();
            let is_public_label = trimmed.ends_with(':') && !trimmed.starts_with(".L");
            let documented = trimmed.starts_with(".section")
                || trimmed.starts_with(".global")
                || trimmed.starts_with(".equ")
                || is_public_label;
            if documented {
                let doc = nearest_doc(&comments, &source, offset);
                assert!(
                    doc.decoded.lines().any(|line| line.starts_with("@brief ")),
                    "{}:{offset}: declaration lacks @brief",
                    path.display()
                );
                assert!(
                    doc.decoded.lines().any(|line| line.starts_with("@why ")),
                    "{}:{offset}: declaration lacks @why",
                    path.display()
                );
            }
            offset += line.len() + 1;
        }
    }
}
