//! Bounded replacement of the generated table inside its protocol section.

use crate::error::{Error, Result};

const SECTION: &str = "\n## Control matrix\n";
const BEGIN: &str = "<!-- capability-matrix:start -->";
const END: &str = "<!-- capability-matrix:end -->";

pub(super) fn replace_matrix(document: &str, matrix: &str) -> Result<String> {
    let Some(section_start) = document.find(SECTION).map(|index| index + SECTION.len()) else {
        return Err(Error::MatrixSectionMissing);
    };
    let section_end = document[section_start..]
        .find("\n## ")
        .map_or(document.len(), |index| section_start + index);
    let section = &document[section_start..section_end];
    let generated = format!("\n\n{}\n\n", matrix.trim_end());
    let mut output = String::new();
    match (section.find(BEGIN), section.find(END)) {
        (Some(begin), Some(end))
            if begin < end
                && section.matches(BEGIN).count() == 1
                && section.matches(END).count() == 1 =>
        {
            output.push_str(&document[..section_start + begin + BEGIN.len()]);
            output.push_str(&generated);
            output.push_str(&document[section_start + end..]);
        }
        (None, None) => {
            output.push_str(&document[..section_end]);
            if !output.ends_with('\n') {
                output.push('\n');
            }
            if !output.ends_with("\n\n") {
                output.push('\n');
            }
            output.push_str(BEGIN);
            output.push_str(&generated);
            output.push_str(END);
            output.push('\n');
            output.push_str(&document[section_end..]);
        }
        _ => return Err(Error::MatrixMarkersInvalid),
    }
    Ok(output)
}
