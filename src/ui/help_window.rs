/// Functions for rendering the help window
use crossterm::style::{StyledContent, Stylize};
use textwrap::{self, word_splitters::NoHyphenation, Options};

const README_STR: &str = include_str!("../../README.md");

/// Word-wrap the help string to be displayed in the help window, and apply correct formatting
/// (such as bolding) using crossterm::style.
///
/// Returns a vector of vectors, where the outer vector represents lines, and the inner vector
/// contains either a single string for the whole line, or multiple strings, if the style varies
/// within the line.
pub fn get_formatted_help_text(width: usize) -> Vec<Vec<StyledContent<String>>> {
    let help_str = &README_STR[
        README_STR.find("## User guide").expect("Could not find user guide in README")
        ..
        README_STR.find("## Similar projects").expect("Could not find end of user guide in README")
    ];

    // Skip the table of keyboard shortcuts, we'll format it separately
    let (help_str, rest) = help_str
        .split_once("\n\n|")
        .expect("Could not find keyboard shortcuts table in readme");

    let rest = rest
        .split_once("\n\n")
        .expect("Could not find end of keyboard shortcuts table in readme")
        .1;

    // Add justified keyboard shortcuts table to help string
    let mut help_str = help_str.to_string();
    help_str.push_str("\n\n"); // add back newlines eaten by split_once
    help_str.push_str(&get_justified_keyboard_shortcuts_table());
    help_str.push_str(rest);

    // We need to get rid of the `<kbd>` tags before wrapping so it works correctly. We're going to
    // bold all words within backticks, so replace the tags with backticks as well.
    let help_str = help_str
        .replace("<kbd>",  "`")
        .replace("</kbd>", "`");

    // Strip out markup and extract the locations where we need to toggle bold on/off.
    let (help_str, bold_toggle_locs) = strip_markup_and_extract_bold_positions(&help_str);

    // apply text wrapping
    let opts = Options::with_word_splitter(width, NoHyphenation);
    let help_str = textwrap::wrap(&help_str, opts);

    // apply bold at the toggle locations and return
    stylize_wrapped_lines(help_str, bold_toggle_locs)
}

/// Apply justification to the table of keyboard shortcuts in the README and render it to a String
/// without the markup
pub fn get_justified_keyboard_shortcuts_table() -> String {
    let keyboard_shortcuts = README_STR
        .split_once("keyboard shortcuts:\n\n")
        .expect("Couldn't find table of keyboard shortcuts in README")
        .1;
    let keyboard_shortcuts = keyboard_shortcuts
        .split_once("\n\n")
        .expect("Couldn't find end of keyboard shortcuts table in README")
        .0;

    let first_column_width = keyboard_shortcuts
        .lines()
        .map(|line| line.split('|').nth(1).unwrap_or("").len())
        .max()
        .unwrap_or(10);

    let mut justified = String::new();

    for (i, line) in keyboard_shortcuts.lines().enumerate() {
        let cols: Vec<&str> = line.split('|').collect();
        // cols[0] is empty, because the lines start with '|'.
        let mut action = cols[1].trim().to_string();
        let mut shortcut = cols[2].trim().to_string();

        // skip markdown table formatting row
        if action.starts_with(":--") {
            continue;
        }

        if i == 0 {
            // add backticks so that first line is bolded
            action = format!("`{}`", &action);
            shortcut = format!("`{}`", &shortcut);
        }

        justified.push_str(&action);

        // backticks will be removed, so add extra space for them
        let extra_len = action.chars().filter(|c| *c == '`').count();
        let padding = first_column_width + extra_len + 2 - action.len();
        justified.push_str(&" ".repeat(padding));
        // It's ok to add "\n" at the end of every line, because the split_once() above has
        // eaten too many newlines from the end anyway.
        justified.push_str(&shortcut);
        justified.push('\n');
    }

    // add extra newline at end
    justified.push('\n');

    justified
}

/// Return a version of `text`, where all markup has been strippeed, and also return a vector of
/// indices into the returned string where bold should toggle.
fn strip_markup_and_extract_bold_positions(text: &str) -> (String, Vec<usize>) {
    let mut bold_toggle_locs: Vec<usize> = vec![];
    let mut help_string_no_markup = String::new();
    let mut prev_char: Option<char> = None;
    let mut parsing_heading = false;
    let mut counter = 0;
    for c in text.chars() {
        if c == '#' {
            if !parsing_heading {
                parsing_heading = true;
                bold_toggle_locs.push(counter);
            }
        } else if c == ' ' && parsing_heading && prev_char == Some('#') {
            // skip space after hashes that indicate heading
        } else if c == '\n' && parsing_heading {
            bold_toggle_locs.push(counter);
            parsing_heading = false;
            counter += 1;
            help_string_no_markup.push(c);
        } else if c == '`' {
            bold_toggle_locs.push(counter);
        } else {
            counter += 1;
            help_string_no_markup.push(c);
        }
        prev_char = Some(c);
    }

    (help_string_no_markup, bold_toggle_locs)
}

/// Apply stylization to the text. Toggle bold at the positions indicated by `bold_toggle_locs`.
fn stylize_wrapped_lines<S>(
    lines: Vec<S>,
    bold_toggle_locs: Vec<usize>,
) -> Vec<Vec<StyledContent<String>>>
where
    S: AsRef<str>,
{
    let mut counter = 0;
    let mut bold_toggle_locs = bold_toggle_locs.iter();
    let mut next_toggle_loc = bold_toggle_locs.next();
    let mut res = vec![];
    let mut bold = false;

    for line in lines {
        let mut line_chunks = vec![];
        let mut cur_chunk = String::new();

        for c in line.as_ref().chars() {
            if Some(&counter) == next_toggle_loc {
                line_chunks.push(if bold {
                    cur_chunk.bold()
                } else {
                    cur_chunk.stylize()
                });
                bold = !bold;
                next_toggle_loc = bold_toggle_locs.next();
                cur_chunk = String::new();
            }
            cur_chunk.push(c);
            counter += 1;
        }

        if !cur_chunk.is_empty() {
            line_chunks.push(if bold {
                cur_chunk.bold()
            } else {
                cur_chunk.stylize()
            });
        }

        // always turn off bold at the end of the line
        if bold {
            bold = false;
            next_toggle_loc = bold_toggle_locs.next();
        }

        res.push(line_chunks);

        // increment counter for newline
        counter += 1;
    }

    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_guide_found() {
        // this should panic if the README is incorrectly formatted
        get_formatted_help_text(100);
    }

    #[test]
    fn test_strip_markup() {
        let input = "## foo bar\n\nlorem ipsum `dolor` sit amet";
        let (output, locs) = strip_markup_and_extract_bold_positions(input);
        assert_eq!(output, "foo bar\n\nlorem ipsum dolor sit amet");
        assert_eq!(locs, vec![0, 7, 21, 26]);
    }

    #[test]
    fn test_stylize_wrapped_lines() {
        let lines = vec!["foo bar", "", "lorem ipsum dolor sit amet"];
        let stylized = stylize_wrapped_lines(lines, vec![0, 7, 21, 26]);

        assert_eq!(
            stylized[0],
            vec!["".to_string().stylize(), "foo bar".to_string().bold()]
        );
        assert_eq!(stylized[1], vec![]);
        assert_eq!(stylized[2][0], "lorem ipsum ".to_string().stylize());
        assert_eq!(stylized[2][1], "dolor".to_string().bold());
        assert_eq!(stylized[2][2], " sit amet".to_string().stylize());
    }
}
