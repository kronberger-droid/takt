use std::{io, path::Path};

use crate::error::TaktError;

#[derive(Debug)]
struct TagNode {
    name: String,
    children: Vec<TagNode>,
}

#[derive(Debug)]
pub(crate) struct TagTree {
    root: Vec<TagNode>,
}

impl TagTree {
    pub fn load(path: &Path) -> Result<TagTree, TaktError> {
        match std::fs::read_to_string(path) {
            Ok(content) => Ok(Self::parse(&content)?),
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                Ok(TagTree { root: Vec::new() })
            }
            Err(e) => Err(TaktError::Io(e)),
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), TaktError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, self.write())?;
        Ok(())
    }

    pub fn lex(content: &str) -> Result<Vec<(usize, usize, &str)>, TaktError> {
        let mut output = Vec::new();

        for (i, raw) in content.lines().enumerate() {
            let line = raw.trim_end();

            if line.is_empty() {
                continue;
            }

            let indent = line.bytes().take_while(|&b| b == b' ').count();
            if indent % 2 != 0 {
                return Err(TaktError::MalformedLine {
                    line: i,
                    content: line.into(),
                });
            }
            let depth = indent / 2;
            let name = &line[indent..];
            if name.contains(char::is_whitespace) || name.contains('/') {
                return Err(TaktError::MalformedLine {
                    line: i,
                    content: line.into(),
                });
            }
            output.push((i, depth, name));
        }
        Ok(output)
    }

    pub fn parse(content: &str) -> Result<TagTree, TaktError> {
        let lexed = Self::lex(content)?;

        let mut lines = lexed.into_iter().peekable();

        Ok(TagTree {
            root: Self::parse_nodes(0, &mut lines)?,
        })
    }

    fn parse_nodes<'a>(
        depth: usize,
        lines: &mut std::iter::Peekable<
            impl Iterator<Item = (usize, usize, &'a str)>,
        >,
    ) -> Result<Vec<TagNode>, TaktError> {
        let mut nodes = Vec::new();

        while let Some((i, d, name)) = lines.peek().copied() {
            match d.cmp(&depth) {
                std::cmp::Ordering::Less => break,
                std::cmp::Ordering::Equal => {
                    lines.next();
                    let children = Self::parse_nodes(depth + 1, lines);
                    nodes.push(TagNode {
                        name: name.into(),
                        children: children?,
                    })
                }
                std::cmp::Ordering::Greater => {
                    return Err(TaktError::UnexpectedIndent {
                        line: i,
                        max: depth,
                        depth: d,
                    });
                }
            }
        }
        Ok(nodes)
    }

    pub fn write(&self) -> String {
        let mut out = String::new();
        Self::write_nodes(&self.root, 0, &mut out);
        out
    }

    fn write_nodes(nodes: &[TagNode], depth: usize, out: &mut String) {
        for node in nodes {
            out.push_str(&" ".repeat(depth * 2));
            out.push_str(&node.name);
            out.push('\n');
            Self::write_nodes(&node.children, depth + 1, out);
        }
    }

    pub fn resolve(&self, name: &str) -> Result<String, TaktError> {
        let matches = Self::resolve_nodes(&self.root, name, &mut Vec::new());
        match matches.len() {
            0 => Err(TaktError::UnknownTag(name.to_string())),
            1 => Ok(matches.into_iter().next().unwrap()),
            _ => Err(TaktError::AmbiguousTag(matches)),
        }
    }

    fn resolve_nodes(
        nodes: &[TagNode],
        name: &str,
        path: &mut Vec<String>,
    ) -> Vec<String> {
        let mut matches = Vec::new();

        for node in nodes {
            path.push(node.name.clone());

            if node.name == name {
                matches.push(path.join("/"));
            }

            matches.extend(Self::resolve_nodes(&node.children, name, path));
            path.pop();
        }
        matches
    }

    pub fn add(&mut self, path: &str) {
        let mut nodes = &mut self.root;

        for segment in path.split('/') {
            let idx = nodes.iter().position(|n| n.name == segment);

            match idx {
                Some(i) => nodes = &mut nodes[i].children,
                None => {
                    nodes.push(TagNode {
                        name: segment.into(),
                        children: Vec::new(),
                    });
                    let last = nodes.len() - 1;
                    nodes = &mut nodes[last].children;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
work
  project-x
    fix-bug
    implement-api
  project-y
study
  math
    linear-algebra
  rust
personal";

    #[test]
    fn parse_and_write_round_trip() {
        let tree = TagTree::parse(SAMPLE).unwrap();
        let output = tree.write();
        assert_eq!(output.trim_end(), SAMPLE);
    }

    #[test]
    fn resolve_unique_leaf() {
        let tree = TagTree::parse(SAMPLE).unwrap();
        assert_eq!(tree.resolve("fix-bug").unwrap(), "work/project-x/fix-bug");
    }

    #[test]
    fn resolve_unique_intermediate() {
        let tree = TagTree::parse(SAMPLE).unwrap();
        assert_eq!(tree.resolve("project-x").unwrap(), "work/project-x");
    }

    #[test]
    fn resolve_top_level() {
        let tree = TagTree::parse(SAMPLE).unwrap();
        assert_eq!(tree.resolve("personal").unwrap(), "personal");
    }

    #[test]
    fn resolve_unknown() {
        let tree = TagTree::parse(SAMPLE).unwrap();
        let err = tree.resolve("nonexistent").unwrap_err();
        assert!(matches!(err, TaktError::UnknownTag(_)));
    }

    #[test]
    fn resolve_ambiguous() {
        let input = "\
a
  shared
b
  shared";
        let tree = TagTree::parse(input).unwrap();
        let err = tree.resolve("shared").unwrap_err();
        match err {
            TaktError::AmbiguousTag(paths) => {
                assert_eq!(paths, vec!["a/shared", "b/shared"]);
            }
            _ => panic!("expected Ambiguous"),
        }
    }

    #[test]
    fn add_new_path() {
        let mut tree = TagTree::parse(SAMPLE).unwrap();
        tree.add("work/project-x/new-task");
        assert_eq!(
            tree.resolve("new-task").unwrap(),
            "work/project-x/new-task"
        );
    }

    #[test]
    fn add_entirely_new_branch() {
        let mut tree = TagTree::parse(SAMPLE).unwrap();
        tree.add("hobbies/guitar");
        assert_eq!(tree.resolve("guitar").unwrap(), "hobbies/guitar");
    }

    #[test]
    fn add_does_not_duplicate_existing() {
        let mut tree = TagTree::parse(SAMPLE).unwrap();
        tree.add("work/project-x/fix-bug");
        // should still resolve uniquely, not create a duplicate
        assert_eq!(tree.resolve("fix-bug").unwrap(), "work/project-x/fix-bug");
    }

    #[test]
    fn parse_rejects_depth_jump() {
        let input = "foo\n    bar"; // jumps from 0 to 2
        let err = TagTree::parse(input).unwrap_err();
        assert!(matches!(err, TaktError::UnexpectedIndent { .. }))
    }

    #[test]
    fn parse_rejects_indented_first_line() {
        let input = "  foo"; // top-level line can't be indented
        let err = TagTree::parse(input).unwrap_err();
        assert!(matches!(err, TaktError::UnexpectedIndent { .. }));
    }

    #[test]
    fn lex_rejects_odd_indentation() {
        let input = "foo\n   bar"; // 3 spaces — not a multiple of 2
        let err = TagTree::parse(input).unwrap_err();
        assert!(matches!(err, TaktError::MalformedLine { .. }));
    }

    #[test]
    fn lex_rejects_tab_indentation() {
        let input = "\tfoo"; // tab leaves a whitespace char in the name
        let err = TagTree::parse(input).unwrap_err();
        assert!(matches!(err, TaktError::MalformedLine { .. }));
    }

    #[test]
    fn lex_rejects_whitespace_in_name() {
        let input = "foo bar"; // internal space
        let err = TagTree::parse(input).unwrap_err();
        assert!(matches!(err, TaktError::MalformedLine { .. }));
    }

    #[test]
    fn parse_skips_blank_lines() {
        let input = "work\n\n  project-x\n\n";
        let tree = TagTree::parse(input).unwrap();
        assert_eq!(tree.resolve("project-x").unwrap(), "work/project-x");
    }

    #[test]
    fn parse_tolerates_trailing_whitespace() {
        let input = "work   \n  project-x  ";
        let tree = TagTree::parse(input).unwrap();
        assert_eq!(tree.resolve("project-x").unwrap(), "work/project-x");
    }
}
