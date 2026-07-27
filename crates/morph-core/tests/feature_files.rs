use morph::format::Format;
use std::path::Path;

const FEATURE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/features");

#[derive(Debug)]
struct Scenario {
    name: String,
    given: String,
    expected: String,
}

fn parse_feature_file(path: &Path) -> Vec<Scenario> {
    let content = std::fs::read_to_string(path).expect("read feature file");
    let mut scenarios = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();

        // Skip commented lines
        if line.starts_with('#') {
            i += 1;
            continue;
        }

        if line.starts_with("Scenario:") {
            let name = line.strip_prefix("Scenario:").unwrap().trim().to_string();
            i += 1;

            let mut given = None;
            let mut expected = None;

            while i < lines.len() {
                let l = lines[i].trim();

                if l.starts_with("Scenario:") {
                    break;
                }

                if l.starts_with("Given the Markdown source") {
                    i += 1;
                    given = Some(extract_docstring(&lines, &mut i));
                } else if l.starts_with("Then the result should match the AsciiDoc source") {
                    i += 1;
                    expected = Some(extract_docstring(&lines, &mut i));
                } else {
                    i += 1;
                }
            }

            if let (Some(g), Some(e)) = (given, expected) {
                scenarios.push(Scenario {
                    name,
                    given: g,
                    expected: e,
                });
            }
        } else {
            i += 1;
        }
    }

    scenarios
}

fn extract_docstring(lines: &[&str], i: &mut usize) -> String {
    // Find opening """ and record its indentation
    let mut base_indent = 0;
    while *i < lines.len() {
        let line = lines[*i];
        if line.trim() == "\"\"\"" {
            base_indent = line.len() - line.trim_start().len();
            *i += 1;
            break;
        }
        *i += 1;
    }

    // Collect content lines until closing """
    let mut content_lines = Vec::new();
    while *i < lines.len() {
        if lines[*i].trim() == "\"\"\"" {
            *i += 1;
            break;
        }
        content_lines.push(lines[*i]);
        *i += 1;
    }

    // Strip the base indent (from the """ line) from content lines
    let result: Vec<String> = content_lines
        .iter()
        .map(|l| {
            if l.len() >= base_indent && l[..base_indent].trim().is_empty() {
                l[base_indent..].to_string()
            } else if l.trim().is_empty() {
                String::new()
            } else {
                l.to_string()
            }
        })
        .collect();

    // Apply {sp} substitution (space character placeholder)
    result.join("\n").replace("{sp}", " ")
}

fn run_feature_file(filename: &str) {
    let path = Path::new(FEATURE_DIR).join(filename);
    let scenarios = parse_feature_file(&path);

    let mut pass_count = 0;
    let mut fail_count = 0;
    let mut failures = Vec::new();

    for scenario in &scenarios {
        let actual = morph::convert(&scenario.given, Format::Markdown, Format::AsciiDoc)
            .expect("conversion should not error");

        // The feature file expected output doesn't include trailing newline,
        // but morph always outputs trailing newline. Also, some legacy scenarios
        // encode trailing spaces at line ends (a quirk of the original tool that
        // morph deliberately avoids). Normalize both.
        fn normalize(s: &str) -> String {
            s.lines()
                .map(|line| line.trim_end())
                .collect::<Vec<_>>()
                .join("\n")
                .trim_end_matches('\n')
                .to_string()
        }
        let actual_trimmed = normalize(&actual);
        let expected_trimmed = normalize(&scenario.expected);

        if actual_trimmed == expected_trimmed {
            pass_count += 1;
        } else {
            fail_count += 1;
            failures.push((
                scenario.name.clone(),
                expected_trimmed.to_string(),
                actual_trimmed.to_string(),
            ));
        }
    }

    if !failures.is_empty() {
        let mut msg = format!(
            "\n{filename}: {pass_count} passed, {fail_count} failed out of {} scenarios\n",
            pass_count + fail_count
        );
        for (name, expected, actual) in &failures {
            msg.push_str(&format!("\n  FAIL: {name}\n"));
            msg.push_str("    Expected:\n");
            for line in expected.lines() {
                msg.push_str(&format!("      |{line}|\n"));
            }
            msg.push_str("    Actual:\n");
            for line in actual.lines() {
                msg.push_str(&format!("      |{line}|\n"));
            }
        }
        panic!("{msg}");
    }
}

#[test]
fn feature_headings() {
    run_feature_file("headings.feature");
}

#[test]
fn feature_paragraphs() {
    run_feature_file("paragraphs.feature");
}

#[test]
fn feature_code() {
    run_feature_file("code.feature");
}

#[test]
fn feature_markup() {
    run_feature_file("markup.feature");
}

#[test]
fn feature_lines() {
    run_feature_file("lines.feature");
}

#[test]
fn feature_links() {
    run_feature_file("links.feature");
}

#[test]
fn feature_lists() {
    run_feature_file("lists.feature");
}

#[test]
fn feature_tables() {
    run_feature_file("tables.feature");
}

#[test]
fn feature_descriptionlists() {
    run_feature_file("descriptionlists.feature");
}
