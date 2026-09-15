#![allow(clippy::missing_errors_doc)]

use hm_core::{Error, ErrorCode};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BudgetProfile {
    pub mandatory_percent: u8,
    pub intent_bindings_work_percent: u8,
    pub conversation_percent: u8,
    pub recall_percent: u8,
    pub tools_percent: u8,
    pub reserve_percent: u8,
}

impl Default for BudgetProfile {
    fn default() -> Self {
        Self {
            mandatory_percent: 15,
            intent_bindings_work_percent: 10,
            conversation_percent: 40,
            recall_percent: 15,
            tools_percent: 10,
            reserve_percent: 10,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BudgetAllocation {
    pub mandatory: usize,
    pub intent_bindings_work: usize,
    pub conversation: usize,
    pub recall: usize,
    pub tools: usize,
    pub reserve: usize,
}

impl BudgetProfile {
    pub fn validate(self) -> Result<Self, Error> {
        let total = u16::from(self.mandatory_percent)
            + u16::from(self.intent_bindings_work_percent)
            + u16::from(self.conversation_percent)
            + u16::from(self.recall_percent)
            + u16::from(self.tools_percent)
            + u16::from(self.reserve_percent);
        if total == 100 {
            Ok(self)
        } else {
            Err(Error::new(ErrorCode::InvalidArgument))
        }
    }

    pub fn allocate(self, total: usize) -> Result<BudgetAllocation, Error> {
        self.validate()?;
        if total == 0 {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let share = |percent: u8| -> Result<usize, Error> {
            total
                .checked_mul(usize::from(percent))
                .map(|value| value / 100)
                .ok_or_else(|| Error::new(ErrorCode::CapacityExceeded))
        };
        let mandatory = share(self.mandatory_percent)?;
        let intent_bindings_work = share(self.intent_bindings_work_percent)?;
        let conversation = share(self.conversation_percent)?;
        let recall = share(self.recall_percent)?;
        let tools = share(self.tools_percent)?;
        let assigned = mandatory
            .checked_add(intent_bindings_work)
            .and_then(|value| value.checked_add(conversation))
            .and_then(|value| value.checked_add(recall))
            .and_then(|value| value.checked_add(tools))
            .ok_or_else(|| Error::new(ErrorCode::CapacityExceeded))?;
        let reserve = total
            .checked_sub(assigned)
            .ok_or_else(|| Error::new(ErrorCode::InvariantViolation))?;
        Ok(BudgetAllocation {
            mandatory,
            intent_bindings_work,
            conversation,
            recall,
            tools,
            reserve,
        })
    }
}
