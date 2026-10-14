# Vendored crates

## fontique 0.11.0 (crates.io, upstream commit 033e0b0)

The font collection under parley, patched in through `[patch.crates-io]` so that parley resolves
against the same copy. Without the examples. Every local change is marked `ZGUI-PATCH:` in the
source.

### Query state kept across queries (`collection/query.rs`)

Upstream clears a collection's query state when a query starts and again when it ends, and parley
starts one query per paragraph. For a generic family such as `sans-serif`, which a system
collection resolves to some two hundred families, every paragraph then rebuilt that list, and
every digit, which parley shapes as a possible emoji, rebuilt it again with the emoji families
appended. The state now outlives the query:

- the current family list and the last three are kept with the families and faces they loaded,
  and a `set_families` call that repeats one of them reuses it;
- the fallback list is rebuilt only when its script and locale change;
- loaded faces are dropped in `matches_with` when the attributes changed since they were matched;
- a query that sets no families or fallbacks still matches none.

`Collection` counts a generation, raised by every mutating method and by a sync that copies a
shared clone's changes; a query that finds the state built against another generation clears it.
`crates/zgui-text-parley/tests/fonts.rs`
(`a_face_unregistered_after_shaping_is_not_drawn_with_again`) pins the invalidation.
