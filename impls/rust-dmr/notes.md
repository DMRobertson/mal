TODO
====

* `make perf^rust-dmr` runs `env STEP=stepA_mal MAL_IMPL=js ../rust-dmr/run ../tests/perf3.mal` from within the rust-dmr directory.
This causes a stack overflow :(. Infinite recursion somewhere?


My thoughts on implementing mal
===============================

* I remember being a bit frustrated that sometimes I wouldn't have the full picture. I'd implement something in one way, only to learn something new about the language which would change how I'd implement this. On the other hand, maybe the author would say that you constantly have to update your understanding when developing real software!


Types
=====

A Mal object is one of the following:

* A scalar type representing some kind of indivisible data.
    - A **nil** value carrying no payload. 
    - An **integer**.
    - A **boolean**.
    - A **string**.
    - A **symbol**, whose payload is a string.

* A composite of multiple Mal objects. 
    - A **list** of 0 or more Mal objects.
    - A **vector**, which is also a list of 0 or more Mal objects.
    - A **map** from symbols or keywords to mal objects.

* A callable function. 
    - A **primitive function** which is provided by the implementation
    - A **closure**, a user-defined function which captures the environment in which it was defined
    
* A single reference type:
    - An **atom**, an mutable indirection which points to another Mal object. This has nothing to do with the "atom" mentioned in `read_atom`.

Note that everything is immutable except for atoms.

Literals
--------

* Integer literals are a series of one or more arabic digits `0123456789`, optionally preceded by a `+` or `-`.
* String literals are textual data enclosed within double quotes `"`. The usual escapes `\"` and `\\` express literal double quotes and backslashes, respectively.
* Symbol literals are sequences of one or more "plain" characters. All characters are plain except for the following: whitespace, `[`, `]`, `{`, `}`, `(`, `)`, `'`, `"`, `` ` ``, `,`, `;`.
* List literals are a pair of round brackets `( ... )` which enclose a sequence of zero or more mal literals separated by whitespace or commas.
    - I think the whitespace is only necessary for disambiguation: so we can distinguish `(12)` from `(1 2)`. I don't think the grammar always requires this, e.g. I think `("a""b)` represents a list of two strings by the letter of the law.
* Vector literals are list literals that use square brackets `[ ... ]` instead of round brackets.
* Map literals are a list literals that use brace brackets `{}`.

The other mal data types cannot be expressed with literal syntax.

Remarks
=======

At first glance it feels like there's duplication going on here. Why do we need to distinguish strings, symbols and keywords, if they all carry the same data? I think the point is to express intent. Strings are data; symbols are names. You can associate a value like the integer `123` to a symbol `x`, but not to a string `"x"`. (To make an analogy, consider the difference between `let x = 2` and `let "x" = 2`.)

Common Lisp speaks of "keyword symbols"; I think it's best to consider keywords a special kind of symbol. Why the need for special symbols? Clojure writes that "keywords always evaluate to themselves", but one evaluates a symbol by retrieving the value associated with it from the environment. Emacs Lisp explicitly considers `nil` and `t` to be symbols that evaluate to themselves; there's no special particular data type; there is only the symbol `nil`.

Why do we need to distinguish between lists and vectors? Not sure I have a good answer here, but I think it's like symbols and keywords. We might want to evaluate or interpret a sequence of objects, or we might like to leave it alone as raw data. Contrast the list `(+ 1 2)` and vector `[+ 1 2]`. Their contents are the same, but the left is evaluated as a function call; the right evaluates to itself. A quick search also threw up that Clojure's lists extend from the front, but its vectors extend from the end.
