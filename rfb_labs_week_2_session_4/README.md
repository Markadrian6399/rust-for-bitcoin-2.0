# Rust for Bitcoin 2.0 — Week 2, Session 4

Build a small lending library while practising structs, enums, traits,
ownership, borrowing, collections, and `Result`-based error handling. No
Bitcoin and no external crates — just Rust.

The crate is intentionally incomplete. Search for `TODO` and implement each
part; do not change the public type names or function signatures.

## Recommended workflow

1. Read [ASSIGNMENT.md](ASSIGNMENT.md).
2. Complete Part 2 in `error.rs`, then Part 3 in `library.rs`.
3. Remove `#[ignore]` from the relevant test and run it.
4. Complete the traits in Part 4 and the two operations in Parts 5–6.
5. Run the ownership experiments and record the errors.
6. Build the demo in `main.rs`.
7. Add the remaining required tests yourself.

```bash
cargo test
cargo test -- --ignored
cargo run
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
```

`cargo test` checks the starter project. Ignored tests intentionally exercise
unfinished code; enable them progressively rather than leaving them ignored in
the submission.

## Part 7 — ownership experiments

**Experiment A** — read `item.title` after `library.add_item(item)?`:

```
error[E0382]: borrow of moved value: `item`
 --> examples/exp_a.rs:7:20
  |
5 |     let item = Item::new(1, "Dune".into(), "Frank Herbert".into(), MediaKind::Book { pages: 320 });
  |         ---- move occurs because `item` has type `Item`, which does not implement the `Copy` trait
6 |     library.add_item(item).unwrap();
  |                      ---- value moved here
7 |     println!("{}", item.title);
  |                    ^^^^^^^^^^ value borrowed here after move
```

`add_item` takes `item: Item` by value, so calling it moves ownership of the
`Item` into the `Vec` inside `Library`. `item` the local variable is no longer
a valid binding after that call — the compiler statically tracks that the
value was moved and refuses to let the old binding be read again, because
reading it could observe a value that has (conceptually) been relocated.
There is nothing left to read `item.title` from at the call site; the title
now lives inside `library`, reachable only through `library.find_item(1)`.

**Experiment B** — hold the result of `library.find_item(1)`, call
`library.checkout(..)?`, then print what was held:

```
error[E0502]: cannot borrow `library` as mutable because it is also borrowed as immutable
  --> examples/exp_b.rs:10:5
   |
 9 |     let held = library.find_item(1);
   |                ------- immutable borrow occurs here
10 |     library.checkout(1, 100, 0).unwrap();
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^ mutable borrow occurs here
11 |     println!("{:?}", held);
   |                      ---- immutable borrow later used here
```

`find_item` returns `Option<&Item>`, a shared borrow tied to `library`'s
lifetime. Because `held` is used again in the final `println!`, that borrow is
still alive across the `checkout` call. `checkout` needs `&mut self` to update
the item's status and the member's list, and Rust's borrow checker will not
allow a mutable borrow to coexist with a live immutable one — a mutation
could invalidate the reference `held` points to (e.g. if `items` were a
growable collection that reallocates). The fix is to drop `held` (or only use
it before the mutating call) before calling `checkout`.

Both experiments are recorded (commented out) in [`src/main.rs`](src/main.rs)
next to the working demo.

## Written answers

1. **Why is `LoanStatus` an enum rather than a `bool` plus two `Option`
   fields?** An enum makes illegal states unrepresentable. With a `bool` plus
   two `Option<u32>` fields (say, `is_lost: bool`, `member_id: Option<u32>`,
   `day_borrowed: Option<u32>`), nothing stops `is_lost == true` while
   `member_id` is also `Some(_)`, or `member_id` being `Some` while
   `day_borrowed` is `None`. `LoanStatus` collapses those combinations into
   exactly the three that are meaningful: `Available`, `OnLoan { member_id,
   day_borrowed }` (both fields present together, always), and `Lost`. A
   `match` on the enum is exhaustive, so every call site is forced to handle
   all three cases — the compiler, not a runtime bug report, catches a
   forgotten case.

2. **What does `match` force you to do when a fourth `MediaKind` is added
   later?** Every non-wildcard `match` on `MediaKind` (in `loan_days`,
   `daily_late_fee_cents`, and the `Display` impl) fails to compile with a
   "non-exhaustive patterns" error until the new variant is handled
   explicitly. That is the main payoff of enums over open-ended
   representations: the compiler enumerates every call site that needs a
   decision for the new case, rather than leaving it to be discovered at
   runtime (or never).

3. **`Item::new` takes `String` rather than `&str`. Who owns the title
   afterwards?** The `Item` owns it. Taking `String` means the caller has
   already decided to hand over a heap allocation it controls; `Item::new`
   just moves that allocation into the `title` field. If it took `&str`
   instead, `Item` would need its own `String` (via `.to_owned()` inside the
   constructor) or a lifetime parameter tying it to the caller's string —
   either adds complexity for no benefit here, since every caller in this
   crate already owns a `String` (or a string literal it can `.into()`).

4. **Why does `add_item` take `self` by `&mut` but `item` by value?**
   `&mut self` is a mutable borrow: the library must be modified in place
   (pushing into its `Vec`), but the caller still owns `library` after the
   call and can keep using it. `item` is different — the whole point is to
   transfer ownership of the `Item` into the library's `Vec<Item>` so the
   library becomes the sole owner going forward. Taking `item` by value is
   what makes that transfer possible; a borrow (`&Item`) would only let the
   library look at data it doesn't own, which isn't enough to store it.

5. **When `add_item` returns `Err`, what happened to the `Item` the caller
   passed in? Was that a good design choice, and what is the alternative?**
   Because `item: Item` is taken by value and the error paths return before
   `self.items.push(item)`, the `Item` is dropped at the end of `add_item`'s
   scope — it is gone, not handed back to the caller. That is a reasonable
   choice here because the two rejection reasons (empty title, duplicate id)
   are checked before anything is consumed and a caller can trivially
   reconstruct or re-send an equivalent `Item` if it wants to retry. The
   alternative, used by APIs that want to let callers retry without
   rebuilding, is to put the rejected value inside the error itself (e.g.
   `Err(LibraryError::EmptyTitle(item))`), giving ownership back through the
   `Result`.

6. **Why does `find_item` return `Option<&Item>` rather than `Option<Item>`?**
   Returning `Option<Item>` would require either cloning the item (extra
   allocation, and a copy that immediately goes stale if the original
   changes) or moving it out of the `Vec` (leaving a hole, and usually
   impossible without `Clone`/`Default`/index tricks). Returning
   `Option<&Item>` lets the caller look at the library's own data in place,
   for as long as it needs, at zero cost — and the borrow checker guarantees
   that reference can't outlive the `Library` it points into.

7. **What is the lifetime `'a` in `items_by_author` actually saying?** In
   `pub fn items_by_author<'a>(&'a self, author: &str) -> Vec<&'a Item>`,
   `'a` ties the lifetime of every reference in the returned `Vec` to the
   borrow of `self`. It says: "the `&Item`s you get back are only valid for as
   long as this particular borrow of the `Library` is valid" — i.e. the
   returned vector cannot outlive the `library` value it was produced from,
   and the borrow checker will reject any attempt to use it after `library`
   is dropped or mutably borrowed again. `author: &str` deliberately has an
   independent, elided lifetime, since the returned data borrows from `self`,
   not from `author`.

8. **Why can't `checkout` hold a `&mut Item` and a `&mut Member` from the same
   `Library` at once, and how did you structure the method around that?**
   `Item` and `Member` live in two different `Vec`s (`self.items` and
   `self.members`) inside the same `Library`, but both are reached through
   `&mut self`. Rust's aliasing rules are per-borrow-checker-pass, not
   per-field-tracked-across-a-method the way this might suggest at first
   glance — `iter_mut().find(..)` on `self.items` and `iter_mut().find(..)`
   on `self.members` don't actually alias (they're disjoint fields), but two
   *sequential* mutable borrows from the same `&mut self` still can't be held
   live *simultaneously* as named bindings without running into the single
   `&mut self` receiver. The straightforward way around this is exactly what
   `checkout` does: look everything up immutably first (with plain
   `iter().find(..)`, validating every rule while `self` is only borrowed
   shared), decide the outcome, then take two short-lived, sequential mutable
   borrows — one to update the item's status, one to push onto the member's
   list — never holding both `&mut` borrows open at the same time.

9. **Why are `Library`'s fields private?** So the only way to change an
   item's `LoanStatus` or a member's `borrowed_item_ids` is through
   `Library`'s own methods (`checkout`, `return_item`), which are written to
   update both together. If `items` and `members` were public, any caller
   could set an item to `OnLoan` without touching the member's list (or vice
   versa), and the two would drift out of sync — an item could look
   borrowed while no member's list mentions it, or a member's list could
   reference an item that's actually `Available`.

10. **What duplication does the provided `late_fee_cents` removes, and what
    would you lose by making it a free function instead?** Without it, both
    `impl LoanTerms for MediaKind` and `impl LoanTerms for Item` would need
    to repeat `days_held.saturating_sub(self.loan_days()) *
    self.daily_late_fee_cents()` verbatim — two copies of the same formula
    that could silently drift apart if one were edited and the other
    forgotten. Because it's a default method on the trait, both impls get it
    for free by implementing only `loan_days` and `daily_late_fee_cents`. A
    free function (`fn late_fee_cents(loan_days: u32, daily_fee: u32,
    days_held: u32) -> u32`) would remove the duplication too, but it would
    lose the ergonomics of being called as `item.late_fee_cents(days_held)` —
    callers would have to pull `loan_days()` and `daily_late_fee_cents()` out
    manually first and pass them along, and the connection between "a type
    that has loan terms" and "how its late fee is computed" would no longer
    be expressed in the type system at all.

11. **Why is `Result` preferable to `panic!` for validation failures? Name a
    place in this crate where a panic would be defensible.** Validation
    failures here (an unknown id, a duplicate id, a return day before the
    borrow day) are *expected*, recoverable outcomes of normal library
    operation — a caller might legitimately look up an id that doesn't exist
    yet, or attempt a checkout that's supposed to fail a business rule.
    `panic!` unwinds (or aborts) the whole program/thread and gives the
    caller no chance to react; `Result` turns the same failure into an
    ordinary value the caller can match on, log, retry around, or propagate
    with `?`. A panic would be defensible somewhere that signals a broken
    *internal* invariant rather than bad input — e.g. the `.unwrap()` calls
    in `checkout`/`return_item` right after validation has already confirmed
    the item and member exist: if those `.find` calls ever returned `None`
    there, it would mean the library's own bookkeeping is corrupted, which is
    a bug worth crashing loudly on rather than quietly returning a
    misleading `Result`.

12. **Which derive did you deliberately leave off a type, and why?** `Item`
    and `Member` do not derive `Clone` (or `Copy`). Both are meant to be
    owned by exactly one place (`Library`'s vectors) and reached everywhere
    else through borrows (`find_item`, `items_by_author`, etc.). Leaving
    `Clone` off means the compiler enforces that discipline: it's impossible
    to accidentally end up with two independent `Item`s sharing an id whose
    `LoanStatus` values can drift apart, because there is no way to make a
    second copy in the first place — every `Item` you can get your hands on
    is either *the* one inside the library or a reference to it.

## Design notes

The core invariant this crate protects is: an item's `LoanStatus` and the
borrowing member's `borrowed_item_ids` always agree. That's why `Library`'s
fields are private and `Item`/`Member` aren't `Clone` — the only door in is
`checkout`/`return_item`, and each of those methods updates both sides of the
relationship in the same call, right next to each other, so there's no window
where one side is updated and the other forgotten. `checkout` and
`return_item` both follow "validate first, mutate second": every rejection
path runs entirely against shared (`&self`-style) lookups before any `&mut`
borrow is taken, so a validation failure never leaves partial mutation behind
— the method either fully succeeds or changes nothing.

The optional generic search (Part 9) is implemented as `Library::filter_items`,
taking `F: Fn(&Item) -> bool`. `items_by_author` and `available_items` are now
both one-line calls into it with a closure, and the new
`filter_items_supports_an_arbitrary_predicate` test checks it directly with an
ad hoc predicate (matching on `MediaKind::# Week 2 Session 4 Assignment — A Community Lending Library

The point is enums, structs, traits,
ownership, borrowing, and `Result`, with nothing else competing for attention.
Money is in whole cents; time is a whole day number counted from an arbitrary
epoch.

## Required work

- [ ] **Part 1 — Data model:** review the provided `MediaKind`, `LoanStatus`,
  `Item`, and `Member` types. Explain why `LoanStatus` is an enum rather than a
  `bool` plus two `Option` fields, and what `match` forces you to handle.
- [ ] **Part 2 — Errors:** implement useful `Display` messages for every
  `LibraryError`, including the ids each variant carries. Expected invalid data
  must never call `panic!`.
- [ ] **Part 3 — Ownership and borrowing:** implement `add_item` and
  `register_member`, which take ownership and reject empty titles and duplicate
  ids. Implement `find_item`, `find_member`, `items_by_author`, and
  `available_items` using borrowed references without cloning.
- [ ] **Part 4 — Traits:** implement `LoanTerms` for both `MediaKind` and
  `Item`, writing the shared fee formula once in `late_fee_cents`. Implement
  `Display` for `MediaKind`, `LoanStatus`, and `Item`, and `longest_loan_item`.
- [ ] **Part 5 — Checkout:** implement `checkout`. Validate first and mutate
  second; on success the item's status and the member's borrowed list must both
  change. Use `?` where appropriate.
- [ ] **Part 6 — Return:** implement `return_item`. Compute the days held with
  checked arithmetic, charge the fee through `LoanTerms`, set the item back to
  `Available`, and drop its id from the member's list.
- [ ] **Part 7 — Experiments:** complete the two ownership experiments and
  record the compiler errors in `README.md`.
- [ ] **Part 8 — Demo:** in `main.rs`, stock a library, register a member, run
  a complete loan and a late return, and print one handled error using its
  `Display` message. `main` returns `Result`, so use `?`.
- [ ] **Part 9 (optional) — Generic search:** add `filter_items` taking a
  `Fn(&Item) -> bool` and re-express the two filtered lookups in terms of it.

## Loan terms and validation rules

Books may be kept 21 days, audiobooks 14, ebooks 7. Late items cost 25 cents a
day; ebooks are never late. `checkout` checks, in this order: unknown item,
unknown member, lost item, item already on loan, then borrow limit reached. The
order matters — a caller fixing one problem at a time deserves a predictable
next error. `return_item` rejects an unknown item, a lost item, an item that is
not on loan, and a return day earlier than the borrow day.

## The two experiments

Run each, paste the real `cargo check` error into `README.md`, explain it, then
comment the line out. **A** — read `item.title` after `library.add_item(item)?`.
**B** — hold the result of `library.find_item(1)`, call `library.checkout(..)?`,
then print what you held.

## Testing checklist

Write tests for a successful checkout, an item that cannot be lent twice, the
borrow limit, a late return's fee, an on-time return owing nothing, an ebook
returned late still owing nothing, and author search returning borrowed items.
Also test each validation error. The repository contains a few ignored starter
tests; remove their `#[ignore]` attributes and add the remaining cases.Book`).

## Example output

```
$ cargo run
Ada returned item 2 on time, owing 0 cents. Status: "Project Hail Mary" by Andy Weir (audiobook, 540 minutes) — available
Ada returned item 1 late, owing 225 cents. Status: "Dune" by Frank Herbert (book, 320 pages) — available
Expected error checking out for an unknown member: no member with id 999
```
