use std::{io, path::Path};

use crate::error::TaktError;

struct TagNode {
    name: String,
    children: Vec<TagNode>,
}

pub struct TagTree {
    root: Vec<TagNode>,
}

impl TagTree {
    pub fn load(path: &Path) -> Result<TagTree, io::Error> {
        match std::fs::read_to_string(path) {
            Ok(content) => Ok(Self::parse(&content)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                Ok(TagTree { root: Vec::new() })
            }
            Err(e) => Err(e),
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), std::io::Error> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, self.write())
    }

    pub fn parse(content: &str) -> TagTree {
        let mut lines = content
            .lines()
            .map(|line| {
                let trimmed = line.trim_start();
                let depth = (line.len() - trimmed.len()) / 2;
                (depth, trimmed.trim_end())
            })
            .peekable();
        TagTree {
            root: Self::parse_nodes(&mut lines, 0),
        }
    }

    fn parse_nodes<'a>(
        lines: &mut std::iter::Peekable<impl Iterator<Item = (usize, &'a str)>>,
        depth: usize,
    ) -> Vec<TagNode> {
        let mut nodes = Vec::new();

        while let Some((d, name)) = lines.peek().copied() {
            if d < depth {
                break;
            }
            if d == depth {
                lines.next();
                let children = Self::parse_nodes(lines, depth + 1);
                nodes.push(TagNode {
                    name: name.into(),
                    children,
                })
            }
        }
        nodes
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
        let tree = TagTree::parse(SAMPLE);
        let output = tree.write();
        assert_eq!(output.trim_end(), SAMPLE);
    }

    #[test]
    fn resolve_unique_leaf() {
        let tree = TagTree::parse(SAMPLE);
        assert_eq!(tree.resolve("fix-bug").unwrap(), "work/project-x/fix-bug");
    }

    #[test]
    fn resolve_unique_intermediate() {
        let tree = TagTree::parse(SAMPLE);
        assert_eq!(tree.resolve("project-x").unwrap(), "work/project-x");
    }

    #[test]
    fn resolve_top_level() {
        let tree = TagTree::parse(SAMPLE);
        assert_eq!(tree.resolve("personal").unwrap(), "personal");
    }

    #[test]
    fn resolve_unknown() {
        let tree = TagTree::parse(SAMPLE);
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
        let tree = TagTree::parse(input);
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
        let mut tree = TagTree::parse(SAMPLE);
        tree.add("work/project-x/new-task");
        assert_eq!(
            tree.resolve("new-task").unwrap(),
            "work/project-x/new-task"
        );
    }

    #[test]
    fn add_entirely_new_branch() {
        let mut tree = TagTree::parse(SAMPLE);
        tree.add("hobbies/guitar");
        assert_eq!(tree.resolve("guitar").unwrap(), "hobbies/guitar");
    }

    #[test]
    fn add_does_not_duplicate_existing() {
        let mut tree = TagTree::parse(SAMPLE);
        tree.add("work/project-x/fix-bug");
        // should still resolve uniquely, not create a duplicate
        assert_eq!(tree.resolve("fix-bug").unwrap(), "work/project-x/fix-bug");
    }
}
