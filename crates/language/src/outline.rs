use crate::{BufferSnapshot, Point, ToPoint, ToTreeSitterPoint};
use fuzzy::{StringMatch, StringMatchCandidate};
use gpui::{BackgroundExecutor, HighlightStyle};
use std::ops::Range;

pub const MAX_OUTLINE_ITEMS: usize = 20_000;
const MAX_OUTLINE_PATH_TEXT_BYTES: usize = 256 * 1024;
const MAX_OUTLINE_SYMBOL_TEXT_BYTES: usize = 4 * 1024;
const MAX_OUTLINE_NAME_RANGES: usize = 256;
pub const MAX_OUTLINE_SEARCH_MATCHES: usize = 100;
const MAX_OUTLINE_TREE_MATCHES: usize = 2_000;

fn warn_truncated_outline_materialization(label: &str, actual: usize, max: usize) {
    if actual > max {
        log::warn!("truncating {label} from {actual} to {max} entries");
    }
}

fn bounded_utf8_prefix(text: &str, max_bytes: usize) -> &str {
    if text.len() <= max_bytes {
        return text;
    }

    let mut end = max_bytes;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

fn push_bounded_outline_text(target: &mut String, text: &str, max_bytes: usize) {
    if target.len() >= max_bytes {
        return;
    }

    let remaining = max_bytes - target.len();
    target.push_str(bounded_utf8_prefix(text, remaining));
}

/// An outline of all the symbols contained in a buffer.
#[derive(Debug)]
pub struct Outline<T> {
    pub items: Vec<OutlineItem<T>>,
    candidates: Vec<StringMatchCandidate>,
    pub path_candidates: Vec<StringMatchCandidate>,
    path_candidate_prefixes: Vec<usize>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct OutlineItem<T> {
    pub depth: usize,
    pub range: Range<T>,
    pub source_range_for_text: Range<T>,
    pub text: String,
    pub highlight_ranges: Vec<(Range<usize>, HighlightStyle)>,
    pub name_ranges: Vec<Range<usize>>,
    pub body_range: Option<Range<T>>,
    pub annotation_range: Option<Range<T>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SymbolPath(pub String);

impl<T: ToPoint> OutlineItem<T> {
    /// Converts to an equivalent outline item, but with parameterized over Points.
    pub fn to_point(&self, buffer: &BufferSnapshot) -> OutlineItem<Point> {
        OutlineItem {
            depth: self.depth,
            range: self.range.start.to_point(buffer)..self.range.end.to_point(buffer),
            source_range_for_text: self.source_range_for_text.start.to_point(buffer)
                ..self.source_range_for_text.end.to_point(buffer),
            text: self.text.clone(),
            highlight_ranges: self.highlight_ranges.clone(),
            name_ranges: self.name_ranges.clone(),
            body_range: self
                .body_range
                .as_ref()
                .map(|r| r.start.to_point(buffer)..r.end.to_point(buffer)),
            annotation_range: self
                .annotation_range
                .as_ref()
                .map(|r| r.start.to_point(buffer)..r.end.to_point(buffer)),
        }
    }

    pub fn body_range(&self, buffer: &BufferSnapshot) -> Option<Range<Point>> {
        if let Some(range) = self.body_range.as_ref() {
            return Some(range.start.to_point(buffer)..range.end.to_point(buffer));
        }

        let range = self.range.start.to_point(buffer)..self.range.end.to_point(buffer);
        let start_indent = buffer.indent_size_for_line(range.start.row);
        let node = buffer.syntax_ancestor(range.clone())?;

        let mut cursor = node.walk();
        loop {
            let node = cursor.node();
            if node.start_position() >= range.start.to_ts_point()
                && node.end_position() <= range.end.to_ts_point()
            {
                break;
            }
            cursor.goto_first_child_for_point(range.start.to_ts_point());
        }

        if !cursor.goto_last_child() {
            return None;
        }
        let body_node = loop {
            let node = cursor.node();
            if node.child_count() > 0 {
                break node;
            }
            if !cursor.goto_previous_sibling() {
                return None;
            }
        };

        let mut start_row = body_node.start_position().row as u32;
        let mut end_row = body_node.end_position().row as u32;

        while start_row < end_row && buffer.indent_size_for_line(start_row) == start_indent {
            start_row += 1;
        }
        while start_row < end_row && buffer.indent_size_for_line(end_row - 1) == start_indent {
            end_row -= 1;
        }
        if start_row < end_row {
            return Some(Point::new(start_row, 0)..Point::new(end_row, 0));
        }
        None
    }
}

impl<T> Outline<T> {
    pub fn new(mut items: Vec<OutlineItem<T>>) -> Self {
        warn_truncated_outline_materialization("outline items", items.len(), MAX_OUTLINE_ITEMS);
        items.truncate(MAX_OUTLINE_ITEMS);

        let mut candidates = Vec::with_capacity(items.len());
        let mut path_candidates = Vec::with_capacity(items.len());
        let mut path_candidate_prefixes = Vec::with_capacity(items.len());
        let mut path_text = String::new();
        let mut path_stack = Vec::new();

        for (id, item) in items.iter().enumerate() {
            if item.depth < path_stack.len() {
                path_stack.truncate(item.depth);
                path_text.truncate(path_stack.last().copied().unwrap_or(0));
            }
            if !path_text.is_empty() && path_text.len() < MAX_OUTLINE_PATH_TEXT_BYTES {
                path_text.push(' ');
            }
            path_candidate_prefixes.push(path_text.len());
            push_bounded_outline_text(&mut path_text, &item.text, MAX_OUTLINE_PATH_TEXT_BYTES);
            path_stack.push(path_text.len());

            let mut candidate_text = String::new();
            for range in item.name_ranges.iter().take(MAX_OUTLINE_NAME_RANGES) {
                if let Some(text) = item.text.get(range.clone()) {
                    push_bounded_outline_text(
                        &mut candidate_text,
                        text,
                        MAX_OUTLINE_SYMBOL_TEXT_BYTES,
                    );
                }
                if candidate_text.len() >= MAX_OUTLINE_SYMBOL_TEXT_BYTES {
                    break;
                }
            }

            path_candidates.push(StringMatchCandidate::new(id, &path_text));
            candidates.push(StringMatchCandidate::new(id, &candidate_text));
        }

        Self {
            candidates,
            path_candidates,
            path_candidate_prefixes,
            items,
        }
    }

    /// Find the most similar symbol to the provided query using normalized Levenshtein distance.
    pub fn find_most_similar(&self, query: &str) -> Option<(SymbolPath, &OutlineItem<T>)> {
        const SIMILARITY_THRESHOLD: f64 = 0.6;

        let (position, similarity) = self
            .path_candidates
            .iter()
            .enumerate()
            .map(|(index, candidate)| {
                let similarity = strsim::normalized_levenshtein(&candidate.string, query);
                (index, similarity)
            })
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())?;

        if similarity >= SIMILARITY_THRESHOLD {
            self.path_candidates
                .get(position)
                .map(|candidate| SymbolPath(candidate.string.clone()))
                .zip(self.items.get(position))
        } else {
            None
        }
    }

    /// Find all outline symbols according to a longest subsequence match with the query, ordered descending by match score.
    pub async fn search(&self, query: &str, executor: BackgroundExecutor) -> Vec<StringMatch> {
        let query = query.trim_start();
        let is_path_query = query.contains(' ');
        let smart_case = query.chars().any(|c| c.is_uppercase());
        let mut matches = fuzzy::match_strings(
            if is_path_query {
                &self.path_candidates
            } else {
                &self.candidates
            },
            query,
            smart_case,
            true,
            MAX_OUTLINE_SEARCH_MATCHES,
            &Default::default(),
            executor.clone(),
        )
        .await;
        matches.sort_unstable_by_key(|m| m.candidate_id);

        let mut tree_matches = Vec::new();

        let mut prev_item_ix = 0;
        for mut string_match in matches {
            if tree_matches.len() >= MAX_OUTLINE_TREE_MATCHES {
                break;
            }

            let outline_match = &self.items[string_match.candidate_id];
            string_match.string.clone_from(&outline_match.text);

            if is_path_query {
                let prefix_len = self.path_candidate_prefixes[string_match.candidate_id];
                string_match
                    .positions
                    .retain(|position| *position >= prefix_len);
                for position in &mut string_match.positions {
                    *position -= prefix_len;
                }
            } else {
                let mut name_ranges = outline_match.name_ranges.iter();
                let Some(mut name_range) = name_ranges.next() else {
                    continue;
                };
                let mut preceding_ranges_len = 0;
                for position in &mut string_match.positions {
                    while *position >= preceding_ranges_len + name_range.len() {
                        preceding_ranges_len += name_range.len();
                        name_range = name_ranges.next().unwrap();
                    }
                    *position = name_range.start + (*position - preceding_ranges_len);
                }
            }

            let insertion_ix = tree_matches.len();
            let mut cur_depth = outline_match.depth;
            for (ix, item) in self.items[prev_item_ix..string_match.candidate_id]
                .iter()
                .enumerate()
                .rev()
            {
                if cur_depth == 0 || tree_matches.len() >= MAX_OUTLINE_TREE_MATCHES {
                    break;
                }

                let candidate_index = ix + prev_item_ix;
                if item.depth == cur_depth - 1 {
                    if tree_matches.len() >= MAX_OUTLINE_TREE_MATCHES {
                        break;
                    }
                    tree_matches.insert(
                        insertion_ix,
                        StringMatch {
                            candidate_id: candidate_index,
                            score: Default::default(),
                            positions: Default::default(),
                            string: Default::default(),
                        },
                    );
                    cur_depth -= 1;
                }
            }

            prev_item_ix = string_match.candidate_id + 1;
            if tree_matches.len() >= MAX_OUTLINE_TREE_MATCHES {
                break;
            }
            tree_matches.push(string_match);
        }

        tree_matches
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;

    #[gpui::test]
    async fn test_entries_with_no_names(cx: &mut TestAppContext) {
        let outline = Outline::new(vec![
            OutlineItem {
                depth: 0,
                range: Point::new(0, 0)..Point::new(5, 0),
                source_range_for_text: Point::new(0, 0)..Point::new(0, 9),
                text: "class Foo".to_string(),
                highlight_ranges: vec![],
                name_ranges: vec![6..9],
                body_range: None,
                annotation_range: None,
            },
            OutlineItem {
                depth: 0,
                range: Point::new(2, 0)..Point::new(2, 7),
                source_range_for_text: Point::new(0, 0)..Point::new(0, 7),
                text: "private".to_string(),
                highlight_ranges: vec![],
                name_ranges: vec![],
                body_range: None,
                annotation_range: None,
            },
        ]);
        assert_eq!(
            outline
                .search(" ", cx.executor())
                .await
                .into_iter()
                .map(|mat| mat.string)
                .collect::<Vec<String>>(),
            vec!["class Foo".to_string()]
        );
    }

    #[test]
    fn test_find_most_similar_with_low_similarity() {
        let outline = Outline::new(vec![
            OutlineItem {
                depth: 0,
                range: Point::new(0, 0)..Point::new(5, 0),
                source_range_for_text: Point::new(0, 0)..Point::new(0, 10),
                text: "fn process".to_string(),
                highlight_ranges: vec![],
                name_ranges: vec![3..10],
                body_range: None,
                annotation_range: None,
            },
            OutlineItem {
                depth: 0,
                range: Point::new(7, 0)..Point::new(12, 0),
                source_range_for_text: Point::new(0, 0)..Point::new(0, 20),
                text: "struct DataProcessor".to_string(),
                highlight_ranges: vec![],
                name_ranges: vec![7..20],
                body_range: None,
                annotation_range: None,
            },
        ]);
        assert_eq!(
            outline.find_most_similar("pub fn process"),
            Some((SymbolPath("fn process".into()), &outline.items[0]))
        );
        assert_eq!(
            outline.find_most_similar("async fn process"),
            Some((SymbolPath("fn process".into()), &outline.items[0])),
        );
        assert_eq!(
            outline.find_most_similar("struct Processor"),
            Some((SymbolPath("struct DataProcessor".into()), &outline.items[1]))
        );
        assert_eq!(outline.find_most_similar("struct User"), None);
        assert_eq!(outline.find_most_similar("struct"), None);
    }
}
