#![allow(clippy::missing_errors_doc)]

use hm_core::{Error, ErrorCode, LSN};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Default)]
pub struct TripwireSet {
    lsns: BTreeSet<LSN>,
}

impl TripwireSet {
    pub fn seeded(lsns: impl IntoIterator<Item = LSN>) -> Result<Self, Error> {
        let lsns: BTreeSet<_> = lsns.into_iter().collect();
        if lsns.iter().any(|lsn| lsn.get() == 0) {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        Ok(Self { lsns })
    }

    pub fn guard(&self, touched_lsns: impl IntoIterator<Item = LSN>) -> Result<(), Error> {
        for lsn in touched_lsns {
            if self.lsns.contains(&lsn) {
                return Err(Error::new(ErrorCode::Tripwire).at_lsn(lsn));
            }
        }
        Ok(())
    }

    #[must_use]
    pub fn contains(&self, lsn: LSN) -> bool {
        self.lsns.contains(&lsn)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.lsns.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lsns.is_empty()
    }
}
