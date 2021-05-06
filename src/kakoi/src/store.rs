use crate::arena::{Arena, ArenaKey};
use rayon::result;
use slotmap::{new_key_type, SlotMap};
use std::collections::VecDeque;
use std::collections::{HashMap, HashSet};

type Image = image::RgbaImage;

struct Variable {
    name: usize,
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
fn tarjan_scc(arena: &Arena, start: ArenaKey) -> (Vec<Vec<ArenaKey>>, HashMap<ArenaKey, usize>) {
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

    let sccs: Vec<Vec<_>> = petgraph::algo::tarjan_scc(&g)
        .into_iter()
        .map(|v| v.into_iter().map(|n| g[n]).collect())
        .collect();

    let sccs_lookup: HashMap<ArenaKey, usize> = sccs
        .iter()
        .enumerate()
        .map(|(i, c)| c.iter().map(move |&v| (v, i)))
        .flatten()
        .collect();

    (sccs, sccs_lookup)
}

fn compute_expr(arena: &Arena, start_key: ArenaKey) -> (Expr, ExprKey) {
    let mut expr = Expr::new();
    let mut intermediate_exprs: HashMap<ArenaKey, ExprKey> = HashMap::new();

    let (mut sccs, sccs_lookup) = tarjan_scc(arena, start_key);
    for current_component in 0..sccs.len() {
        compute_component_exprs(
            arena,
            &sccs,
            &sccs_lookup,
            current_component,
            &mut expr,
            &mut intermediate_exprs,
        )
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

fn compute_for_complicated_component_set(
    arena: &Arena,
    components: &Vec<Vec<ArenaKey>>,
    components_lookup: &HashMap<ArenaKey, usize>,
    current_component: usize,
    expr: &mut Expr,
    intermediate_exprs: &mut HashMap<ArenaKey, ExprKey>,
    indications: HashMap<usize, Vec<ArenaKey>>,
) -> ExprKey {
    todo!()
}

fn compute_for_simple_comonent_set(
    arena: &Arena,
    components: &Vec<Vec<ArenaKey>>,
    components_lookup: &HashMap<ArenaKey, usize>,
    current_component: usize,
    expr: &mut Expr,
    intermediate_exprs: &mut HashMap<ArenaKey, ExprKey>,
    indications: HashMap<usize, Vec<ArenaKey>>,
) -> ExprKey {
    let mut set: HashSet<ExprKey> = HashSet::new();

    // components = [[k1,k2,k3],[k4,k5,k6],[k7,k8,k9],[k10]]
    // components_lookup = (k1,k2,k3)->0,(k4,k5,k6)->1,(k7,k8,k9)->2,(k10)->3
    // indications = 0->[k1,k2,k3],2->[k7,k9],3->[k10]]

    // vars = k1->0,k2->1,k3->2,k7->3,k8->4,k9->5,k10->6
    // positions = k1->0,k2->1,k3->2,k7->0,k8->1,k9->2,k10->0

    // args = k1->[2,3],k2->[3,1],k3->[1,2],k7->[5,6],k9->[4,5],k10->[]

    // set        = {
    //                  (0 2 3), (2 3 1), (3 1 2),
    //                  (4 5 6), (6 4 5),
    //                  7
    //              }

    // func = 0.1.2.3.4.5.6.set

    // app = (func k1 k2 k3 k7 k8 k9 k10)

    // binding, name
    let vars: HashMap<ArenaKey, usize> = components
        .iter()
        .filter(|component| {
            component
                .iter()
                .any(|k| indications.get(components_lookup.get(k).unwrap()).is_some())
        })
        .flatten()
        .copied()
        .zip(0..)
        .collect();

    let positions: HashMap<ArenaKey, usize> = components
        .iter()
        .map(|component| {
            component
                .iter()
                .enumerate()
                .map(|(position, key)| (*key, position))
        })
        .flatten()
        .collect();

    let applications: Vec<(usize, Vec<usize>)> = indications
        .iter()
        .map(|(component_index, elements)| {
            let component = components.get(*component_index).unwrap();
            let component_length = component.len();

            elements.iter().map(|element| {
                let position_in_component = *positions.get(element).unwrap();
                let var_name = vars.get(element).unwrap();

                let mut args = Vec::with_capacity(component_length - 1);
                let mut current_position = position_in_component;
                loop {
                    current_position = (current_position + 1) % component_length;
                    if current_position == position_in_component {
                        break;
                    }
                    args.push(current_position + var_name);
                }
            });

            todo!()
        })
        .collect();

    // <fn name, arg name ...> ...
    // let applications: Vec<(usize, Vec<usize>)> = vars
    //     .iter()
    //     .map(|(binding, &var_name)| {
    //         let position = *positions.get(binding).unwrap();
    //         let self_component = components
    //             .get(*components_lookup.get(binding).unwrap())
    //             .unwrap();
    //         let self_component_len = self_component.len();

    //         let mut args = Vec::with_capacity(self_component_len - 1);
    //         let mut n = position;
    //         loop {
    //             n = (n + 1) % self_component_len;
    //             if n == var_name {
    //                 break;
    //             }
    //             args.push(var_name + n);
    //         }

    //         (var_name, args)
    //     })
    //     .collect();

    // let set: HashSet<ExprKey> = applications.into_iter().map(|(func, args)| {

    // }).collect();

    // // <component c, <elt in c, variable name for elt>>
    // let parameters: Vec<(usize, Vec<(ArenaKey, usize)>)> = {
    //     let mut var = 0;
    //     indications
    //         .iter()
    //         .map(|(component, elements)| {
    //             let result = (
    //                 *component,
    //                 elements.iter().zip(var..).map(|(a, b)| (*a, b)).collect(),
    //             );

    //             var += elements.len();

    //             result
    //         })
    //         .collect()
    // };

    // // // <variable name, variable name arguments>
    // // let parameters_arguments: HashMap<usize, Vec<usize>>;

    // for (component, elements) in indications {}

    todo!()
}

fn compute_for_set(
    arena: &Arena,
    components: &Vec<Vec<ArenaKey>>,
    components_lookup: &HashMap<ArenaKey, usize>,
    current_component: usize,
    expr: &mut Expr,
    intermediate_exprs: &mut HashMap<ArenaKey, ExprKey>,
    set: &HashSet<ArenaKey>,
) -> ExprKey {
    let mut indications: HashMap<usize, Vec<ArenaKey>> = HashMap::new();

    for k in set {
        let k_component = *components_lookup.get(k).unwrap();
        indications
            .entry(k_component)
            .or_insert_with(|| Vec::new())
            .push(*k);
    }

    let current_component_indications = indications.get(&current_component);

    match current_component_indications {
        Some(c) => compute_for_complicated_component_set(
            arena,
            components,
            components_lookup,
            current_component,
            expr,
            intermediate_exprs,
            indications,
        ),
        None => compute_for_simple_comonent_set(
            arena,
            components,
            components_lookup,
            current_component,
            expr,
            intermediate_exprs,
            indications,
        ),
    }
}

fn compute_component_exprs(
    arena: &Arena,
    components: &Vec<Vec<ArenaKey>>,
    components_lookup: &HashMap<ArenaKey, usize>,
    current_component: usize,
    expr: &mut Expr,
    intermediate_exprs: &mut HashMap<ArenaKey, ExprKey>,
) {
    // for each key 'self_key' in 'component', we need to return a lambda
    // expression.
    //
    // The expression we produce should take 'n' arguments, and produce a set.
    // The set that is produced by the expression is taken to represent the set
    // bound to 'self_key' in the arena. Each argument to the lambda expression
    // is another lambda expression that, when applied to the proper arguments,
    // will produce a set in the same component as the one represented by the
    // expression we are producing.
    //
    // The content of our lambda expression depends on the content of our
    // component. There are two cases:
    //
    // (1) The component has at least one value that either directly or
    //     indirectly contains itself.
    // (2) The component has no values that either directly or indirectly
    //     contain themselves. Then the component must have exactly one value.
    //     Moreover, if that value is a set, then the set must only contain
    //     values from components different from its own component.
    //
    // In case (1), we return something resembling the following:
    //
    // (\a1.\b1.\c1.\a2.\b2.\c2.(fix \self.\m.\n.Set {
    //   (a1 b1 c1), (b1 c1 a1), (c1 a1 b1),
    //   (a2 b2 c2),
    //   (self m n), (m n self), (n self m)
    //  })
    //  lookup(a1) lookup(b1) lookup(c1)
    //  lookup(a2) lookup(b2) lookup(c2))
    //
    // In case (2), we return something resembling the following instead:
    //
    // (\a1.\b1.\c1.\a2.\b2.\c2.Set {
    //   (a1 b1 c1), (b1 c1 a1), (c1 a1 b1),
    //   (a2 b2 c2),
    //  lookup(a1) lookup(b1) lookup(c1)
    //  lookup(a2) lookup(b2) lookup(c2))
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
    // Set { ... } refers to a set constructor. We should also be able to
    // construct expressions from maps, but this is not yet implemented.
    //
    // lookup(p) accesses the previously-processed expression associated with
    // 'p'.
    //
    // a1, b1, c1 are values that 'self_key' contains and are all part of the
    // same component that is a different component from 'component'. That other
    // component has already been processed by this function.
    //
    // a2 is a value that 'self_key' contains. b2 and c2 are values that are all
    // part of the same component of a2, which is a different component from
    // 'component'. That other component is also a different component from the
    // one that a1, b1, and c1 are in. The other component has already been
    // processed by this function. a2 is the only value from that component that
    // 'self_key' contains.
    //
    // Both the a1, b1, c1 and a2, b2, c2 lists need not contain three elements,
    // nor need they contain the same number of elements as each other. There
    // may also be a fourth or fifth or any number of extra lists. There need
    // not be any lists here.
    //
    // The Set { ... } need only contain elements that 'self_key' actually
    // contains. To give a more complete overview of what is possible, these
    // examples assume that the set contains a lot of things. This is unlikely
    // to happen. For instance, the set in example (1) includes elements a1, b1,
    // and c1. This is not necessary. It can just include one of those, or two,
    // but does not need all of them. Since they are all part of the same
    // component though, they all need to be passed in as arguments, even if
    // some are not directly included in the set.
    //
    // self refers to the expression associated with self_key, i.e., this
    // expression itself.
    //
    // m and n are values in 'component'. In case (1) there will always be at
    // least one of these. The size of this list is exactly equal to the number
    // of values in 'component' minus one (subtracting off self).
    //
    // The set only contains (self x y ...) if the set directly contains itself.

    let res: Vec<(ArenaKey, ExprKey)> = components
        .get(current_component)
        .unwrap()
        .iter()
        .map(|&value| {
            use crate::arena::Structure::*;
            match &arena.slot_map.get(value).unwrap().structure {
                Image(i) => {
                    assert!(components.len() == 1);
                    (value, expr.slot_map.insert(ExprNode::Image((**i).clone())))
                }
                String(s) => {
                    assert!(components.len() == 1);
                    (value, expr.slot_map.insert(ExprNode::String((**s).clone())))
                }
                Set(s) => (
                    value,
                    compute_for_set(
                        arena,
                        components,
                        components_lookup,
                        current_component,
                        expr,
                        intermediate_exprs,
                        (*s).as_ref(),
                    ),
                ),
                Map(m) => todo!(),
            }
            // todo!()
        })
        .collect();

    res.into_iter().for_each(|(k, v)| {
        intermediate_exprs.insert(k, v);
    });
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
        // dbg!(&arena.slot_map);
        let a_key = arena.register("a").unwrap();
        let b_key = arena.register("b").unwrap();
        let c_key = arena.register("c").unwrap();
        let a_str_key = arena.string("a");
        let b_str_key = arena.string("b");
        let c_str_key = arena.string("c");
        let (sccs, sccs_lookup) = tarjan_scc(&arena, a_key);
        // dbg!(&sccs);
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
        for (k, v) in sccs_lookup {
            assert!(sccs[v].contains(&k));
        }
    }
}
