use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKind {
    Book { pages: u32 },
    Audiobook { minutes: u32 },
    Ebook { size_kb: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoanStatus {
    Available,
    OnLoan { member_id: u32, day_borrowed: u32 },
    Lost,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Item {
    pub id: u32,
    pub title: String,
    pub author: String,
    pub kind: MediaKind,
    pub status: LoanStatus,
}

/// How long each kind of media may be kept, and what it costs to keep it too
/// long. Loan lengths are in whole days; fees are in whole cents.
pub trait LoanTerms {
    fn loan_days(&self) -> u32;

    fn daily_late_fee_cents(&self) -> u32;

    fn late_fee_cents(&self, days_held: u32) -> u32 {
        let overdue_days = days_held.saturating_sub(self.loan_days());
        overdue_days * self.daily_late_fee_cents()
    }
}

impl Item {
    pub fn new(id: u32, title: String, author: String, kind: MediaKind) -> Self {
        Self {
            id,
            title,
            author,
            kind,
            status: LoanStatus::Available,
        }
    }
}

impl LoanTerms for MediaKind {
    fn loan_days(&self) -> u32 {
        match self {
            MediaKind::Book { .. } => 21,
            MediaKind::Audiobook { .. } => 14,
            MediaKind::Ebook { .. } => 7,
        }
    }

    fn daily_late_fee_cents(&self) -> u32 {
        match self {
            MediaKind::Book { .. } | MediaKind::Audiobook { .. } => 25,
            MediaKind::Ebook { .. } => 0,
        }
    }
}

impl LoanTerms for Item {
    fn loan_days(&self) -> u32 {
        self.kind.loan_days()
    }

    fn daily_late_fee_cents(&self) -> u32 {
        self.kind.daily_late_fee_cents()
    }
}

impl fmt::Display for MediaKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MediaKind::Book { pages } => write!(formatter, "book, {pages} pages"),
            MediaKind::Audiobook { minutes } => write!(formatter, "audiobook, {minutes} minutes"),
            MediaKind::Ebook { size_kb } => write!(formatter, "ebook, {size_kb} KB"),
        }
    }
}

impl fmt::Display for LoanStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoanStatus::Available => write!(formatter, "available"),
            LoanStatus::OnLoan {
                member_id,
                day_borrowed,
            } => write!(
                formatter,
                "on loan to member {member_id} since day {day_borrowed}"
            ),
            LoanStatus::Lost => write!(formatter, "lost"),
        }
    }
}

impl fmt::Display for Item {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "\"{}\" by {} ({}) — {}",
            self.title, self.author, self.kind, self.status
        )
    }
}
