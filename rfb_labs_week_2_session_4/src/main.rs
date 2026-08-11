//! Small executable for Part 8 of the assignment.

use rfb_labs_week_2_session_4::{Item, Library, LibraryError, MediaKind, Member};

fn main() -> Result<(), LibraryError> {
    let mut library = Library::new();

    library.add_item(Item::new(
        1,
        "Dune".into(),
        "Frank Herbert".into(),
        MediaKind::Book { pages: 320 },
    ))?;
    library.add_item(Item::new(
        2,
        "Project Hail Mary".into(),
        "Andy Weir".into(),
        MediaKind::Audiobook { minutes: 540 },
    ))?;
    library.add_item(Item::new(
        3,
        "The Rust Programming Language".into(),
        "Steve Klabnik".into(),
        MediaKind::Ebook { size_kb: 1_200 },
    ))?;

    library.register_member(Member::new(100, "Ada".into()))?;

    // A complete, on-time loan.
    library.checkout(2, 100, 0)?;
    let fee = library.return_item(2, 10)?;
    println!(
        "Ada returned item 2 on time, owing {fee} cents. Status: {}",
        library.find_item(2).unwrap()
    );

    // A late loan: books may be kept 21 days, this one is held for 30.
    library.checkout(1, 100, 0)?;
    let fee = library.return_item(1, 30)?;
    println!(
        "Ada returned item 1 late, owing {fee} cents. Status: {}",
        library.find_item(1).unwrap()
    );

    // Trigger and print a handled error.
    match library.checkout(1, 999, 30) {
        Ok(()) => unreachable!("member 999 was never registered"),
        Err(error) => println!("Expected error checking out for an unknown member: {error}"),
    }

    Ok(())
}

// Part 7 — ownership experiments. Both lines below fail to compile; the real
// `cargo check` errors and explanations are recorded in README.md.

// Experiment A: `item` was moved into `add_item`, so it can no longer be read.
// let item = Item::new(1, "Dune".into(), "Frank Herbert".into(), MediaKind::Book { pages: 320 });
// library.add_item(item)?;
// println!("{}", item.title);

// Experiment B: `held` is an immutable borrow of `library` that is still live
// when `checkout` tries to borrow `library` mutably.
// let held = library.find_item(1);
// library.checkout(1, 100, 0)?;
// println!("{:?}", held);
