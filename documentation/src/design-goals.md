# kakoi - design goals #

## [allow the presentation of any kind of data](any-data.html) ##

## [appreciate the distinction between data and its interpretation](data-versus-interpretation.html) ##

## impose structure by surrounding alike things ##

[kakoi](kakoi.html) allows the grouping of alike things. A single interpretation
of a piece of data may be placed in any number of---possibly
overlapping---groups. Groups themselves are treated as pieces of information
that can be placed in further groups.

## display groups as nested circles ##

[kakoi](kakoi.html) presents a group visually as a set of circles. Each of these
circles may be a group itself (in which it would contain further circles) or a
fundamental interpretation of data, like an image, piece of text, or video. The
circles must be zoomable, to allow for deeply-nested groups to be examined.

## preserve the meaning of existing groups ##

The purpose behind grouping certain pieces of information is likely to differ
among individuals. [kakoi](kakoi.html) ensures that---once a group is
formed---it cannot be changed: no new information can be added to it, removed
from it, or changed inside of it.

## build upon existing groups efficiently ##

[kakoi](kakoi.html) must allow the creation of a group that is like a different
group, except changed in some way---perhaps by the addition or removal of a
piece of information. The process of creating new groups must simple and
efficient.

## enable the creation of recursive groups ##

Some concepts seem necessarily circular. For instance, the idea of naming a
concept can itself be named (by "naming"). For this reason, it must be possible
to create a group that references itself.

## facilitate sharing of groups over a network connection ##

[kakoi](kakoi.html) facilitates the sharing of groups between computers.

## [support queries](support-queries.html) ##

## enable introspection of fundamental data types ##

[kakoi](kakoi.html) must be able to traverse the structure of the data it
stores. This is necessary to, for instance, [support queries](#support-queries)
of arbitrary types of data (not just text).

## enable the creation of new fundamental data types ##

[kakoi](kakoi.html) should be extendable, allowing users to use a programming
language to define new ways of interpreting data. These interpretations must be
able to control the way they are presented and interacted with.
