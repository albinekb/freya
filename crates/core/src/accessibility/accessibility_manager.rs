use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use accesskit::{Action, DefaultActionVerb, Node, NodeBuilder, Rect, Role, Tree, TreeUpdate};
use freya_engine::prelude::Point;
use freya_node_state::AccessibilityNodeState;
use itertools::Itertools;
use torin::prelude::LayoutNode;

use crate::{accessibility::*, dom::DioxusNode};

pub type SharedAccessibilityManager = Arc<Mutex<AccessibilityManager>>;

pub const ACCESSIBILITY_ROOT_ID: AccessibilityId = AccessibilityId(0);

use std::cmp::Ordering;

fn avg_y(bounds: &Rect) -> f32 {
    ((bounds.y0 + bounds.y1) as f64 / 2.0) as f32
}

fn avg_x(bounds: &Rect) -> f32 {
    ((bounds.x0 + bounds.x1) as f64 / 2.0) as f32
}

fn center_p(bounds: &Rect) -> Point {
    Point::new(avg_x(bounds), avg_y(bounds))
}
fn get_history_score(history: &VecDeque<AccessibilityId>, id: AccessibilityId) -> usize {
    if id == ACCESSIBILITY_ROOT_ID {
        return usize::MAX;
    }
    history.iter().position(|&x| x == id).unwrap_or(usize::MAX)
}

fn same_x(bounds: &Rect, other: &Rect) -> bool {
    // Check if the two rectangles are on the same x-axis
    let x0 = bounds.x0 as i64;
    let x1 = bounds.x1 as i64;
    let other_x0 = other.x0 as i64;
    let other_x1 = other.x1 as i64;

    // Check if the two rectangles are on the same x-axis
    x0.cmp(&other_x0) == Ordering::Equal || x1.cmp(&other_x1) == Ordering::Equal
}

fn same_y(bounds: &Rect, other: &Rect) -> bool {
    // Check if the two rectangles are on the same y-axis
    let y0 = bounds.y0 ;
    let y1 = bounds.y1 ;
    let other_y0 = other.y0 ;
    let other_y1 = other.y1 ;

    // Check if the two rectangles are on the same y-axis
    return y0 == other_y0 || y1 == other_y1;
}

fn check_within_bounds(
    bounds: &Rect,
    current_bounds: &Rect,
    direction: &AccessibilityFocusDirection,
) -> bool {
    match direction {
        AccessibilityFocusDirection::Up => same_x(bounds, current_bounds),
        AccessibilityFocusDirection::Down => same_x(bounds, current_bounds),
        AccessibilityFocusDirection::Left => same_y(bounds, current_bounds),
        AccessibilityFocusDirection::Right => same_y(bounds, current_bounds),
        _ => false,
    }
}

fn calculate_wrapped_distance(bounds_a: &Rect, bounds_b: &Rect) -> f64 {
    let screen_height = 720.0;
    let screen_width = 720.0;
    let center_a_x = (bounds_a.x0 + bounds_a.x1) / 2.0;
    let center_a_y = (bounds_a.y0 + bounds_a.y1) / 2.0;
    let center_b_x = (bounds_b.x0 + bounds_b.x1) / 2.0;
    let center_b_y = (bounds_b.y0 + bounds_b.y1) / 2.0;

    // Calculate horizontal distance
    let dx = (center_a_x - center_b_x).abs();
    let wrapped_dx = f64::min(dx, screen_width - dx);

    // Calculate vertical distance
    let dy = (center_a_y - center_b_y).abs();
    let wrapped_dy = f64::min(dy, screen_height - dy);

    // Calculate Euclidean distance using the wrapped distances
    (wrapped_dx.powi(2) + wrapped_dy.powi(2)).sqrt()
}

fn check_is_immediate(
    bounds: &Rect,
    current_bounds: &Rect,
    direction: &AccessibilityFocusDirection,
) -> bool {
    match direction {
        AccessibilityFocusDirection::Up => {
            let center_y = (bounds.y0 + bounds.y1) / 2.0;
            let current_center_y = (current_bounds.y0 + current_bounds.y1) / 2.0;
            center_y < current_center_y
        }
        AccessibilityFocusDirection::Down => {
            let center_y = (bounds.y0 + bounds.y1) / 2.0;
            let current_center_y = (current_bounds.y0 + current_bounds.y1) / 2.0;
            center_y > current_center_y
        }
        AccessibilityFocusDirection::Left => {
            if !same_y(bounds, current_bounds) {
                return false;
            }
            let center_x = (bounds.x0 + bounds.x1) / 2.0;
            let current_center_x = (current_bounds.x0 + current_bounds.x1) / 2.0;

            center_x < current_center_x
        }
        AccessibilityFocusDirection::Right => {
            if !same_y(bounds, current_bounds) {
                return false;
            }
            let center_x = (bounds.x0 + bounds.x1) / 2.0;
            let current_center_x = (current_bounds.x0 + current_bounds.x1) / 2.0;
            center_x > current_center_x
        }
        _ => false,
    }
}

fn find_node_in_direction(
    current_bounds: Rect,
    nodes: &[(AccessibilityId, Node)],
    focused_id: AccessibilityId,
    direction: &AccessibilityFocusDirection,
    history: &VecDeque<AccessibilityId>,
) -> Option<usize> {
    let current_center_x = (current_bounds.x0 + current_bounds.x1) / 2.0;
    let current_center_y = (current_bounds.y0 + current_bounds.y1) / 2.0;
    println!("Curently focused node: {:?}", focused_id);

    let (immediate_nodes, wrap_around_nodes): (Vec<_>, Vec<_>) = nodes
        .iter()
        .enumerate()
        .filter_map(|(index, (id, node))| {
            if *id == focused_id {
                return None;
            }

            let is_modal = node.is_modal();
            if is_modal {
                println!("------");
                println!("Node: {:?}, Is modal: {:?}", id, is_modal);
                println!("------");
                
            }

            if let Some(bounds) = node.bounds() {
                let node_center_x = (bounds.x0 + bounds.x1) / 2.0;
                let node_center_y = (bounds.y0 + bounds.y1) / 2.0;

                let (
                    distance_primary,
                    distance_secondary,
                    is_within_bounds,
                    history_score,
                    is_immediate,
                ) = match direction {
                    AccessibilityFocusDirection::Up => (
                        (bounds.y0 - current_bounds.y1) as i64,
                        (current_bounds.y1 - bounds.y0) as i64,
                        check_within_bounds(&bounds, &current_bounds, direction),
                        get_history_score(history, *id),
                        check_is_immediate(&bounds, &current_bounds, direction),
                    ),
                    AccessibilityFocusDirection::Down => (
                        (current_bounds.y0 - bounds.y1) as i64,
                        (bounds.y1 - current_bounds.y0) as i64,
                        check_within_bounds(&bounds, &current_bounds, direction),
                        get_history_score(history, *id),
                        check_is_immediate(&bounds, &current_bounds, direction),
                    ),
                    AccessibilityFocusDirection::Left => (
                        // calculate_wrapped_distance(&current_bounds, &bounds) as i64,
                        (bounds.x0 - current_bounds.x1) as i64,
                        (current_bounds.x1 - bounds.x0) as i64,
                        check_within_bounds(&bounds, &current_bounds, direction),
                        get_history_score(history, *id),
                        check_is_immediate(&bounds, &current_bounds, direction),
                    ),
                    AccessibilityFocusDirection::Right => (
                        // calculate_wrapped_distance(&current_bounds, &bounds) as i64,
                        (current_bounds.x0 - bounds.x1) as i64,
                        (bounds.x1 - current_bounds.x0) as i64,
                        check_within_bounds(&bounds, &current_bounds, direction),
                        get_history_score(history, *id),
                        check_is_immediate(&bounds, &current_bounds, direction),
                    ),
                    _ => return None, // Handle other directions if needed
                };

                Some((
                    index,
                    id,
                    distance_primary,
                    distance_secondary,
                    is_within_bounds,
                    history_score,
                    is_immediate,
                ))
            } else {
                None
            }
        })
        .partition(|&(_, _, _, _, is_within_bounds, _, is_immediate)| {
            is_immediate
        });

    let nodes_to_consider = if !immediate_nodes.is_empty() {
        &immediate_nodes
    } else {
        &wrap_around_nodes
    };

    nodes_to_consider
        .iter()
        .inspect(
            |(
                index,
                id,
                distance_primary,
                distance_secondary,
                is_within_bounds,
                history_score,
                _,
            )| {
                println!(
                    "Node: {:?}, Distance: {}, Is within bounds: {}, History: {}",
                    id, distance_primary, is_within_bounds, history_score
                );
            },
        )
        .min_by(
            |&(_, _, dist_a_primary, dist_a_secondary, is_a_within, history_a, is_a_immediate),
             &(_, _, dist_b_primary, dist_b_secondary, is_b_within, history_b, is_b_immediate)| {
        
                dist_a_secondary.cmp(&dist_b_secondary)
                    .then_with(|| dist_a_primary.cmp(&dist_b_primary))
                    .then_with(|| is_a_within.cmp(&is_b_within)) // Reverse to prioritize true over false
                    // .then_with(|| history_a.cmp(&history_b))
                    .then_with(|| is_a_immediate.cmp(&is_b_immediate)) // Reverse to prioritize true over false
            },
        )
        .map(|&(index, _, _, _, _, _, _)| index)
}
pub struct AccessibilityManager {
    /// Accessibility Nodes
    pub nodes: Vec<(AccessibilityId, Node)>,
    /// Current focused Accessibility Node.
    pub focused_id: AccessibilityId,
    history: VecDeque<AccessibilityId>,
}
const HISTORY_SIZE: usize = 3;
impl AccessibilityManager {
    pub fn new(focused_id: AccessibilityId) -> Self {
        Self {
            focused_id,
            nodes: Vec::default(),
            history: VecDeque::with_capacity(HISTORY_SIZE),
        }
    }

    /// Wrap it in a `Arc<Mutex<T>>`.
    pub fn wrap(self) -> SharedAccessibilityManager {
        Arc::new(Mutex::new(self))
    }

    /// Clear the Accessibility Nodes.
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.history.clear();
    }

    pub fn push_node(&mut self, id: AccessibilityId, node: Node) {
        self.nodes.push((id, node))
    }

    /// Add a Node to the Accessibility Tree.
    pub fn add_node(
        &mut self,
        dioxus_node: &DioxusNode,
        layout_node: &LayoutNode,
        accessibility_id: AccessibilityId,
        node_accessibility: &AccessibilityNodeState,
    ) {
        let mut builder = NodeBuilder::new(Role::Unknown);

        // Set children
        let children = dioxus_node.get_accessibility_children();
        if !children.is_empty() {
            builder.set_children(children);
        }

        // Set text value
        if let Some(alt) = &node_accessibility.alt {
            builder.set_value(alt.to_owned());
        } else if let Some(value) = dioxus_node.get_inner_texts() {
            builder.set_value(value);
            builder.set_role(Role::Label);
        }

        // Set name
        if let Some(name) = &node_accessibility.name {
            builder.set_name(name.to_owned());
        }

        // Set role
        if let Some(role) = node_accessibility.role {
            builder.set_role(role);
            if role == Role::Dialog  {
                builder.set_modal();
            }
        }

        

        // Set the area
        let area = layout_node.area.to_f64();
        builder.set_bounds(Rect {
            x0: area.min_x(),
            x1: area.max_x(),
            y0: area.min_y(),
            y1: area.max_y(),
        });

            // Set focusable action
         if node_accessibility.focusable  || node_accessibility.role == Some(Role::Button) {
            
            builder.add_action(Action::Focus);
            
         } else {
            builder.add_action(Action::Default);
            builder.set_default_action_verb(DefaultActionVerb::Focus);
        }

   

        // Insert the node into     the Tree
        let node = builder.build();
        self.push_node(accessibility_id, node);
    }

    /// Update the focused Node ID and generate a TreeUpdate if necessary.
    pub fn set_focus_with_update(&mut self, new_focus_id: AccessibilityId) -> Option<TreeUpdate> {
        self.focused_id = new_focus_id;

        // Only focus the element if it exists
        let node_focused_exists = new_focus_id == ACCESSIBILITY_ROOT_ID
            || self.nodes.iter().any(|node| node.0 == new_focus_id);
        if node_focused_exists {
            self.history_push(new_focus_id);

            Some(TreeUpdate {
                nodes: Vec::new(),
                tree: None,
                focus: self.focused_id,
            })
        } else {
            self.history_rm(new_focus_id);
            None
        }
    }

    fn find_next_prev_node(
        &self,
        node_index: Option<usize>,
        direction: AccessibilityFocusDirection,
    ) -> Option<&(accesskit::NodeId, accesskit::Node)> {
        match direction {
            AccessibilityFocusDirection::Forward
            | AccessibilityFocusDirection::Down
            | AccessibilityFocusDirection::Right => {
                // Find the next Node
                if let Some(node_index) = node_index {
                    if node_index == self.nodes.len() - 1 {
                        self.nodes.first()
                    } else {
                        self.nodes.get(node_index + 1)
                      
                        
                    }
                } else {
                    self.nodes.first()
                }
                // if let Some(node_index) = node_index {
                //     if node_index == self.nodes.len() - 1 {
                //         self.nodes.first()
                //     } else {
                //         let found = self.nodes.iter().skip(node_index +1).find( |((id, node))| {
                //             node.supports_action(Action::Focus)
                //         });
                        
                //         if found.is_some() {
                //             found
                            
                //         } else {
                //             self.nodes.first()
                //         }
                      
                        
                //     }
                // } else {
                //     self.nodes.first()
                // }
            }
            AccessibilityFocusDirection::Backward
            | AccessibilityFocusDirection::Up
            | AccessibilityFocusDirection::Left => {
                // Find the previous Node
                if let Some(node_index) = node_index {
                    if node_index == 0 {
                        self.nodes.last()
                    } else {
                        self.nodes.get(node_index - 1)
                    }
                } else {
                    self.nodes.last()
                }
            }
        }
    }

    fn history_rm(&mut self, id: AccessibilityId) {
        if self.history.contains(&id) {
            self.history.retain(|&x| x != id);
        }
    }

    fn history_push(&mut self, id: AccessibilityId) {
        self.history_rm(id);

        if self.history.len() > HISTORY_SIZE {
            self.history.pop_back();
        }
        self.history.push_front(id);
    }

    /// Create the root Accessibility Node.
    pub fn build_root(&mut self, root_name: &str) -> Node {
        let mut builder = NodeBuilder::new(Role::Window);
        builder.set_name(root_name.to_string());
        builder.set_children(
            self.nodes
                .iter()
                .map(|(id, _)| *id)
                .collect::<Vec<AccessibilityId>>(),
        );

        builder.build()
    }

    /// Process the Nodes accessibility Tree
    pub fn process(&mut self, root_id: AccessibilityId, root_name: &str) -> TreeUpdate {
        let root = self.build_root(root_name);
        let mut nodes = vec![(root_id, root)];
        nodes.extend(self.nodes.clone());
        nodes.reverse();

        let focus = self
            .nodes
            .iter()
            .find_map(|node| {
                if node.0 == self.focused_id {
                    Some(node.0)
                } else {
                    None
                }
            })
            .unwrap_or(ACCESSIBILITY_ROOT_ID);

        TreeUpdate {
            nodes,
            tree: Some(Tree::new(root_id)),
            focus,
        }
    }

    /// Focus the next/previous Node starting from the currently focused Node.
    pub fn set_focus_on_next_node(&mut self, direction: AccessibilityFocusDirection) -> TreeUpdate {
        let node_index = self
            .nodes
            .iter()
            .enumerate()
            .find(|(_, node)| node.0 == self.focused_id)
            .map(|(i, _)| i);

        let target_node = match direction {
            AccessibilityFocusDirection::Forward => {
                self.find_next_prev_node(node_index, direction)
                // // Find the next Node
                // if let Some(node_index) = node_index {
                //     if node_index == self.nodes.len() - 1 {
                //         self.nodes.first()
                //     } else {
                //         self.nodes.get(node_index + 1)
                //     }
                // } else {
                //     self.nodes.first()
                // }
            }
            AccessibilityFocusDirection::Backward => {
                self.find_next_prev_node(node_index, direction)
                // // Find the previous Node
                // if let Some(node_index) = node_index {
                //     if node_index == 0 {
                //         self.nodes.last()
                //     } else {
                //         self.nodes.get(node_index - 1)
                //     }
                // } else {
                //     self.nodes.last()
                // }
            }
            AccessibilityFocusDirection::Up
            | AccessibilityFocusDirection::Down
            | AccessibilityFocusDirection::Left
            | AccessibilityFocusDirection::Right => {
                if let Some(node_index) = node_index {
                    let target_node: Option<&(AccessibilityId, Node)> =
                        if let Some((_, current_node)) = self.nodes.get(node_index) {
                            if let Some(current_bounds) = current_node.bounds() {
                                let found = find_node_in_direction(
                                    current_bounds,
                                    &self.nodes,
                                    self.focused_id,
                                    &direction,
                                    &self.history,
                                );

                                if let Some(index) = found {
                                    self.nodes.get(index)
                                } else {
                                    println!("No node found in direction");
                                    self.find_next_prev_node(Some(node_index), direction)
                                }
                            } else {
                                println!("Current node has no bounds");
                                self.nodes.first()
                            }
                        } else {
                            println!("Current node not found");
                            self.nodes.first()
                        };

                    target_node
                } else {
                    println!("Current node not found");
                    self.find_next_prev_node(node_index, direction)
                }
            }
        };
        

        self.focused_id = target_node
            .map(|(id, _)| *id)
            .unwrap_or(ACCESSIBILITY_ROOT_ID);

        println!("Focused node: {:?}", self.focused_id);
        self.history_push(self.focused_id);

        TreeUpdate {
            nodes: Vec::new(),
            tree: None,
            focus: self.focused_id,
        }
    }
}
