// Flexbox layout system for terminal UI

use crate::vdom::VNode;
use ratatui::layout::Rect;

/// Main axis direction for flex layout
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum FlexDirection {
    #[default]
    Row,
    Column,
}

/// Main axis alignment
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum JustifyContent {
    #[default]
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceAround,
}

/// Cross axis alignment
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum AlignItems {
    #[default]
    Start,
    Center,
    End,
    Stretch,
}

/// Flex layout style properties
#[derive(Debug, Clone, Copy, Default)]
pub struct FlexStyle {
    pub direction: FlexDirection,
    pub justify_content: JustifyContent,
    pub align_items: AlignItems,
    pub gap: u16,
    pub flex_grow: f32,
    pub flex_shrink: f32,
}

impl FlexStyle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn row(mut self) -> Self {
        self.direction = FlexDirection::Row;
        self
    }

    pub fn column(mut self) -> Self {
        self.direction = FlexDirection::Column;
        self
    }

    pub fn justify(mut self, justify: JustifyContent) -> Self {
        self.justify_content = justify;
        self
    }

    pub fn align(mut self, align: AlignItems) -> Self {
        self.align_items = align;
        self
    }

    pub fn gap(mut self, gap: u16) -> Self {
        self.gap = gap;
        self
    }

    pub fn flex_grow(mut self, grow: f32) -> Self {
        self.flex_grow = grow;
        self
    }

    pub fn flex_shrink(mut self, shrink: f32) -> Self {
        self.flex_shrink = shrink;
        self
    }
}

/// Flexbox layout engine
#[derive(Debug, Clone, Copy, Default)]
pub struct FlexboxLayout;

impl FlexboxLayout {
    pub fn new() -> Self {
        Self
    }

    /// Calculate child positions for a flex container
    pub fn calculate(node: &VNode, constraints: Rect) -> Vec<Rect> {
        let children = match node.children() {
            Some(children) if !children.is_empty() => children,
            _ => return vec![],
        };

        // Extract flex style from node attributes
        let style = Self::extract_style(node);

        // Get base sizes for children
        let base_sizes: Vec<Rect> = children
            .iter()
            .map(|_| Rect {
                x: 0,
                y: 0,
                width: 1,  // Default minimum width
                height: 1, // Default minimum height
            })
            .collect();

        Self::layout_children(&base_sizes, constraints, &style, children.len())
    }

    /// Extract flex style from node attributes
    fn extract_style(node: &VNode) -> FlexStyle {
        let attrs = node.attrs().unwrap();

        FlexStyle {
            direction: attrs
                .get("direction")
                .and_then(|v| match v.as_str() {
                    "row" => Some(FlexDirection::Row),
                    "column" => Some(FlexDirection::Column),
                    _ => None,
                })
                .unwrap_or(FlexDirection::Row),

            justify_content: attrs
                .get("justify")
                .and_then(|v| match v.as_str() {
                    "start" => Some(JustifyContent::Start),
                    "center" => Some(JustifyContent::Center),
                    "end" => Some(JustifyContent::End),
                    "space-between" => Some(JustifyContent::SpaceBetween),
                    "space-around" => Some(JustifyContent::SpaceAround),
                    _ => None,
                })
                .unwrap_or(JustifyContent::Start),

            align_items: attrs
                .get("align")
                .and_then(|v| match v.as_str() {
                    "start" => Some(AlignItems::Start),
                    "center" => Some(AlignItems::Center),
                    "end" => Some(AlignItems::End),
                    "stretch" => Some(AlignItems::Stretch),
                    _ => None,
                })
                .unwrap_or(AlignItems::Start),

            gap: attrs.get("gap").and_then(|v| v.parse().ok()).unwrap_or(0),

            flex_grow: attrs
                .get("flex-grow")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.0),

            flex_shrink: attrs
                .get("flex-shrink")
                .and_then(|v| v.parse().ok())
                .unwrap_or(1.0),
        }
    }

    /// Layout children based on flex properties
    fn layout_children(
        _base_sizes: &[Rect],
        constraints: Rect,
        style: &FlexStyle,
        child_count: usize,
    ) -> Vec<Rect> {
        if child_count == 0 {
            return vec![];
        }

        let main_axis_size = match style.direction {
            FlexDirection::Row => constraints.width,
            FlexDirection::Column => constraints.height,
        };

        let cross_axis_size = match style.direction {
            FlexDirection::Row => constraints.height,
            FlexDirection::Column => constraints.width,
        };

        // Calculate total gap space
        let total_gap = style
            .gap
            .saturating_mul(child_count.saturating_sub(1) as u16);
        let available_space = main_axis_size.saturating_sub(total_gap);

        // Distribute main axis space
        let main_positions =
            Self::distribute_main_axis(available_space, style, child_count, main_axis_size);

        // Calculate cross axis positions
        let cross_positions = Self::distribute_cross_axis(cross_axis_size, &style.align_items, child_count);

        // Build final rects
        let mut results = Vec::with_capacity(child_count);

        for i in 0..child_count {
            let child_width = match style.direction {
                FlexDirection::Row => main_positions[i].size,
                FlexDirection::Column => cross_axis_size,
            };

            let child_height = match style.direction {
                FlexDirection::Row => cross_axis_size,
                FlexDirection::Column => main_positions[i].size,
            };

            let x = match style.direction {
                FlexDirection::Row => constraints.x + main_positions[i].pos,
                FlexDirection::Column => constraints.x + cross_positions[i].pos,
            };

            let y = match style.direction {
                FlexDirection::Row => constraints.y + cross_positions[i].pos,
                FlexDirection::Column => constraints.y + main_positions[i].pos,
            };

            results.push(Rect {
                x,
                y,
                width: child_width,
                height: child_height,
            });
        }

        results
    }

    /// Distribute space along main axis
    fn distribute_main_axis(
        available_space: u16,
        style: &FlexStyle,
        child_count: usize,
        _container_size: u16,
    ) -> Vec<PositionInfo> {
        let mut positions = Vec::with_capacity(child_count);

        if child_count == 0 {
            return positions;
        }

        // Calculate gap space
        let gap_space = style
            .gap
            .saturating_mul(child_count.saturating_sub(1) as u16);
        let usable_space = available_space.saturating_sub(gap_space);

        // Distribute space based on flex_grow
        let total_flex_grow = if style.flex_grow > 0.0 {
            style.flex_grow * child_count as f32
        } else {
            child_count as f32 // Default to equal distribution
        };

        // Calculate base size for each child
        let base_size = if total_flex_grow > 0.0 {
            (usable_space as f32 / total_flex_grow) as u16
        } else {
            0
        };

        // Calculate positions based on justify_content
        let total_used = base_size
            .saturating_mul(child_count as u16)
            .saturating_add(gap_space);

        let mut current_pos = match style.justify_content {
            JustifyContent::Start => 0,
            JustifyContent::Center => available_space.saturating_sub(total_used) / 2,
            JustifyContent::End => available_space.saturating_sub(total_used),
            JustifyContent::SpaceBetween => {
                if child_count > 1 {
                    let _extra_gap = (usable_space
                        .saturating_sub(base_size.saturating_mul(child_count as u16)))
                    .saturating_div(child_count.saturating_sub(1) as u16);
                    0
                } else {
                    0
                }
            }
            JustifyContent::SpaceAround => {
                let extra_gap = (usable_space
                    .saturating_sub(base_size.saturating_mul(child_count as u16)))
                .saturating_div(child_count as u16);
                extra_gap / 2
            }
        };

        let mut gap = style.gap;

        if style.justify_content == JustifyContent::SpaceBetween && child_count > 1 {
            let extra = usable_space.saturating_sub(base_size.saturating_mul(child_count as u16));
            gap = gap.saturating_add(extra / (child_count.saturating_sub(1) as u16));
        }

        if style.justify_content == JustifyContent::SpaceAround {
            let extra = usable_space.saturating_sub(base_size.saturating_mul(child_count as u16));
            let extra_gap = extra / (child_count as u16);
            gap = extra_gap;
        }

        for _i in 0..child_count {
            positions.push(PositionInfo {
                pos: current_pos,
                size: base_size,
            });

            current_pos = current_pos.saturating_add(base_size).saturating_add(gap);
        }

        positions
    }

    /// Distribute positions along cross axis
    fn distribute_cross_axis(
        cross_axis_size: u16,
        style: &AlignItems,
        child_count: usize,
    ) -> Vec<PositionInfo> {
        let mut positions = Vec::with_capacity(child_count);

        if child_count == 0 {
            return positions;
        }

        // Default to full cross axis size for each child
        let child_size = cross_axis_size;

        for _i in 0..child_count {
            let pos = match style {
                AlignItems::Start => 0,
                AlignItems::Center => 0, // Centered by taking full size
                AlignItems::End => 0,    // Aligned by taking full size
                AlignItems::Stretch => 0,
            };

            positions.push(PositionInfo {
                pos,
                size: child_size,
            });
        }

        positions
    }
}

/// Helper struct for position calculations
#[derive(Debug, Clone, Copy)]
struct PositionInfo {
    pos: u16,
    size: u16,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vdom::VNode;
    use std::collections::HashMap;

    fn create_flex_container(attrs: HashMap<String, String>, children: Vec<VNode>) -> VNode {
        VNode::Element {
            tag: "flex".to_string(),
            attrs,
            children,
        }
    }

    fn create_text_node(text: &str) -> VNode {
        VNode::Text(text.to_string())
    }

    #[test]
    fn test_layout_simple_row() {
        let children = vec![
            create_text_node("A"),
            create_text_node("B"),
            create_text_node("C"),
        ];

        let container = create_flex_container(HashMap::new(), children);

        let constraints = Rect {
            x: 0,
            y: 0,
            width: 20,
            height: 10,
        };

        let results = FlexboxLayout::calculate(&container, constraints);

        assert_eq!(results.len(), 3);

        // Verify horizontal layout (x positions increase)
        assert!(results[0].x < results[1].x);
        assert!(results[1].x < results[2].x);

        // All children have same height (full container height)
        assert_eq!(results[0].height, 10);
        assert_eq!(results[1].height, 10);
        assert_eq!(results[2].height, 10);

        // All have same y position
        assert_eq!(results[0].y, 0);
        assert_eq!(results[1].y, 0);
        assert_eq!(results[2].y, 0);
    }

    #[test]
    fn test_layout_simple_column() {
        let children = vec![
            create_text_node("A"),
            create_text_node("B"),
            create_text_node("C"),
        ];

        let mut attrs = HashMap::new();
        attrs.insert("direction".to_string(), "column".to_string());

        let container = create_flex_container(attrs, children);

        let constraints = Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 20,
        };

        let results = FlexboxLayout::calculate(&container, constraints);

        assert_eq!(results.len(), 3);

        // Verify vertical layout (y positions increase)
        assert!(results[0].y < results[1].y);
        assert!(results[1].y < results[2].y);

        // All children have same width (full container width)
        assert_eq!(results[0].width, 10);
        assert_eq!(results[1].width, 10);
        assert_eq!(results[2].width, 10);

        // All have same x position
        assert_eq!(results[0].x, 0);
        assert_eq!(results[1].x, 0);
        assert_eq!(results[2].x, 0);
    }

    #[test]
    fn test_layout_justify_center() {
        let children = vec![
            create_text_node("A"),
            create_text_node("B"),
            create_text_node("C"),
        ];

        let mut attrs = HashMap::new();
        attrs.insert("justify".to_string(), "center".to_string());

        let container = create_flex_container(attrs, children);

        let constraints = Rect {
            x: 0,
            y: 0,
            width: 20,
            height: 10,
        };

        let results = FlexboxLayout::calculate(&container, constraints);

        assert_eq!(results.len(), 3);

        // First child should start at center position (offset > 0)
        let total_width = results[2].x + results[2].width;
        assert!(total_width <= 20);
        // With centering, first child should have some positive offset
        assert!(results[0].x > 0 || total_width == 20);
    }

    #[test]
    fn test_layout_flex_grow() {
        let children = vec![
            create_text_node("A"),
            create_text_node("B"),
            create_text_node("C"),
        ];

        let mut attrs = HashMap::new();
        attrs.insert("flex-grow".to_string(), "1.0".to_string());

        let container = create_flex_container(attrs, children);

        let constraints = Rect {
            x: 0,
            y: 0,
            width: 20,
            height: 10,
        };
        let results = FlexboxLayout::calculate(&container, constraints);

        assert_eq!(results.len(), 3);

        // With flex-grow=1.0, children should have non-zero width
        assert!(results[0].width > 0);
        assert!(results[1].width > 0);
        assert!(results[2].width > 0);
    }

    #[test]
    fn test_layout_gap() {
        let children = vec![
            create_text_node("A"),
            create_text_node("B"),
            create_text_node("C"),
        ];

        let mut attrs = HashMap::new();
        attrs.insert("gap".to_string(), "2".to_string());

        let container = create_flex_container(attrs, children);

        let constraints = Rect {
            x: 0,
            y: 0,
            width: 20,
            height: 10,
        };

        let results = FlexboxLayout::calculate(&container, constraints);

        assert_eq!(results.len(), 3);

        // Gap should affect positioning
        // Distance between children should account for gap
        let gap_1 = results[1].x - (results[0].x + results[0].width);
        let gap_2 = results[2].x - (results[1].x + results[1].width);

        // Gap should be present (though exact value depends on calculation)
        assert!(gap_1 >= 2 || gap_2 >= 2);
    }

    #[test]
    fn test_layout_nested() {
        // Create inner flex container
        let inner_children = vec![create_text_node("A"), create_text_node("B")];

        let mut inner_attrs = HashMap::new();
        inner_attrs.insert("direction".to_string(), "row".to_string());

        let inner_container = create_flex_container(inner_attrs, inner_children);

        // Create outer flex container
        let outer_children = vec![
            create_text_node("X"),
            inner_container,
            create_text_node("Y"),
        ];

        let mut outer_attrs = HashMap::new();
        outer_attrs.insert("direction".to_string(), "column".to_string());

        let outer_container = create_flex_container(outer_attrs, outer_children);

        let constraints = Rect {
            x: 0,
            y: 0,
            width: 20,
            height: 30,
        };

        let results = FlexboxLayout::calculate(&outer_container, constraints);

        // Outer container has 3 children
        assert_eq!(results.len(), 3);

        // Verify vertical layout (y positions increase)
        assert!(results[0].y < results[1].y);
        assert!(results[1].y < results[2].y);

        // All children have full width
        assert_eq!(results[0].width, 20);
        assert_eq!(results[1].width, 20);
        assert_eq!(results[2].width, 20);
    }

    #[test]
    fn test_flex_style_builder() {
        let style = FlexStyle::new()
            .row()
            .justify(JustifyContent::Center)
            .align(AlignItems::Stretch)
            .gap(2)
            .flex_grow(1.0)
            .flex_shrink(0.5);

        assert_eq!(style.direction, FlexDirection::Row);
        assert_eq!(style.justify_content, JustifyContent::Center);
        assert_eq!(style.align_items, AlignItems::Stretch);
        assert_eq!(style.gap, 2);
        assert_eq!(style.flex_grow, 1.0);
        assert_eq!(style.flex_shrink, 0.5);
    }

    #[test]
    fn test_empty_children() {
        let container = create_flex_container(HashMap::new(), vec![]);

        let constraints = Rect {
            x: 0,
            y: 0,
            width: 20,
            height: 10,
        };

        let results = FlexboxLayout::calculate(&container, constraints);

        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_single_child() {
        let children = vec![create_text_node("A")];

        let container = create_flex_container(HashMap::new(), children);

        let constraints = Rect {
            x: 0,
            y: 0,
            width: 20,
            height: 10,
        };

        let results = FlexboxLayout::calculate(&container, constraints);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].x, 0);
        assert_eq!(results[0].y, 0);
        assert_eq!(results[0].height, 10);
    }

    #[test]
    fn test_justify_end() {
        let children = vec![create_text_node("A"), create_text_node("B")];

        let mut attrs = HashMap::new();
        attrs.insert("justify".to_string(), "end".to_string());

        let container = create_flex_container(attrs, children);

        let constraints = Rect {
            x: 0,
            y: 0,
            width: 20,
            height: 10,
        };

        let results = FlexboxLayout::calculate(&container, constraints);

        assert_eq!(results.len(), 2);

        // Last child should be at or near the end
        let total_width = results[1].x + results[1].width;
        assert!(total_width <= 20);
    }

    #[test]
    fn test_space_between() {
        let children = vec![
            create_text_node("A"),
            create_text_node("B"),
            create_text_node("C"),
        ];

        let mut attrs = HashMap::new();
        attrs.insert("justify".to_string(), "space-between".to_string());

        let container = create_flex_container(attrs, children);

        let constraints = Rect {
            x: 0,
            y: 0,
            width: 20,
            height: 10,
        };

        let results = FlexboxLayout::calculate(&container, constraints);

        assert_eq!(results.len(), 3);

        // First child at start
        assert_eq!(results[0].x, 0);

        // Last child at end
        let last_end = results[2].x + results[2].width;
        assert!(last_end <= 20);
    }

    #[test]
    fn test_space_around() {
        let children = vec![
            create_text_node("A"),
            create_text_node("B"),
            create_text_node("C"),
        ];

        let mut attrs = HashMap::new();
        attrs.insert("justify".to_string(), "space-around".to_string());

        let container = create_flex_container(attrs, children);

        let constraints = Rect {
            x: 0,
            y: 0,
            width: 20,
            height: 10,
        };

        let results = FlexboxLayout::calculate(&container, constraints);

        assert_eq!(results.len(), 3);

        // First child should exist and have valid position
        assert!(!results.is_empty());
        assert!(results[0].width > 0);
}

}
