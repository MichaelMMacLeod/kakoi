use petgraph::graph::NodeIndex;

// Maintains a stack of history elements. Each element represents a circle 
// that we have zoomed in on so that it fills the screen.
pub struct History {
    // The stack. This must always have at least one element inside it.
    elements: Vec<Element>,
}

pub struct Element {
    pub flat_graph_index: NodeIndex<u32>,
    pub indication_tree_index: NodeIndex<u32>,
}

impl History {
    pub fn new(first_element: Element) -> Self {
        Self {
            elements: vec![first_element],
        }
    }

    pub fn elements(&self) -> &Vec<Element> {
        &self.elements
    }

    pub fn top(&self) -> &Element {
        // We always have at least one element on the top of the stack, 
        // so the .unwrap() here is safe.
        self.elements.last().unwrap()
    }

    pub fn push(&mut self, element: Element) {
        self.elements.push(element);
    }

    pub fn pop(&mut self) -> Option<Element> {
        // We need to ensure that there is always at least one element
        // in the stack.
        if self.elements.len() > 1 {
            self.elements.pop()
        } else {
            None
        }
    }
}