% kakoi - design goal: allow the presentation of any kind of data

[kakoi](kakoi.html) is like a notebook or journal in that it should allow users
to express just about any idea they please. Such expressions need not be limited
to text. Indeed, sometimes an idea is best expressed through a picture or a
video or a sound clip or something else. For this reason, [kakoi](kakoi.html)
should be able to display anything that can be represented on a computer
including, but not limited to: text, video, audio, images, numbers, boolean
values, lists, trees, graphs, and tables. [kakoi](kakoi.html) should be able to
display anything on a computer, including but not limited to: text, video,
audio, sets, lists, trees, graphs, and tables.

Non-exhaustive list of unknowns:

- How can we make each of these things zoomable in a way that doesn't look bad?
  
  Related:
  - [GPU-Centered Font Rendering Directly from Glyph Outlines](http://jcgt.org/published/0006/02/02/paper.pdf)
  
    Gives a really nice approach to drawing TrueType fonts . The person behind
    this paper incorporated these algorithms into the [Slug Font Rendering
    Library](https://sluglibrary.com/). Unfortunately, it has a proprietary
    license, so we can't use it here.
    
    I believe the algorithm is patented, too---see [Method for rendering
    resolution-independent shapes directly from outline control
    points](https://patents.google.com/patent/US10373352B1).
  - [ab_glyph_rasterizer](https://crates.io/crates/ab_glyph_rasterizer)
  
    A rust crate for "Coverage rasterization for lines, quadratic & cubic
    beziers. Ueful for drawing .otf font glyphs".
    
    The downside of using this is that we'd have to continuously re-calculate
    rasterizations in order to avoid blurriness, which has the potential to be
    computationally expensive.
  - [RustType](https://crates.io/crates/rusttype)
  
    A rust crate described as "a pure Rust alternative to libraries like
    FreeType".

- How should the visual presentation of new fundamental data types be
  implemented?

  Presumably, we could expose some Vulkan functionality to the custom data
  types, just like how the fundamental data types would be implemented. I don't
  know enough about Vulkan to say more about this right now.
