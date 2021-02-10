# kakoi - design goal: support queries #

[kakoi](kakoi.html) must allow its users to use search functionality to locate
specific groups.

Some types of queries:

- Given a circle `C` and a positive integer `n`, return the circles nested
  inside of `C` until a nesting depth of `n` is reached.
  
  This query will happen each time a new circle needs to be drawn on screen.
- Given a circle `C` and a piece of text `T`, locate all circles within `C` that
  contain the text `T`
  
  This type of query is useful for locating circles to be used in further
  queries.
- Given a circle `C` and a set of circles `S` that each reside inside `C`, find
  the smallest circles inside `C` that encloses every circle in `S`.
  
  This query finds abstract connections between a set of concrete things.
- Given a set of circles `S`, determine the set of largest circles that are
  contained in every circle of `S`.

  This query finds concrete connections between a set of abstract things.
- Given a circle `C` and some custom logical structure of circles `L`, return
  every circle within `C` that conforms to `L`
  
  This query enables custom queries.

Non-exhaustive list of unknowns:

- How can declarative queries be implemented efficiently?
  
  A declarative language for writing queries will probably be useful. For
  instance, we should be able to say "find the circles that satisfy this
  predicate" instead of "use breadth-first-search starting from x, stopping when
  ..." and so on.

  What Neo4j has to say about how Cypher solves this:

  > Cypher is declarative, and so usually the query itself does not specify the
  > algorithm to use to perform the search. Neo4j will automatically work out the
  > best approach to finding start nodes and matching patterns. Predicates in
  > WHERE parts can be evaluated before pattern matching, during pattern matching,
  > or after finding matches. However, there are cases where you can influence the
  > decisions taken by the query compiler. Read more about indexes in Indexes for
  > search performance, and more about specifying hints to force Neo4j to solve a
  > query in a specific way in Planner hints and the USING keyword.
  >
  > -- [Neo4j documentation - MATCH](https://neo4j.com/docs/cypher-manual/current/clauses/match/#match-introduction)
- How can custom data types be queried?

  New data types should expose some kind of query functionality. For instance, a
  data type representing an array of integers should allow for query by
  numerical value. Certain arrays aught to support further queries. For
  instance, a sorted array should support binary search.

  How can all of this functionality be exposed in a nice way?
