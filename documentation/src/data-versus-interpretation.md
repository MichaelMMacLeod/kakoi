# kakoi - design goal: appreciate the distinction between data and its interpretation #

[kakoi](kakoi.html) separates the interpretation of data from the data itself.
This may be implemented by storing exactly two different types of data: binary
blobs, and functions `BinaryBlob -> Interpretation`. Users of the system may
associate (encircle) a function and a binary blob. By doing so,
[kakoi](kakoi.html) will use the interpretation of the data given by the
function.

There should be several built-in interpretations available including utf-8 text,
png images, and video files.
