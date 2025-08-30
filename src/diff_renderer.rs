use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct DiffLine {
    pub line_type: DiffLineType,
    pub content: String,
    pub line_number_old: Option<usize>,
    pub line_number_new: Option<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DiffLineType {
    Added,
    Removed,
    Context,
    Header,
}

#[derive(Debug)]
pub struct FileDiff {
    pub file_path: String,
    pub old_content: String,
    pub new_content: String,
    pub lines: Vec<DiffLine>,
    pub stats: DiffStats,
}

#[derive(Debug)]
pub struct DiffStats {
    pub additions: usize,
    pub deletions: usize,
    pub changes: usize,
}

pub struct DiffRenderer {
    show_line_numbers: bool,
    context_lines: usize,
    use_colors: bool,
}

impl DiffRenderer {
    pub fn new(show_line_numbers: bool, context_lines: usize, use_colors: bool) -> Self {
        Self {
            show_line_numbers,
            context_lines,
            use_colors,
        }
    }

    pub fn render_diff(&self, diff: &FileDiff) -> String {
        let mut output = String::new();

        // File header
        output.push_str(&self.render_header(&diff.file_path, &diff.stats));

        // Render each diff line
        for line in &diff.lines {
            output.push_str(&self.render_line(line));
            output.push('\n');
        }

        // Footer with summary
        output.push_str(&self.render_footer(&diff.stats));

        output
    }

    fn render_header(&self, file_path: &str, stats: &DiffStats) -> String {
        let mut header = format!(
            "\n{} File: {}\n",
            if self.use_colors {
                "\x1b[1;34mFILE\x1b[0m"
            } else {
                "FILE"
            },
            if self.use_colors {
                format!("\x1b[1;36m{}\x1b[0m", file_path)
            } else {
                file_path.to_string()
            }
        );

        header.push_str(&format!(
            "{} Changes: {} additions, {} deletions, {} modifications\n",
            if self.use_colors {
                "\x1b[1;35mSTATS\x1b[0m"
            } else {
                "STATS"
            },
            self.colorize(&stats.additions.to_string(), "\x1b[1;32m"),
            self.colorize(&stats.deletions.to_string(), "\x1b[1;31m"),
            self.colorize(&stats.changes.to_string(), "\x1b[1;33m")
        ));

        if self.show_line_numbers {
            header.push_str("┌─────┬─────────────────────────────────────────────────\n");
        } else {
            header.push_str("┌───────────────────────────────────────────────────────\n");
        }

        header
    }

    fn render_line(&self, line: &DiffLine) -> String {
        let prefix = match line.line_type {
            DiffLineType::Added => "+",
            DiffLineType::Removed => "-",
            DiffLineType::Context => " ",
            DiffLineType::Header => "@",
        };

        let color = match line.line_type {
            DiffLineType::Added => "\x1b[1;32m",   // Green
            DiffLineType::Removed => "\x1b[1;31m", // Red
            DiffLineType::Context => "\x1b[2;37m", // Dim white
            DiffLineType::Header => "\x1b[1;34m",  // Blue
        };

        let mut result = String::new();

        if self.show_line_numbers {
            let old_num = line
                .line_number_old
                .map_or("".to_string(), |n| format!("{:4}", n));
            let new_num = line
                .line_number_new
                .map_or("".to_string(), |n| format!("{:4}", n));
            result.push_str(&format!("│{}/{}│", old_num, new_num));
        }

        if self.use_colors {
            result.push_str(&format!("{}{}{}", color, prefix, line.content));
            result.push_str("\x1b[0m"); // Reset color
        } else {
            result.push_str(&format!("{}{}", prefix, line.content));
        }

        result
    }

    fn render_footer(&self, stats: &DiffStats) -> String {
        let mut footer = String::new();

        if self.show_line_numbers {
            footer.push_str("└─────┴─────────────────────────────────────────────────\n");
        } else {
            footer.push_str("└───────────────────────────────────────────────────────\n");
        }

        footer.push_str(&format!(
            "{} Summary: {} lines added, {} lines removed, {} lines changed\n\n",
            if self.use_colors {
                "\x1b[1;36mSUMMARY\x1b[0m"
            } else {
                "SUMMARY"
            },
            stats.additions,
            stats.deletions,
            stats.changes
        ));

        footer
    }

    fn colorize(&self, text: &str, color: &str) -> String {
        if self.use_colors {
            format!("{}{}{}", color, text, "\x1b[0m")
        } else {
            text.to_string()
        }
    }

    pub fn generate_diff(&self, old_content: &str, new_content: &str, file_path: &str) -> FileDiff {
        let old_lines: Vec<&str> = old_content.lines().collect();
        let new_lines: Vec<&str> = new_content.lines().collect();

        let mut lines = Vec::new();
        let mut additions = 0;
        let mut deletions = 0;
        let mut changes = 0;

        // Simple diff algorithm - can be enhanced with more sophisticated diffing
        let mut old_idx = 0;
        let mut new_idx = 0;

        while old_idx < old_lines.len() || new_idx < new_lines.len() {
            if old_idx < old_lines.len() && new_idx < new_lines.len() {
                if old_lines[old_idx] == new_lines[new_idx] {
                    // Same line - context
                    lines.push(DiffLine {
                        line_type: DiffLineType::Context,
                        content: old_lines[old_idx].to_string(),
                        line_number_old: Some(old_idx + 1),
                        line_number_new: Some(new_idx + 1),
                    });
                    old_idx += 1;
                    new_idx += 1;
                } else {
                    // Lines differ - find the difference
                    let (old_end, new_end) =
                        self.find_difference(&old_lines, &new_lines, old_idx, new_idx);

                    // Add removed lines
                    for i in old_idx..old_end {
                        lines.push(DiffLine {
                            line_type: DiffLineType::Removed,
                            content: old_lines[i].to_string(),
                            line_number_old: Some(i + 1),
                            line_number_new: None,
                        });
                        deletions += 1;
                    }

                    // Add added lines
                    for i in new_idx..new_end {
                        lines.push(DiffLine {
                            line_type: DiffLineType::Added,
                            content: new_lines[i].to_string(),
                            line_number_old: None,
                            line_number_new: Some(i + 1),
                        });
                        additions += 1;
                    }

                    old_idx = old_end;
                    new_idx = new_end;
                }
            } else if old_idx < old_lines.len() {
                // Remaining old lines are deletions
                lines.push(DiffLine {
                    line_type: DiffLineType::Removed,
                    content: old_lines[old_idx].to_string(),
                    line_number_old: Some(old_idx + 1),
                    line_number_new: None,
                });
                deletions += 1;
                old_idx += 1;
            } else if new_idx < new_lines.len() {
                // Remaining new lines are additions
                lines.push(DiffLine {
                    line_type: DiffLineType::Added,
                    content: new_lines[new_idx].to_string(),
                    line_number_old: None,
                    line_number_new: Some(new_idx + 1),
                });
                additions += 1;
                new_idx += 1;
            }
        }

        changes = additions + deletions;

        FileDiff {
            file_path: file_path.to_string(),
            old_content: old_content.to_string(),
            new_content: new_content.to_string(),
            lines,
            stats: DiffStats {
                additions,
                deletions,
                changes,
            },
        }
    }

    fn find_difference(
        &self,
        old_lines: &[&str],
        new_lines: &[&str],
        start_old: usize,
        start_new: usize,
    ) -> (usize, usize) {
        let mut old_end = start_old;
        let mut new_end = start_new;

        // Look for the next matching line
        while old_end < old_lines.len() && new_end < new_lines.len() {
            if old_lines[old_end] == new_lines[new_end] {
                return (old_end, new_end);
            }

            // Check if we can find a match within context window
            let mut found = false;
            for i in 1..=self.context_lines {
                if old_end + i < old_lines.len() && new_end + i < new_lines.len() {
                    if old_lines[old_end + i] == new_lines[new_end + i] {
                        old_end += i;
                        new_end += i;
                        found = true;
                        break;
                    }
                }
            }

            if !found {
                old_end += 1;
                new_end += 1;
            }
        }

        (old_end, new_end)
    }
}

pub struct DiffChatRenderer {
    diff_renderer: DiffRenderer,
}

impl DiffChatRenderer {
    pub fn new(show_line_numbers: bool, context_lines: usize, use_colors: bool) -> Self {
        Self {
            diff_renderer: DiffRenderer::new(show_line_numbers, context_lines, use_colors),
        }
    }

    pub fn render_file_change(
        &self,
        file_path: &Path,
        old_content: &str,
        new_content: &str,
    ) -> String {
        let diff = self.diff_renderer.generate_diff(
            old_content,
            new_content,
            &file_path.to_string_lossy(),
        );
        self.diff_renderer.render_diff(&diff)
    }

    pub fn render_multiple_changes(&self, changes: Vec<(String, String, String)>) -> String {
        let mut output = format!("\n🔄 Multiple File Changes ({} files)\n", changes.len());
        output.push_str("═".repeat(60).as_str());
        output.push_str("\n\n");

        for (file_path, old_content, new_content) in changes {
            let diff = self
                .diff_renderer
                .generate_diff(&old_content, &new_content, &file_path);
            output.push_str(&self.diff_renderer.render_diff(&diff));
        }

        output
    }

    pub fn render_operation_summary(
        &self,
        operation: &str,
        files_affected: usize,
        success: bool,
    ) -> String {
        let status = if success { "✅" } else { "❌" };
        let mut summary = format!("\n{} {}\n", status, operation);
        summary.push_str(&format!("📁 Files affected: {}\n", files_affected));

        if success {
            summary.push_str("🎉 Operation completed successfully!\n");
        } else {
            summary.push_str("⚠️  Operation completed with errors\n");
        }

        summary
    }
}

pub fn generate_unified_diff(old_content: &str, new_content: &str, filename: &str) -> String {
    let mut diff = format!("--- a/{}\n+++ b/{}\n", filename, filename);

    let old_lines: Vec<&str> = old_content.lines().collect();
    let new_lines: Vec<&str> = new_content.lines().collect();

    let mut old_idx = 0;
    let mut new_idx = 0;

    while old_idx < old_lines.len() || new_idx < new_lines.len() {
        // Find the next difference
        let start_old = old_idx;
        let start_new = new_idx;

        // Skip matching lines
        while old_idx < old_lines.len()
            && new_idx < new_lines.len()
            && old_lines[old_idx] == new_lines[new_idx]
        {
            old_idx += 1;
            new_idx += 1;
        }

        if old_idx == old_lines.len() && new_idx == new_lines.len() {
            break; // No more differences
        }

        // Find the end of the difference
        let mut end_old = old_idx;
        let mut end_new = new_idx;

        // Look for next matching context
        let mut context_found = false;
        for i in 0..3 {
            // Look ahead 3 lines for context
            if end_old + i < old_lines.len() && end_new + i < new_lines.len() {
                if old_lines[end_old + i] == new_lines[end_new + i] {
                    end_old += i;
                    end_new += i;
                    context_found = true;
                    break;
                }
            }
        }

        if !context_found {
            end_old = old_lines.len();
            end_new = new_lines.len();
        }

        // Generate hunk
        let old_count = end_old - start_old;
        let new_count = end_new - start_new;

        diff.push_str(&format!(
            "@@ -{},{} +{},{} @@\n",
            start_old + 1,
            old_count,
            start_new + 1,
            new_count
        ));

        // Add context before
        for i in (start_old.saturating_sub(3))..start_old {
            if i < old_lines.len() {
                diff.push_str(&format!(" {}\n", old_lines[i]));
            }
        }

        // Add removed lines
        for i in start_old..end_old {
            if i < old_lines.len() {
                diff.push_str(&format!("-{}\n", old_lines[i]));
            }
        }

        // Add added lines
        for i in start_new..end_new {
            if i < new_lines.len() {
                diff.push_str(&format!("+{}\n", new_lines[i]));
            }
        }

        // Add context after
        for i in end_old..(end_old + 3) {
            if i < old_lines.len() {
                diff.push_str(&format!(" {}\n", old_lines[i]));
            }
        }

        old_idx = end_old;
        new_idx = end_new;
    }

    diff
}
