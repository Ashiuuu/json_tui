use ratatui::{
    prelude::Stylize,
    text::{Line, Span, Text},
};
use serde_json::Value;
use slotmap::{DefaultKey, SlotMap};

#[derive(Debug)]
pub struct Tree {
    root: DefaultKey,
    slot_map: SlotMap<DefaultKey, Node>,
    current_node: DefaultKey,
}

#[derive(Debug)]
pub struct Node {
    parent: Option<DefaultKey>,
    highlighted: bool,
    node: NodeType,
}

#[derive(Debug)]
enum NodeType {
    Terminal(Value),
    NonTerminal(HidableValue),
}

#[derive(Debug)]
enum NonTerminalNode {
    Array(Vec<DefaultKey>),
    Object(Vec<(String, DefaultKey)>),
}

#[derive(Debug)]
struct HidableValue {
    visible: bool,
    node: NonTerminalNode,
}

impl Node {
    pub fn is_visible(&self) -> bool {
        match &self.node {
            NodeType::Terminal(_) => true,
            NodeType::NonTerminal(v) => v.is_visible(),
        }
    }
}

impl HidableValue {
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn toggle_visibility(&mut self) {
        self.visible = !self.visible
    }

    pub fn is_array(&self) -> bool {
        match self.node {
            NonTerminalNode::Array(_) => true,
            NonTerminalNode::Object(_) => false,
        }
    }

    pub fn find_last(&self) -> Option<DefaultKey> {
        self.node.find_last()
    }
}

impl NonTerminalNode {
    pub fn find_last(&self) -> Option<DefaultKey> {
        match self {
            NonTerminalNode::Array(arr) => arr.last().copied(),
            NonTerminalNode::Object(obj) => {
                let (_, k) = obj.last().unwrap();
                Some(k).copied()
            }
        }
    }

    pub fn find_next_key(&self, key: DefaultKey) -> Option<DefaultKey> {
        match &self {
            Self::Array(array) => array
                .iter()
                .position(|k| *k == key)
                .and_then(|i| {
                    if i < (array.len() - 1) {
                        array.get(i + 1)
                    } else {
                        None
                    }
                })
                .copied(),
            Self::Object(obj) => obj
                .iter()
                .position(|(_, k)| *k == key)
                .and_then(|i| {
                    if i < (obj.len() - 1) {
                        let o = obj.get(i + 1);
                        let (_, k) = o.unwrap();
                        Some(k)
                    } else {
                        None
                    }
                })
                .copied(),
        }
    }

    pub fn find_previous_key(&self, key: DefaultKey) -> Option<DefaultKey> {
        match &self {
            Self::Array(array) => array
                .iter()
                .position(|k| *k == key)
                //.and_then(|i| i > 0 { array.get(i - 1) } else { None })
                .and_then(|i| if i > 0 { array.get(i - 1) } else { None })
                .copied(),
            Self::Object(obj) => obj
                .iter()
                .position(|(_, k)| *k == key)
                .and_then(|i| {
                    if i > 0 {
                        let o = obj.get(i - 1);
                        let (_, k) = o.unwrap();
                        Some(k)
                    } else {
                        None
                    }
                })
                .copied(),
        }
    }
}

impl Tree {
    const INDENT: &str = "  ";

    pub fn key_to_node(&self, key: DefaultKey) -> &Node {
        self.slot_map.get(key).unwrap()
    }

    pub fn key_to_node_mut(&mut self, key: DefaultKey) -> &mut Node {
        self.slot_map.get_mut(key).unwrap()
    }

    pub fn next_node_down(&mut self) -> Option<DefaultKey> {
        {
            let current_node = self.key_to_node_mut(self.current_node);
            current_node.highlighted = false;
        }

        let current_node = self.key_to_node(self.current_node);

        let next_key = match &current_node.node {
            NodeType::Terminal(_) => {
                let mut current_key = self.current_node;

                loop {
                    let current_node = self.key_to_node(current_key);

                    let next_key = current_node
                        .parent
                        .and_then(|k| self.slot_map.get(k))
                        .and_then(|n| match &n.node {
                            NodeType::NonTerminal(v) => v.node.find_next_key(current_key),
                            NodeType::Terminal(_) => unreachable!(),
                        });

                    if next_key.is_some() {
                        break next_key;
                    }

                    current_key = match current_node.parent {
                        None => break None,
                        Some(k) => k,
                    };
                }
            }
            NodeType::NonTerminal(_) if !current_node.is_visible() => {
                let mut current_key = self.current_node;

                loop {
                    let current_node = self.key_to_node(current_key);

                    let next_key = current_node
                        .parent
                        .and_then(|k| self.slot_map.get(k))
                        .and_then(|n| match &n.node {
                            NodeType::NonTerminal(v) => v.node.find_next_key(current_key),
                            NodeType::Terminal(_) => unreachable!(),
                        });

                    if next_key.is_some() {
                        break next_key;
                    }

                    current_key = match current_node.parent {
                        None => break None,
                        Some(k) => k,
                    };
                }
            }
            NodeType::NonTerminal(v) => match &v.node {
                NonTerminalNode::Array(array) => array.first().cloned(),
                NonTerminalNode::Object(obj) => {
                    let (_, k) = obj.first().unwrap();
                    Some(k).copied()
                }
            },
        };

        if let Some(k) = next_key {
            self.current_node = k;
        }

        {
            let current_node = self.key_to_node_mut(self.current_node);
            current_node.highlighted = true;
        }

        next_key
    }

    pub fn next_node_up(&mut self) -> Option<DefaultKey> {
        {
            let current_node = self.key_to_node_mut(self.current_node);
            current_node.highlighted = false;
        }

        let current_node = self.key_to_node(self.current_node);

        let next_key = {
            let previous_key = current_node.parent.and_then(|k| {
                let node = self.slot_map.get(k);
                match node {
                    None => None,
                    Some(n) => match &n.node {
                        NodeType::NonTerminal(_) if !n.is_visible() => Some(k),
                        NodeType::NonTerminal(v) => v.node.find_previous_key(self.current_node),
                        NodeType::Terminal(_) => unreachable!(),
                    },
                }
            });

            let mut previous_key = previous_key;

            loop {
                let t = match previous_key {
                    s @ Some(k) => {
                        let node = self.key_to_node(k);
                        match &node.node {
                            NodeType::Terminal(_) => break s,
                            NodeType::NonTerminal(_) if !node.is_visible() => break s,
                            NodeType::NonTerminal(v) => v.find_last(),
                        }
                    }
                    None => break current_node.parent,
                };
                previous_key = t;
            }
        };

        if let Some(k) = next_key {
            self.current_node = k;
        }

        {
            let current_node = self.key_to_node_mut(self.current_node);
            current_node.highlighted = true;
        }

        next_key
    }

    pub fn from_value(v: Value) -> Self {
        let mut slot_map = SlotMap::new();
        let root_key = value_to_key(v, &mut slot_map, None);

        let mut ret = Self {
            root: root_key,
            slot_map,
            current_node: root_key,
        };

        ret.highlight_current_node();

        ret
    }

    pub fn toggle_current_node_visibility(&mut self) {
        let node = self.slot_map.get_mut(self.current_node).unwrap();
        match &mut node.node {
            NodeType::Terminal(_) => (),
            NodeType::NonTerminal(v) => {
                v.toggle_visibility();
            }
        }
    }

    pub fn highlight_current_node(&mut self) {
        let node = self.key_to_node_mut(self.current_node);
        node.highlighted = true;
    }

    pub fn toggle_current_node_highlight(&mut self) {
        let node = self.key_to_node_mut(self.current_node);
        node.highlighted = !node.highlighted;
    }

    pub fn to_text(&self) -> Text<'_> {
        self.to_text_inner(0, self.root)
    }

    pub fn find_current_line(&self) -> usize {
        let mut line_counter = 0;

        match self.find_line_recursive(&mut line_counter, self.root) {
            None => line_counter,
            Some(n) => n,
        }
    }

    fn find_line_recursive(
        &self,
        line_counter: &mut usize,
        current_node: DefaultKey,
    ) -> Option<usize> {
        if current_node == self.current_node {
            return Some(*line_counter);
        }

        let node = self.key_to_node(current_node);

        match &node.node {
            NodeType::Terminal(_) => (),
            NodeType::NonTerminal(v) => match &v.node {
                NonTerminalNode::Array(array) => {
                    *line_counter += 1;

                    for (i, key) in array.iter().enumerate() {
                        if let Some(n) = self.find_line_recursive(line_counter, *key) {
                            return Some(n);
                        }

                        if i < array.len() - 1 {
                            *line_counter += 1;
                        }
                    }

                    *line_counter += 1;
                }
                NonTerminalNode::Object(obj) => {
                    *line_counter += 1;

                    for (i, (_, key)) in obj.iter().enumerate() {
                        if let Some(n) = self.find_line_recursive(line_counter, *key) {
                            return Some(n);
                        }

                        if i < obj.len() - 1 {
                            *line_counter += 1;
                        }
                    }

                    *line_counter += 1;
                }
            },
        }

        None
    }

    fn to_text_inner(&self, indent_level: usize, current_node: DefaultKey) -> Text<'_> {
        let node = self.key_to_node(current_node);

        let ret = match &node.node {
            NodeType::Terminal(v) => match v {
                Value::Number(n) => Text::raw(format!("{n}")),
                Value::Bool(b) => Text::raw(format!("{b}")),
                Value::String(s) => Text::raw(format!("\"{s}\"")),
                Value::Null => Text::raw("{{}}"),
                _ => unreachable!(),
            },
            NodeType::NonTerminal(v) => {
                if v.is_visible() {
                    match &v.node {
                        NonTerminalNode::Array(array) => {
                            let mut ret = Text::raw("[\n");

                            let indent_level = indent_level + 1;
                            let indent = Text::raw(Self::INDENT.repeat(indent_level));

                            for (i, v) in array.iter().enumerate() {
                                let tmp =
                                    join_text(indent.clone(), self.to_text_inner(indent_level, *v));
                                ret.extend(tmp);

                                let tmp = if i == (array.len() - 1) {
                                    let indent = Self::INDENT.repeat(indent_level - 1);
                                    Text::raw(format!("\n{indent}]"))
                                } else {
                                    Text::raw(",\n")
                                };
                                ret = join_text(ret, tmp)
                            }
                            ret
                        }
                        NonTerminalNode::Object(map) => {
                            let mut ret = Text::raw("{\n");

                            let indent_level = indent_level + 1;
                            let indent = Text::raw(Self::INDENT.repeat(indent_level));

                            for (i, (key, v)) in map.iter().enumerate() {
                                ret.extend(Text::raw(format!("{indent}\"{key}\": ")));
                                ret = join_text(ret, self.to_text_inner(indent_level, *v));

                                let tmp = if i == (map.len() - 1) {
                                    let indent = Self::INDENT.repeat(indent_level - 1);
                                    Text::raw(format!("\n{indent}}}"))
                                } else {
                                    Text::raw(",\n")
                                };
                                ret = join_text(ret, tmp);
                            }
                            ret
                        }
                    }
                } else if v.is_array() {
                    Text::raw("[...]")
                } else {
                    Text::raw("{...}")
                }
            }
        };

        if node.highlighted {
            ret.lines
                .into_iter()
                .map(|l| {
                    l.spans
                        .into_iter()
                        .map(|s| s.white().on_dark_gray())
                        .collect::<Vec<Span>>()
                        .into()
                })
                .collect::<Vec<Line>>()
                .into()
        } else {
            ret
        }
    }
}

fn join_text<'a>(mut a: Text<'a>, b: Text<'a>) -> Text<'a> {
    let (b_first, b_rest) = b.lines.split_at(1);
    for span in b_first[0].spans.iter() {
        a.push_span(span.clone());
    }
    a.extend(Text::from(b_rest.to_vec()));
    a
}

pub fn value_to_key(
    value: Value,
    slot_map: &mut SlotMap<DefaultKey, Node>,
    parent: Option<DefaultKey>,
) -> DefaultKey {
    match value {
        v @ (Value::Null | Value::String(_) | Value::Number(_) | Value::Bool(_)) => {
            let node = NodeType::Terminal(v);
            let node = Node {
                parent,
                node,
                highlighted: false,
            };
            slot_map.insert(node)
        }
        Value::Object(map) => {
            let node = Node {
                parent,
                highlighted: false,
                node: NodeType::NonTerminal(HidableValue {
                    visible: true,
                    node: NonTerminalNode::Object(vec![]),
                }),
            };
            let parent_key = slot_map.insert(node);

            let mut vec = vec![];

            for (k, v) in map {
                let key = value_to_key(v, slot_map, Some(parent_key));
                vec.push((k, key));
            }

            // we know all this is safe by construction
            let node = slot_map.get_mut(parent_key).unwrap();

            match &mut node.node {
                NodeType::NonTerminal(hv) => match &mut hv.node {
                    NonTerminalNode::Object(v) => {
                        let _ = std::mem::replace(v, vec);
                    }
                    _ => unreachable!(),
                },
                _ => unreachable!(),
            }
            parent_key
        }
        Value::Array(array) => {
            let node = Node {
                parent,
                highlighted: false,
                node: NodeType::NonTerminal(HidableValue {
                    visible: true,
                    node: NonTerminalNode::Array(vec![]),
                }),
            };
            let parent_key = slot_map.insert(node);

            let mut vec = vec![];

            for v in array {
                let key = value_to_key(v, slot_map, Some(parent_key));
                vec.push(key);
            }

            // we know all this is safe by construction
            let node = slot_map.get_mut(parent_key).unwrap();

            match &mut node.node {
                NodeType::NonTerminal(hv) => match &mut hv.node {
                    NonTerminalNode::Array(v) => {
                        let _ = std::mem::replace(v, vec);
                    }
                    _ => unreachable!(),
                },
                _ => unreachable!(),
            }
            parent_key
        }
    }
}
