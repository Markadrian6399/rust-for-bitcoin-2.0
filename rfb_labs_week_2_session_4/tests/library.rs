use rfb_labs_week_2_session_4::{Item, Library, LibraryError, LoanStatus, MediaKind, Member};

fn library_with_items() -> Library {
    let mut library = Library::new();

    for (id, title, author, kind) in [
        (1, "Dune", "Frank Herbert", MediaKind::Book { pages: 320 }),
        (
            2,
            "Children of Dune",
            "Frank Herbert",
            MediaKind::Book { pages: 180 },
        ),
        (
            3,
            "Project Hail Mary",
            "Andy Weir",
            MediaKind::Audiobook { minutes: 540 },
        ),
        (
            4,
            "The Rust Programming Language",
            "Steve Klabnik",
            MediaKind::Ebook { size_kb: 1_200 },
        ),
    ] {
        library
            .add_item(Item::new(id, title.into(), author.into(), kind))
            .unwrap();
    }

    library
        .register_member(Member::new(100, "Ada".into()))
        .unwrap();

    library
}

#[test]
fn checkout_updates_both_the_item_and_the_member() {
    let mut library = library_with_items();

    library.checkout(1, 100, 5).unwrap();

    assert_eq!(
        library.find_item(1).unwrap().status,
        LoanStatus::OnLoan {
            member_id: 100,
            day_borrowed: 5,
        }
    );
    assert_eq!(library.find_member(100).unwrap().borrowed_item_ids, vec![1]);
}

#[test]
fn a_member_cannot_exceed_the_borrow_limit() {
    let mut library = library_with_items();

    library.checkout(1, 100, 0).unwrap();
    library.checkout(2, 100, 0).unwrap();
    library.checkout(3, 100, 0).unwrap();

    assert_eq!(
        library.checkout(4, 100, 0),
        Err(LibraryError::BorrowLimitReached {
            member_id: 100,
            limit: 3,
        })
    );
}

#[test]
fn returning_a_book_late_charges_a_daily_fee() {
    let mut library = library_with_items();

    // A book may be kept 21 days. Held for 30, so 9 days are overdue.
    library.checkout(1, 100, 10).unwrap();

    assert_eq!(library.return_item(1, 40), Ok(9 * 25));
    assert_eq!(library.find_item(1).unwrap().status, LoanStatus::Available);
    assert!(
        library
            .find_member(100)
            .unwrap()
            .borrowed_item_ids
            .is_empty()
    );
}

#[test]
fn searching_by_author_borrows_rather_than_clones() {
    let library = library_with_items();

    let found = library.items_by_author("Frank Herbert");

    assert_eq!(found.len(), 2);
    assert_eq!(found[0].title, "Dune");
    // `found` holds references into `library`, so these are the same item.
    assert!(std::ptr::eq(found[0], library.find_item(1).unwrap()));
}

#[test]
fn author_search_still_finds_a_borrowed_item() {
    let mut library = library_with_items();

    library.checkout(1, 100, 0).unwrap();

    let found = library.items_by_author("Frank Herbert");
    assert_eq!(found.len(), 2);
}

#[test]
fn an_item_already_on_loan_cannot_be_checked_out_again() {
    let mut library = library_with_items();

    library.checkout(1, 100, 0).unwrap();
    library
        .register_member(Member::new(101, "Bob".into()))
        .unwrap();

    assert_eq!(
        library.checkout(1, 101, 1),
        Err(LibraryError::ItemAlreadyOnLoan {
            id: 1,
            member_id: 100,
        })
    );
}

#[test]
fn an_on_time_return_owes_nothing() {
    let mut library = library_with_items();

    library.checkout(1, 100, 0).unwrap();

    assert_eq!(library.return_item(1, 21), Ok(0));
}

#[test]
fn an_ebook_returned_late_owes_nothing() {
    let mut library = library_with_items();

    library.checkout(4, 100, 0).unwrap();

    assert_eq!(library.return_item(4, 100), Ok(0));
}

#[test]
fn adding_an_item_with_an_empty_title_is_rejected() {
    let mut library = Library::new();

    let item = Item::new(
        1,
        String::new(),
        "Author".into(),
        MediaKind::Book { pages: 1 },
    );

    assert_eq!(library.add_item(item), Err(LibraryError::EmptyTitle));
}

#[test]
fn adding_an_item_with_a_duplicate_id_is_rejected() {
    let mut library = library_with_items();

    let item = Item::new(
        1,
        "Another Title".into(),
        "Someone".into(),
        MediaKind::Book { pages: 1 },
    );

    assert_eq!(
        library.add_item(item),
        Err(LibraryError::DuplicateItemId { id: 1 })
    );
}

#[test]
fn registering_a_member_with_a_duplicate_id_is_rejected() {
    let mut library = library_with_items();

    assert_eq!(
        library.register_member(Member::new(100, "Someone Else".into())),
        Err(LibraryError::DuplicateMemberId { id: 100 })
    );
}

#[test]
fn checkout_of_an_unknown_item_is_rejected() {
    let mut library = library_with_items();

    assert_eq!(
        library.checkout(999, 100, 0),
        Err(LibraryError::ItemNotFound { id: 999 })
    );
}

#[test]
fn checkout_by_an_unknown_member_is_rejected() {
    let mut library = library_with_items();

    assert_eq!(
        library.checkout(1, 999, 0),
        Err(LibraryError::MemberNotFound { id: 999 })
    );
}

#[test]
fn checkout_of_a_lost_item_is_rejected() {
    let mut library = Library::new();

    let mut item = Item::new(
        1,
        "Dune".into(),
        "Frank Herbert".into(),
        MediaKind::Book { pages: 320 },
    );
    item.status = LoanStatus::Lost;
    library.add_item(item).unwrap();
    library
        .register_member(Member::new(100, "Ada".into()))
        .unwrap();

    assert_eq!(
        library.checkout(1, 100, 0),
        Err(LibraryError::ItemIsLost { id: 1 })
    );
}

#[test]
fn returning_an_unknown_item_is_rejected() {
    let mut library = library_with_items();

    assert_eq!(
        library.return_item(999, 0),
        Err(LibraryError::ItemNotFound { id: 999 })
    );
}

#[test]
fn returning_a_lost_item_is_rejected() {
    let mut library = Library::new();

    let mut item = Item::new(
        1,
        "Dune".into(),
        "Frank Herbert".into(),
        MediaKind::Book { pages: 320 },
    );
    item.status = LoanStatus::Lost;
    library.add_item(item).unwrap();

    assert_eq!(
        library.return_item(1, 0),
        Err(LibraryError::ItemIsLost { id: 1 })
    );
}

#[test]
fn returning_an_item_that_is_not_on_loan_is_rejected() {
    let mut library = library_with_items();

    assert_eq!(
        library.return_item(1, 0),
        Err(LibraryError::ItemNotOnLoan { id: 1 })
    );
}

#[test]
fn returning_before_the_borrow_day_is_rejected() {
    let mut library = library_with_items();

    library.checkout(1, 100, 10).unwrap();

    assert_eq!(
        library.return_item(1, 5),
        Err(LibraryError::InvalidReturnDay {
            day_borrowed: 10,
            day_returned: 5,
        })
    );
}

#[test]
fn filter_items_supports_an_arbitrary_predicate() {
    let library = library_with_items();

    let found = library.filter_items(|item| matches!(item.kind, MediaKind::Book { .. }));

    assert_eq!(found.len(), 2);
}
