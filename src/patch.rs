//! The reverse patches for a discard of 1 hunk or 1 line of the working
//! tree. `git apply -R` needs the full file header even for a single hunk.

use crate::{DiffHunk, DiffLine};

fn patch_header(file: &str) -> [String; 3] {
    [
        format!("diff --git a/{file} b/{file}"),
        format!("--- a/{file}"),
        format!("+++ b/{file}"),
    ]
}

fn marker(kind: &str) -> char {
    match kind {
        "add" => '+',
        "rem" => '-',
        _ => ' ',
    }
}

/// The patch of a whole hunk.
pub fn hunk_patch(file: &str, hunk: &DiffHunk) -> String {
    let header = format!(
        "@@ -{},{} +{},{} @@",
        hunk.old_start, hunk.old_lines, hunk.new_start, hunk.new_lines
    );
    let mut out: Vec<String> = patch_header(file).to_vec();
    out.push(header);
    out.extend(hunk.lines.iter().map(|l| format!("{}{}", marker(&l.kind), l.text)));
    out.push(String::new());
    out.join("\n")
}

/// The patch of 1 line. Every other add line turns into context, the
/// working tree has it, and every other rem line is left out, the working
/// tree does not have it. Only an add or a rem line can be the target.
pub fn line_patch(file: &str, hunk: &DiffHunk, target: usize) -> Option<String> {
    let target_line: &DiffLine = hunk.lines.get(target)?;
    if target_line.kind != "add" && target_line.kind != "rem" {
        return None;
    }
    let mut body = Vec::new();
    let mut old_count = 0;
    let mut new_count = 0;
    for (i, line) in hunk.lines.iter().enumerate() {
        if i == target {
            if line.kind == "add" {
                body.push(format!("+{}", line.text));
                new_count += 1;
            } else {
                body.push(format!("-{}", line.text));
                old_count += 1;
            }
        } else if line.kind != "rem" {
            body.push(format!(" {}", line.text));
            old_count += 1;
            new_count += 1;
        }
    }
    let header = format!(
        "@@ -{},{old_count} +{},{new_count} @@",
        hunk.old_start, hunk.new_start
    );
    let mut out: Vec<String> = patch_header(file).to_vec();
    out.push(header);
    out.extend(body);
    out.push(String::new());
    Some(out.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::{hunk_patch, line_patch};
    use crate::{DiffHunk, DiffLine};

    const HEADER: &str = "diff --git a/src/a.rs b/src/a.rs\n--- a/src/a.rs\n+++ b/src/a.rs\n";

    fn line(kind: &str, text: &str) -> DiffLine {
        DiffLine {
            kind:   kind.to_string(),
            text:   text.to_string(),
            old_no: None,
            new_no: None,
        }
    }

    fn hunk() -> DiffHunk {
        DiffHunk {
            old_start: 10,
            old_lines: 3,
            new_start: 10,
            new_lines: 4,
            lines:     vec![
                line("ctx", "one"),
                line("rem", "two"),
                line("add", "two changed"),
                line("add", "extra"),
                line("ctx", "three"),
            ],
        }
    }

    #[test]
    fn hunk_patch_replays_the_whole_hunk() {
        let patch = hunk_patch("src/a.rs", &hunk());
        let body = "@@ -10,3 +10,4 @@\n one\n-two\n+two changed\n+extra\n three\n";
        assert_eq!(patch, format!("{HEADER}{body}"));
    }

    #[test]
    fn line_patch_keeps_other_adds_as_context() {
        let patch = line_patch("src/a.rs", &hunk(), 3).expect("add line is discardable");
        // The other add stays as context and the rem line is left out, so
        // the counts are 3 context lines plus the one discarded add.
        let body = "@@ -10,3 +10,4 @@\n one\n two changed\n+extra\n three\n";
        assert_eq!(patch, format!("{HEADER}{body}"));
    }

    #[test]
    fn line_patch_restores_a_removed_line() {
        let patch = line_patch("src/a.rs", &hunk(), 1).expect("rem line is discardable");
        let body = "@@ -10,5 +10,4 @@\n one\n-two\n two changed\n extra\n three\n";
        assert_eq!(patch, format!("{HEADER}{body}"));
    }

    #[test]
    fn context_lines_are_not_discardable() {
        assert!(line_patch("src/a.rs", &hunk(), 0).is_none());
        assert!(line_patch("src/a.rs", &hunk(), 99).is_none());
    }
}
