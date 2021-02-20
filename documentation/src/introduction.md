% kakoi - introduction

[Kakoi](kakoi.html) is a computer program that enables its users
to create, query, and share *accurate representations* of their thoughts. 

We achieve this with a data structure that represents knowledge as groups of
similar objects. These 'objects' include---but are not limited to---text,
images, videos, and groups themselves. Kakoi places no limitations on the ways
in which an object can be grouped; any single object can be placed in as many or
as few groups as necessary. Groups may even recursively include themselves.

Kakoi is useful for keeping track of ideas in your head in a similar fashion to
a notebook or journal. The minimal structure (grouping) that is enforced upon
knowledge-representations lets us easily inspect both well-understood and new,
ambiguous, and transient ideas.

The name "kakoi" is a term associated with
*[surrounding](https://senseis.xmp.net/?Kakoi)* in the game of
[Go](https://en.wikipedia.org/wiki/Go_(game)). Surrounding alike objects is the
main action users of this system take.

### Accurate representation of knowledge ###

By *representation of knowledge* we mean anything outside of an observer's head
that can be interpreted as having meaning. For example, this paragraph will
likely be interpreted by the reader as having some amount of meaning (whatever
that meaning might be).

*Accuracy* is a quality that may or may not appear when multiple observers
attempt to interpret a single representation of knowledge. We say that a
representation of knowledge is *accurate* when observers come upon compatible
ideas about what the representation means.

The word "train" taken out of context would not be considered accurate, because
observers might interpret that word as indicating the idea of training, say, for
a marathon or sports game, or train as in locomotive. These interpretations are
not compatible.

### Representing knowledge as groups of alike objects ###

[kakoi](kakoi.html) takes much of its inspiration from Bongard problems:

> The idea of a Bongard problem is to present two sets of relatively simple
> diagrams, say A and B. All the diagrams from set A have a common factor or
> attribute, which is lacking in all the diagrams of set B. The problem is to
> find, or to formulate, convincingly, the common factor
> 
> -- [Wikipedia - Bongard problem](https://en.wikipedia.org/wiki/Bongard_problem)

Here is a Bongard problem:

![](images/Bongard_problem_convex_polygons.svg)
Image created by Wikipedia
[User:Cmglee](https://commons.wikimedia.org/wiki/User:Cmglee), licensed under
[CC-BY-SA](https://creativecommons.org/licenses/by-sa/4.0/).

We see two sides: a left side and a right side, each containing shapes. The
puzzle creator asserts that these shapes deserved to be grouped in this
way---it's our job to determine why. [Answer, for the
rushed](bongard-problem-answer.html).

There is an [online list of Bongard
problems](https://www.foundalis.com/res/bps/bpidx.htm). Interestingly enough,
there is no answer key to be found for people to check their solutions. There is
likely a reason for this: it's simply not necessary. During the process of
solving a Bongard problem we come up with many ideas that seem to almost fit,
but are just not quite right. Eventually, when we find a solution, it is quite
clear that it is correct---it just *feels right*. In this sense, we can
confidently say that properly-constructed Bongard problems are *accurate
representations of knowledge*.

Kakoi asks: what if we took this structure---grouping alike things---and ran
with it, limiting ourselves not just to geometric and mathematical concepts, but
to all possible ideas? Moreover, what if we removed the puzzle by pairing each
"problem" with a description of its solution?

We are left with a system that can be used to create and communicate accurate
representations of knowledge.
