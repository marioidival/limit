// Virtual DOM implementation for terminal UI rendering

use std::collections::HashMap;

/// Virtual DOM node
#[derive(Debug, Clone, PartialEq)]
pub enum VNode {
    Text(String),
    Element {
        tag: String,
        attrs: HashMap<String, String>,
        children: Vec<VNode>,
    },
}

/// Patch operation for VNode tree updates
#[derive(Debug, Clone, PartialEq)]
pub enum Patch {
    Replace(VNode),
    UpdateAttrs {
        add: HashMap<String, String>,
        remove: Vec<String>,
    },
    InsertChild(usize, VNode),
    RemoveChild(usize),
}

/// Render VNode to terminal string representation
pub fn render(vnode: &VNode) -> String {
    match vnode {
        VNode::Text(text) => text.clone(),
        VNode::Element {
            tag,
            attrs,
            children,
        } => {
            let mut result = format!("[{}]", tag);
            if !attrs.is_empty() {
                result.push(' ');
                let attr_str: Vec<String> =
                    attrs.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
                result.push_str(&attr_str.join(" "));
            }
            result.push('\n');

            for child in children {
                let child_str = render(child);
                for line in child_str.lines() {
                    result.push_str("  ");
                    result.push_str(line);
                    result.push('\n');
                }
            }

            result
        }
    }
}

/// Compute minimal patches between two VNode trees
pub fn diff(old: &VNode, new: &VNode) -> Vec<Patch> {
    match (old, new) {
        (VNode::Text(old_text), VNode::Text(new_text)) => {
            if old_text == new_text {
                vec![]
            } else {
                vec![Patch::Replace(new.clone())]
            }
        }
        (
            VNode::Element {
                tag: old_tag,
                attrs: old_attrs,
                children: old_children,
            },
            VNode::Element {
                tag: new_tag,
                attrs: new_attrs,
                children: new_children,
            },
        ) => {
            let mut patches = vec![];

            // Tag change requires full replacement
            if old_tag != new_tag {
                return vec![Patch::Replace(new.clone())];
            }

            // Attribute diff
            let mut add_attrs = HashMap::new();
            let mut remove_attrs = Vec::new();

            // Check for removed or changed attributes
            for (key, old_val) in old_attrs.iter() {
                match new_attrs.get(key) {
                    Some(new_val) => {
                        if old_val != new_val {
                            add_attrs.insert(key.clone(), new_val.clone());
                        }
                    }
                    None => {
                        remove_attrs.push(key.clone());
                    }
                }
            }

            // Check for new attributes
            for (key, new_val) in new_attrs.iter() {
                if !old_attrs.contains_key(key) {
                    add_attrs.insert(key.clone(), new_val.clone());
                }
            }

            if !add_attrs.is_empty() || !remove_attrs.is_empty() {
                patches.push(Patch::UpdateAttrs {
                    add: add_attrs,
                    remove: remove_attrs,
                });
            }

            // Children diff
            let old_len = old_children.len();
            let _new_len = new_children.len();

            // Find common prefix
            let common_prefix_len = old_children
                .iter()
                .zip(new_children.iter())
                .take_while(|(o, n)| o == n)
                .count();

            // Process additions
            for (i, child) in new_children.iter().enumerate().skip(common_prefix_len) {
                patches.push(Patch::InsertChild(i, child.clone()));
            }

            // Process removals
            for _i in common_prefix_len..old_len {
                patches.push(Patch::RemoveChild(common_prefix_len));
            }

            patches
        }
        (_, _) => vec![Patch::Replace(new.clone())],
    }
}

/// Apply patches to a VNode tree
pub fn apply(node: &mut VNode, patches: Vec<Patch>) {
    let node_ref = node;

    for patch in patches {
        match patch {
            Patch::Replace(new_node) => {
                *node_ref = new_node;
            }
            Patch::UpdateAttrs { add, remove } => {
                if let VNode::Element { attrs, .. } = node_ref {
                    for key in remove {
                        attrs.remove(&key);
                    }
                    for (key, value) in add {
                        attrs.insert(key, value);
                    }
                }
            }
            Patch::InsertChild(index, child) => {
                if let VNode::Element { children, .. } = node_ref {
                    children.insert(index, child);
                }
            }
            Patch::RemoveChild(index) => {
                if let VNode::Element { children, .. } = node_ref {
                    if index < children.len() {
                        children.remove(index);
                    }
                }
            }
        }
    }
}

// Helper methods for VNode
impl VNode {
    pub fn children(&self) -> Option<&Vec<VNode>> {
        match self {
            VNode::Element { children, .. } => Some(children),
            _ => None,
        }
    }

    pub fn attrs(&self) -> Option<&HashMap<String, String>> {
        match self {
            VNode::Element { attrs, .. } => Some(attrs),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_text() {
        let node = VNode::Text("Hello, World!".to_string());
        assert_eq!(render(&node), "Hello, World!");
    }

    #[test]
    fn test_render_element() {
        let node = VNode::Element {
            tag: "div".to_string(),
            attrs: {
                let mut map = HashMap::new();
                map.insert("id".to_string(), "test".to_string());
                map
            },
            children: vec![VNode::Text("Content".to_string())],
        };

        let result = render(&node);
        assert!(result.contains("[div]"));
        assert!(result.contains("id=test"));
        assert!(result.contains("Content"));
    }

    #[test]
    fn test_diff_no_change() {
        let node = VNode::Text("Same".to_string());
        let patches = diff(&node, &node);
        assert!(patches.is_empty());
    }

    #[test]
    fn test_diff_text_change() {
        let old = VNode::Text("Old".to_string());
        let new = VNode::Text("New".to_string());
        let patches = diff(&old, &new);

        assert_eq!(patches.len(), 1);
        assert_eq!(patches[0], Patch::Replace(new.clone()));
    }

    #[test]
    fn test_diff_add_child() {
        let old = VNode::Element {
            tag: "div".to_string(),
            attrs: HashMap::new(),
            children: vec![VNode::Text("First".to_string())],
        };

        let new = VNode::Element {
            tag: "div".to_string(),
            attrs: HashMap::new(),
            children: vec![
                VNode::Text("First".to_string()),
                VNode::Text("Second".to_string()),
            ],
        };

        let patches = diff(&old, &new);

        assert_eq!(patches.len(), 1);
        assert_eq!(
            patches[0],
            Patch::InsertChild(1, VNode::Text("Second".to_string()))
        );
    }

    #[test]
    fn test_diff_remove_child() {
        let old = VNode::Element {
            tag: "div".to_string(),
            attrs: HashMap::new(),
            children: vec![
                VNode::Text("First".to_string()),
                VNode::Text("Second".to_string()),
            ],
        };

        let new = VNode::Element {
            tag: "div".to_string(),
            attrs: HashMap::new(),
            children: vec![VNode::Text("First".to_string())],
        };

        let patches = diff(&old, &new);

        assert_eq!(patches.len(), 1);
        assert_eq!(patches[0], Patch::RemoveChild(1));
    }

    #[test]
    fn test_apply_patch() {
        let mut node = VNode::Text("Old".to_string());
        let patches = vec![Patch::Replace(VNode::Text("New".to_string()))];
        apply(&mut node, patches);

        assert_eq!(node, VNode::Text("New".to_string()));
    }

    #[test]
    fn test_apply_insert_child() {
        let mut node = VNode::Element {
            tag: "div".to_string(),
            attrs: HashMap::new(),
            children: vec![VNode::Text("First".to_string())],
        };

        let patches = vec![Patch::InsertChild(1, VNode::Text("Second".to_string()))];
        apply(&mut node, patches);

        assert_eq!(node.children().unwrap().len(), 2);
        assert_eq!(
            node.children().unwrap()[1],
            VNode::Text("Second".to_string())
        );
    }

    #[test]
    fn test_apply_remove_child() {
        let mut node = VNode::Element {
            tag: "div".to_string(),
            attrs: HashMap::new(),
            children: vec![
                VNode::Text("First".to_string()),
                VNode::Text("Second".to_string()),
            ],
        };

        let patches = vec![Patch::RemoveChild(1)];
        apply(&mut node, patches);

        assert_eq!(node.children().unwrap().len(), 1);
    }

    #[test]
    fn test_apply_update_attrs() {
        let mut node = VNode::Element {
            tag: "div".to_string(),
            attrs: {
                let mut map = HashMap::new();
                map.insert("id".to_string(), "old".to_string());
                map
            },
            children: vec![],
        };

        let patches = vec![Patch::UpdateAttrs {
            add: {
                let mut map = HashMap::new();
                map.insert("class".to_string(), "test".to_string());
                map
            },
            remove: vec!["id".to_string()],
        }];

        apply(&mut node, patches);

        let attrs = node.attrs().unwrap();
        assert!(!attrs.contains_key("id"));
        assert_eq!(attrs.get("class"), Some(&"test".to_string()));
    }
}
