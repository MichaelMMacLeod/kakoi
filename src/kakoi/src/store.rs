use crate::arena::{Arena, ArenaKey};
use rayon::result;
use slotmap::{new_key_type, SlotMap};
use std::collections::VecDeque;
use std::collections::{HashMap, HashSet};

type Image = image::RgbaImage;

struct Variable {
    name: u64,
}

struct Function {
    /// Each key must be associated with a Variable.
    parameters: Vec<ExprKey>,
    /// Can be any type of expression
    body: ExprKey,
}

struct Application {
    /// Key must be associated with something evaluating to a function
    function: ExprKey,
    /// Can be any type of expression
    arguments: Vec<ExprKey>,
}

type Set = HashSet<ExprKey>;

type Map = HashMap<ExprKey, ExprKey>;

enum ExprNode {
    Fix,
    Function(Function),
    Variable(Variable),
    Application(Application),
    Set(Set),
    Map(Map),
    String(String),
    Image(Image),
}

// struct DB {
//     values: HashMap<u64, String>,
// }

// impl DB {
//     fn extend_with_expr(&mut self, expr: &Expr) {

//     }
// }

new_key_type! {
    pub struct ExprKey;
}

struct Expr {
    slot_map: SlotMap<ExprKey, ExprNode>,
}

impl Expr {
    fn new() -> Self {
        Self {
            slot_map: SlotMap::with_key(),
        }
    }
}

/// uses petgraph's tarjan_scc algorithm to compute the strongly connected
/// components of the arena, starting from a key.
///
/// this should probably be changed to use a custom implementation, so we
/// don't have to build a separate graph.
fn tarjan_scc(arena: &Arena, start: ArenaKey) -> Vec<Vec<ArenaKey>> {
    type Graph = petgraph::Graph<ArenaKey, (), petgraph::Directed, u32>;
    type Node = petgraph::graph::NodeIndex<u32>;

    struct Todo {
        key: ArenaKey,
        node: Node,
    }

    let mut g = Graph::new();

    let start_node = g.add_node(start);
    let mut seen: HashMap<ArenaKey, Node> = vec![(start, start_node)].into_iter().collect();

    let mut todo: VecDeque<Todo> = vec![Todo {
        key: start,
        node: start_node,
    }]
    .into_iter()
    .collect();

    fn maybe_add(
        g: &mut Graph,
        seen: &mut HashMap<ArenaKey, Node>,
        current_node: Node,
        key: ArenaKey,
    ) -> Option<Todo> {
        match seen.get(&key) {
            Some(&k_node) => {
                g.add_edge(current_node, k_node, ());
                None
            }
            None => {
                let k_node = g.add_node(key);
                seen.insert(key, k_node);
                g.add_edge(current_node, k_node, ());
                Some(Todo { key, node: k_node })
            }
        }
    }

    while let Some(Todo { key, node }) = todo.pop_front() {
        use crate::arena::Structure::*;
        match &arena.slot_map.get(key).unwrap().structure {
            Set(s) => {
                todo.extend(
                    s.iter()
                        .copied()
                        .filter_map(|k| maybe_add(&mut g, &mut seen, node, k)),
                );
            }
            Map(m) => {
                todo.extend(
                    m.iter()
                        .map(|(&k, &v)| {
                            vec![
                                maybe_add(&mut g, &mut seen, node, k),
                                maybe_add(&mut g, &mut seen, node, v),
                            ]
                            .into_iter()
                            .filter_map(|v| v)
                        })
                        .flatten(),
                );
            }
            Image(_) => {}
            String(_) => {}
        }
    }

    // dbg!(petgraph::dot::Dot::with_config(&g, &[]));

    petgraph::algo::tarjan_scc(&g)
        .into_iter()
        .map(|v| v.into_iter().map(|n| g[n]).collect())
        .collect()
}

fn compute_expr(arena: &Arena, start_key: ArenaKey) -> (Expr, ExprKey) {
    let mut expr = Expr::new();
    let mut intermediate_exprs: HashMap<ArenaKey, ExprKey> = HashMap::new();

    let mut sccs = tarjan_scc(arena, start_key);
    for component in &sccs {
        compute_component_exprs(arena, component, &mut expr, &mut intermediate_exprs);
    }

    let last_component = sccs.pop().unwrap();
    let result_key = finalize_scc_expr(
        arena,
        start_key,
        last_component,
        &mut expr,
        &intermediate_exprs,
    );
    (expr, result_key)
}

fn compute_component_exprs(
    arena: &Arena,
    component: &Vec<ArenaKey>,
    expr: &mut Expr,
    intermediate_exprs: &mut HashMap<ArenaKey, ExprKey>,
) {
    // for each key 'self_key' in 'component', we need to return a lambda
    // expression.
    //
    // The content of our lambda expression depends on the content of our
    // component. There are three cases:
    //
    // (1) the component has more than one value. Each value must be a
    //     container.
    // (2) the component has exactly one value and that value is a container.
    // (3) the component has exactly one value and that value is not a
    //     container.
    //
    // In case (1), we return something resembling the following:
    //
    // \a1.\b1.\c1.\a2.\b2.\c2(fix \self.\m.\n.SelfDatatype {
    //   u,
    //   v,
    //   w,
    //   (a1 b1 c1),
    //   (b1 c1 a1),
    //   (c1 a1 b1),
    //   (a2 b2 c2),
    //   (b2 c2 a2),
    //   (c2 a2 b2),
    //   (self m n),
    //   (m n self),
    //   (n self m),
    // })
    //
    // In case (2), we return something resembling the following:
    //
    // \a1.\b1.\c1.\a2.\b2.\c2.SelfDatatype {
    //   u,
    //   v,
    //   w,
    //   (a1 b1 c1),
    //   (b1 c1 a1),
    //   (c1 a1 b1),
    //   (a2 b2 c2),
    //   (b2 c2 a2),
    //   (c2 a2 b2),
    // }
    //
    // In case (3), we return the datatype itself whether that be a string or an
    // image.
    //
    // ----------------------------------------------------------------
    //
    // \x.expr is a lambda expression with argument 'x' and body 'expr'
    //
    // (a xs ...) is function application of lambda expression 'a' to arguments
    // 'xs ...'
    //
    // 'fix' is a fixed point combinator, e.g., \f.((\x.f (x x) (\x.f (x x)))),
    // the Y combinator.
    //
    // SelfDatatype refers to a set constructor. This is not yet implemented for
    // maps.
    //
    // a1, b1, c1 are values that 'self_key' contains and are all part of the
    // same component that is a different component from 'component'. That other
    // component has already been processed by this function.
    //
    // a2, b2, c2 are values that 'self_key' contains and are all part of the
    // same component that is a different component from 'component'. That other
    // component is also a different component from the one that a1, b1, and c1
    // are in. The other component has already been processed by this function.
    //
    // Both the a1, b1, c1 and a2, b2, c2 lists need not contain three elements,
    // nor need they contain the same number of elements as each other. There
    // may also be a fourth or fifth or any number of extra lists. There need
    // not be any lists here.
    //
    // u, v, w is a possibly empty list of any number of values each in a
    // different component from 'component'. For each x in [u,v,w], let c be the
    // component associated with x. Then 'self_key' contains no value other than
    // x in component c. This stands in direct contrast to the a1 and a2 lists
    // above, where 'self_key' contained multiple values in the same component.
    //
    // self refers to the expression associated with self_key, i.e., this
    // expression itself.
    //
    // m and n are values in 'component'. In case (1) there will always be at
    // least one of these. The size of this list is exactly equal to the number
    // of values in 'component' minus one (subtracting off self).
    todo!()
}

fn finalize_scc_expr(
    arena: &Arena,
    key: ArenaKey,
    last_component: Vec<ArenaKey>,
    expr: &mut Expr,
    intermediate_exprs: &HashMap<ArenaKey, ExprKey>,
) -> ExprKey {
    if last_component.len() == 1 {
        *intermediate_exprs.get(&key).unwrap()
    } else {
        let mut before_key = true;
        let (before, after): (Vec<_>, Vec<_>) = last_component.into_iter().partition(|v| {
            if *v == key {
                before_key = false;
            }
            before_key
        });
        // key itself is in after, so we need to skip it
        let arguments = after
            .into_iter()
            .skip(1)
            .chain(before)
            .map(|k| *intermediate_exprs.get(&k).unwrap())
            .collect();
        let expr_node = ExprNode::Application(Application {
            function: *intermediate_exprs.get(&key).unwrap(),
            arguments,
        });
        expr.slot_map.insert(expr_node)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn tarjan_0() {
        // a -> "a"
        // ^
        // |
        // v
        // b -> "b"
        // |
        // |
        // v
        // c -> "c"
        let mut arena = Arena::new();
        arena.bind_register_to_empty_set("a");
        arena.bind_register_to_empty_set("b");
        arena.bind_register_to_empty_set("c");
        arena.set_insert_string("a", "a");
        arena.set_insert_string("b", "b");
        arena.set_insert_string("c", "c");
        arena.set_insert("a", "b");
        arena.set_insert("b", "a");
        arena.set_insert("b", "c");
        dbg!(&arena.slot_map);
        let a_key = arena.register("a").unwrap();
        let b_key = arena.register("b").unwrap();
        let c_key = arena.register("c").unwrap();
        let a_str_key = arena.string("a");
        let b_str_key = arena.string("b");
        let c_str_key = arena.string("c");
        let sccs = tarjan_scc(&arena, a_key);
        dbg!(&sccs);
        assert_eq!(sccs.len(), 5);
        assert!(sccs[4].contains(&a_key));
        assert!(sccs[4].contains(&b_key));
        assert!(sccs.iter().position(|v| v.contains(&c_key)).unwrap() < 4);
        assert!(
            sccs.iter().position(|v| v.contains(&c_str_key)).unwrap()
                < sccs.iter().position(|v| v.contains(&c_key)).unwrap()
        );
        assert!(!sccs[4].contains(&a_str_key));
        assert!(!sccs[4].contains(&b_str_key));
    }
}
