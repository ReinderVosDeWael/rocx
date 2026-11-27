use std::collections::VecDeque;

use rocx::editor::BodyBlock;
use serde::Serialize;
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub(crate) const MAX_QUERY_SCALARS: usize = 128;
const CONTEXT_SCALARS: usize = 24;
const MAX_CURSORS: usize = 256;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Position {
    block: usize,
    byte: usize,
    scalar: usize,
}

#[derive(Serialize)]
pub(crate) struct Hit {
    body_index: usize,
    start: usize,
    end: usize,
    snippet_start: usize,
    snippet: String,
}

pub(crate) fn find(
    body: &[BodyBlock],
    query: &str,
    limit: usize,
    from: Position,
) -> (Vec<Hit>, Option<Position>) {
    let query_len = query.chars().count();
    let mut hits = Vec::new();
    for (block, item) in body.iter().enumerate().skip(from.block) {
        let BodyBlock::Paragraph(paragraph) = item else {
            continue;
        };
        // Match exactly the existing compact body projection, without building
        // its JSON rows or issuing anchors for unrelated paragraphs.
        let text = &paragraph.text;
        let (base, mut scalar) = if block == from.block {
            (from.byte, from.scalar)
        } else {
            (0, 0)
        };
        let mut previous = base;
        for (relative, matched) in text[base..].match_indices(query) {
            let byte = base + relative;
            scalar += text[previous..byte].chars().count();
            previous = byte;
            if hits.len() == limit {
                // Resume at the first unreturned hit, not at the paragraph start.
                return (
                    hits,
                    Some(Position {
                        block,
                        byte,
                        scalar,
                    }),
                );
            }
            let end = byte + matched.len();
            let (mut left_bytes, mut left_scalars) = (0, 0);
            for c in text[..byte].chars().rev().take(CONTEXT_SCALARS) {
                left_bytes += c.len_utf8();
                left_scalars += 1;
            }
            let right_bytes: usize = text[end..]
                .chars()
                .take(CONTEXT_SCALARS)
                .map(char::len_utf8)
                .sum();
            hits.push(Hit {
                body_index: block,
                start: scalar,
                end: scalar + query_len,
                snippet_start: scalar - left_scalars,
                snippet: text[byte - left_bytes..end + right_bytes].to_owned(),
            });
        }
    }
    (hits, None)
}

type QueryKey = [u8; 32];

pub(crate) fn query_key(query: &str) -> QueryKey {
    Sha256::digest(query.as_bytes()).into()
}

struct Cursor {
    token: Uuid,
    query: QueryKey,
    next: Position,
}

/// Keep a bounded number of opaque bookmarks per live snapshot without
/// retaining query text, snippets, or precomputed hits.
#[derive(Default)]
pub(crate) struct Cursors(VecDeque<Cursor>);

impl Cursors {
    pub(crate) fn resume(
        &self,
        document: &str,
        query: QueryKey,
        cursor: Option<&str>,
    ) -> Result<Position, String> {
        let Some(cursor) = cursor else {
            return Ok(Position::default());
        };
        let invalid = || "invalid_cursor: restart the search with the selected query".to_owned();
        let (owner, local) = cursor.split_once('/').ok_or_else(invalid)?;
        if owner != document {
            return Err("wrong_owner: use a search cursor from the selected document".into());
        }
        let token = Uuid::try_parse(local).map_err(|_| invalid())?;
        self.0
            .iter()
            .find(|bookmark| bookmark.token == token && bookmark.query == query)
            .map(|bookmark| bookmark.next)
            .ok_or_else(invalid)
    }

    pub(crate) fn issue(&mut self, document: &str, query: QueryKey, next: Position) -> String {
        if let Some(existing) = self
            .0
            .iter()
            .find(|bookmark| bookmark.query == query && bookmark.next == next)
        {
            return format!("{document}/{}", existing.token);
        }
        if self.0.len() == MAX_CURSORS {
            self.0.pop_front();
        }
        let token = Uuid::new_v4();
        self.0.push_back(Cursor { token, query, next });
        format!("{document}/{token}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issuing_distinct_bookmarks_keeps_only_the_most_recent_capacity() {
        let mut cursors = Cursors::default();
        let owner = "document";
        let query = query_key("needle");
        let first = cursors.issue(owner, query, Position::default());
        assert_eq!(first, cursors.issue(owner, query, Position::default()));
        let mut latest = String::new();
        for block in 1..=MAX_CURSORS {
            latest = cursors.issue(
                owner,
                query,
                Position {
                    block,
                    ..Position::default()
                },
            );
        }
        assert!(cursors.resume(owner, query, Some(&first)).is_err());
        let resumed = cursors.resume(owner, query, Some(&latest)).unwrap();
        assert_eq!(resumed.block, MAX_CURSORS);
    }
}
