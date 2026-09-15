pub use root::*;

const _: () = ::planus::check_version_compatibility("planus-1.3.0");

/// The root namespace
///
/// Generated from these locations:
/// * File `schemas/events.fbs`
#[no_implicit_prelude]
#[allow(clippy::needless_lifetimes)]
mod root {
    /// The namespace `hypermind`
    ///
    /// Generated from these locations:
    /// * File `schemas/events.fbs`
    pub mod hypermind {
        /// The namespace `hypermind.schema`
        ///
        /// Generated from these locations:
        /// * File `schemas/events.fbs`
        pub mod schema {
            /// The enum `ResultStatus` in the namespace `hypermind.schema`
            ///
            /// Generated from these locations:
            /// * Enum `ResultStatus` in the file `schemas/events.fbs:3`
            #[derive(
                Copy,
                Clone,
                Debug,
                PartialEq,
                Eq,
                PartialOrd,
                Ord,
                Hash,
                ::serde::Serialize,
                ::serde::Deserialize,
            )]
            #[repr(u8)]
            pub enum ResultStatus {
                /// The variant `ok` in the enum `ResultStatus`
                Ok = 0,

                /// The variant `error` in the enum `ResultStatus`
                Error = 1,

                /// The variant `outcome_unknown` in the enum `ResultStatus`
                OutcomeUnknown = 2,
            }

            impl ResultStatus {
                /// Array containing all valid variants of ResultStatus
                pub const ENUM_VALUES: [Self; 3] = [Self::Ok, Self::Error, Self::OutcomeUnknown];
            }

            impl ::core::convert::TryFrom<u8> for ResultStatus {
                type Error = ::planus::errors::UnknownEnumTagKind;
                #[inline]
                fn try_from(
                    value: u8,
                ) -> ::core::result::Result<Self, ::planus::errors::UnknownEnumTagKind>
                {
                    #[allow(clippy::match_single_binding)]
                    match value {
                        0 => ::core::result::Result::Ok(ResultStatus::Ok),
                        1 => ::core::result::Result::Ok(ResultStatus::Error),
                        2 => ::core::result::Result::Ok(ResultStatus::OutcomeUnknown),

                        _ => ::core::result::Result::Err(::planus::errors::UnknownEnumTagKind {
                            tag: value as i128,
                        }),
                    }
                }
            }

            impl ::core::convert::From<ResultStatus> for u8 {
                #[inline]
                fn from(value: ResultStatus) -> Self {
                    value as u8
                }
            }

            /// # Safety
            /// The Planus compiler correctly calculates `ALIGNMENT` and `SIZE`.
            unsafe impl ::planus::Primitive for ResultStatus {
                const ALIGNMENT: usize = 1;
                const SIZE: usize = 1;
            }

            impl ::planus::WriteAsPrimitive<ResultStatus> for ResultStatus {
                #[inline]
                fn write<const N: usize>(
                    &self,
                    cursor: ::planus::Cursor<'_, N>,
                    buffer_position: u32,
                ) {
                    (*self as u8).write(cursor, buffer_position);
                }
            }

            impl ::planus::WriteAs<ResultStatus> for ResultStatus {
                type Prepared = Self;

                #[inline]
                fn prepare(&self, _builder: &mut ::planus::Builder) -> ResultStatus {
                    *self
                }
            }

            impl ::planus::WriteAsDefault<ResultStatus, ResultStatus> for ResultStatus {
                type Prepared = Self;

                #[inline]
                fn prepare(
                    &self,
                    _builder: &mut ::planus::Builder,
                    default: &ResultStatus,
                ) -> ::core::option::Option<ResultStatus> {
                    if self == default {
                        ::core::option::Option::None
                    } else {
                        ::core::option::Option::Some(*self)
                    }
                }
            }

            impl ::planus::WriteAsOptional<ResultStatus> for ResultStatus {
                type Prepared = Self;

                #[inline]
                fn prepare(
                    &self,
                    _builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<ResultStatus> {
                    ::core::option::Option::Some(*self)
                }
            }

            impl<'buf> ::planus::TableRead<'buf> for ResultStatus {
                #[inline]
                fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'buf>,
                    offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::ErrorKind> {
                    let n: u8 = ::planus::TableRead::from_buffer(buffer, offset)?;
                    ::core::result::Result::Ok(::core::convert::TryInto::try_into(n)?)
                }
            }

            impl<'buf> ::planus::VectorReadInner<'buf> for ResultStatus {
                type Error = ::planus::errors::UnknownEnumTag;
                const STRIDE: usize = 1;
                #[inline]
                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'buf>,
                    offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::UnknownEnumTag>
                {
                    let value = unsafe { *buffer.buffer.get_unchecked(offset) };
                    let value: ::core::result::Result<Self, _> =
                        ::core::convert::TryInto::try_into(value);
                    value.map_err(|error_kind| {
                        error_kind.with_error_location(
                            "ResultStatus",
                            "VectorRead::from_buffer",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<ResultStatus> for ResultStatus {
                const STRIDE: usize = 1;

                type Value = Self;

                #[inline]
                fn prepare(&self, _builder: &mut ::planus::Builder) -> Self {
                    *self
                }

                #[inline]
                unsafe fn write_values(
                    values: &[Self],
                    bytes: *mut ::core::mem::MaybeUninit<u8>,
                    buffer_position: u32,
                ) {
                    let bytes = bytes as *mut [::core::mem::MaybeUninit<u8>; 1];
                    for (i, v) in ::core::iter::Iterator::enumerate(values.iter()) {
                        ::planus::WriteAsPrimitive::write(
                            v,
                            ::planus::Cursor::new(unsafe { &mut *bytes.add(i) }),
                            buffer_position - i as u32,
                        );
                    }
                }
            }

            /// The enum `ApprovalDecision` in the namespace `hypermind.schema`
            ///
            /// Generated from these locations:
            /// * Enum `ApprovalDecision` in the file `schemas/events.fbs:4`
            #[derive(
                Copy,
                Clone,
                Debug,
                PartialEq,
                Eq,
                PartialOrd,
                Ord,
                Hash,
                ::serde::Serialize,
                ::serde::Deserialize,
            )]
            #[repr(u8)]
            pub enum ApprovalDecision {
                /// The variant `approved` in the enum `ApprovalDecision`
                Approved = 0,

                /// The variant `denied` in the enum `ApprovalDecision`
                Denied = 1,
            }

            impl ApprovalDecision {
                /// Array containing all valid variants of ApprovalDecision
                pub const ENUM_VALUES: [Self; 2] = [Self::Approved, Self::Denied];
            }

            impl ::core::convert::TryFrom<u8> for ApprovalDecision {
                type Error = ::planus::errors::UnknownEnumTagKind;
                #[inline]
                fn try_from(
                    value: u8,
                ) -> ::core::result::Result<Self, ::planus::errors::UnknownEnumTagKind>
                {
                    #[allow(clippy::match_single_binding)]
                    match value {
                        0 => ::core::result::Result::Ok(ApprovalDecision::Approved),
                        1 => ::core::result::Result::Ok(ApprovalDecision::Denied),

                        _ => ::core::result::Result::Err(::planus::errors::UnknownEnumTagKind {
                            tag: value as i128,
                        }),
                    }
                }
            }

            impl ::core::convert::From<ApprovalDecision> for u8 {
                #[inline]
                fn from(value: ApprovalDecision) -> Self {
                    value as u8
                }
            }

            /// # Safety
            /// The Planus compiler correctly calculates `ALIGNMENT` and `SIZE`.
            unsafe impl ::planus::Primitive for ApprovalDecision {
                const ALIGNMENT: usize = 1;
                const SIZE: usize = 1;
            }

            impl ::planus::WriteAsPrimitive<ApprovalDecision> for ApprovalDecision {
                #[inline]
                fn write<const N: usize>(
                    &self,
                    cursor: ::planus::Cursor<'_, N>,
                    buffer_position: u32,
                ) {
                    (*self as u8).write(cursor, buffer_position);
                }
            }

            impl ::planus::WriteAs<ApprovalDecision> for ApprovalDecision {
                type Prepared = Self;

                #[inline]
                fn prepare(&self, _builder: &mut ::planus::Builder) -> ApprovalDecision {
                    *self
                }
            }

            impl ::planus::WriteAsDefault<ApprovalDecision, ApprovalDecision> for ApprovalDecision {
                type Prepared = Self;

                #[inline]
                fn prepare(
                    &self,
                    _builder: &mut ::planus::Builder,
                    default: &ApprovalDecision,
                ) -> ::core::option::Option<ApprovalDecision> {
                    if self == default {
                        ::core::option::Option::None
                    } else {
                        ::core::option::Option::Some(*self)
                    }
                }
            }

            impl ::planus::WriteAsOptional<ApprovalDecision> for ApprovalDecision {
                type Prepared = Self;

                #[inline]
                fn prepare(
                    &self,
                    _builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<ApprovalDecision> {
                    ::core::option::Option::Some(*self)
                }
            }

            impl<'buf> ::planus::TableRead<'buf> for ApprovalDecision {
                #[inline]
                fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'buf>,
                    offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::ErrorKind> {
                    let n: u8 = ::planus::TableRead::from_buffer(buffer, offset)?;
                    ::core::result::Result::Ok(::core::convert::TryInto::try_into(n)?)
                }
            }

            impl<'buf> ::planus::VectorReadInner<'buf> for ApprovalDecision {
                type Error = ::planus::errors::UnknownEnumTag;
                const STRIDE: usize = 1;
                #[inline]
                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'buf>,
                    offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::UnknownEnumTag>
                {
                    let value = unsafe { *buffer.buffer.get_unchecked(offset) };
                    let value: ::core::result::Result<Self, _> =
                        ::core::convert::TryInto::try_into(value);
                    value.map_err(|error_kind| {
                        error_kind.with_error_location(
                            "ApprovalDecision",
                            "VectorRead::from_buffer",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<ApprovalDecision> for ApprovalDecision {
                const STRIDE: usize = 1;

                type Value = Self;

                #[inline]
                fn prepare(&self, _builder: &mut ::planus::Builder) -> Self {
                    *self
                }

                #[inline]
                unsafe fn write_values(
                    values: &[Self],
                    bytes: *mut ::core::mem::MaybeUninit<u8>,
                    buffer_position: u32,
                ) {
                    let bytes = bytes as *mut [::core::mem::MaybeUninit<u8>; 1];
                    for (i, v) in ::core::iter::Iterator::enumerate(values.iter()) {
                        ::planus::WriteAsPrimitive::write(
                            v,
                            ::planus::Cursor::new(unsafe { &mut *bytes.add(i) }),
                            buffer_position - i as u32,
                        );
                    }
                }
            }

            /// The enum `EffectState` in the namespace `hypermind.schema`
            ///
            /// Generated from these locations:
            /// * Enum `EffectState` in the file `schemas/events.fbs:5`
            #[derive(
                Copy,
                Clone,
                Debug,
                PartialEq,
                Eq,
                PartialOrd,
                Ord,
                Hash,
                ::serde::Serialize,
                ::serde::Deserialize,
            )]
            #[repr(u8)]
            pub enum EffectState {
                /// The variant `dispatched` in the enum `EffectState`
                Dispatched = 0,

                /// The variant `committed` in the enum `EffectState`
                Committed = 1,

                /// The variant `returned` in the enum `EffectState`
                Returned = 2,

                /// The variant `outcome_unknown` in the enum `EffectState`
                OutcomeUnknown = 3,
            }

            impl EffectState {
                /// Array containing all valid variants of EffectState
                pub const ENUM_VALUES: [Self; 4] = [
                    Self::Dispatched,
                    Self::Committed,
                    Self::Returned,
                    Self::OutcomeUnknown,
                ];
            }

            impl ::core::convert::TryFrom<u8> for EffectState {
                type Error = ::planus::errors::UnknownEnumTagKind;
                #[inline]
                fn try_from(
                    value: u8,
                ) -> ::core::result::Result<Self, ::planus::errors::UnknownEnumTagKind>
                {
                    #[allow(clippy::match_single_binding)]
                    match value {
                        0 => ::core::result::Result::Ok(EffectState::Dispatched),
                        1 => ::core::result::Result::Ok(EffectState::Committed),
                        2 => ::core::result::Result::Ok(EffectState::Returned),
                        3 => ::core::result::Result::Ok(EffectState::OutcomeUnknown),

                        _ => ::core::result::Result::Err(::planus::errors::UnknownEnumTagKind {
                            tag: value as i128,
                        }),
                    }
                }
            }

            impl ::core::convert::From<EffectState> for u8 {
                #[inline]
                fn from(value: EffectState) -> Self {
                    value as u8
                }
            }

            /// # Safety
            /// The Planus compiler correctly calculates `ALIGNMENT` and `SIZE`.
            unsafe impl ::planus::Primitive for EffectState {
                const ALIGNMENT: usize = 1;
                const SIZE: usize = 1;
            }

            impl ::planus::WriteAsPrimitive<EffectState> for EffectState {
                #[inline]
                fn write<const N: usize>(
                    &self,
                    cursor: ::planus::Cursor<'_, N>,
                    buffer_position: u32,
                ) {
                    (*self as u8).write(cursor, buffer_position);
                }
            }

            impl ::planus::WriteAs<EffectState> for EffectState {
                type Prepared = Self;

                #[inline]
                fn prepare(&self, _builder: &mut ::planus::Builder) -> EffectState {
                    *self
                }
            }

            impl ::planus::WriteAsDefault<EffectState, EffectState> for EffectState {
                type Prepared = Self;

                #[inline]
                fn prepare(
                    &self,
                    _builder: &mut ::planus::Builder,
                    default: &EffectState,
                ) -> ::core::option::Option<EffectState> {
                    if self == default {
                        ::core::option::Option::None
                    } else {
                        ::core::option::Option::Some(*self)
                    }
                }
            }

            impl ::planus::WriteAsOptional<EffectState> for EffectState {
                type Prepared = Self;

                #[inline]
                fn prepare(
                    &self,
                    _builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<EffectState> {
                    ::core::option::Option::Some(*self)
                }
            }

            impl<'buf> ::planus::TableRead<'buf> for EffectState {
                #[inline]
                fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'buf>,
                    offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::ErrorKind> {
                    let n: u8 = ::planus::TableRead::from_buffer(buffer, offset)?;
                    ::core::result::Result::Ok(::core::convert::TryInto::try_into(n)?)
                }
            }

            impl<'buf> ::planus::VectorReadInner<'buf> for EffectState {
                type Error = ::planus::errors::UnknownEnumTag;
                const STRIDE: usize = 1;
                #[inline]
                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'buf>,
                    offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::UnknownEnumTag>
                {
                    let value = unsafe { *buffer.buffer.get_unchecked(offset) };
                    let value: ::core::result::Result<Self, _> =
                        ::core::convert::TryInto::try_into(value);
                    value.map_err(|error_kind| {
                        error_kind.with_error_location(
                            "EffectState",
                            "VectorRead::from_buffer",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<EffectState> for EffectState {
                const STRIDE: usize = 1;

                type Value = Self;

                #[inline]
                fn prepare(&self, _builder: &mut ::planus::Builder) -> Self {
                    *self
                }

                #[inline]
                unsafe fn write_values(
                    values: &[Self],
                    bytes: *mut ::core::mem::MaybeUninit<u8>,
                    buffer_position: u32,
                ) {
                    let bytes = bytes as *mut [::core::mem::MaybeUninit<u8>; 1];
                    for (i, v) in ::core::iter::Iterator::enumerate(values.iter()) {
                        ::planus::WriteAsPrimitive::write(
                            v,
                            ::planus::Cursor::new(unsafe { &mut *bytes.add(i) }),
                            buffer_position - i as u32,
                        );
                    }
                }
            }

            /// The enum `LoopCloseReason` in the namespace `hypermind.schema`
            ///
            /// Generated from these locations:
            /// * Enum `LoopCloseReason` in the file `schemas/events.fbs:6`
            #[derive(
                Copy,
                Clone,
                Debug,
                PartialEq,
                Eq,
                PartialOrd,
                Ord,
                Hash,
                ::serde::Serialize,
                ::serde::Deserialize,
            )]
            #[repr(u8)]
            pub enum LoopCloseReason {
                /// The variant `done` in the enum `LoopCloseReason`
                Done = 0,

                /// The variant `abandoned` in the enum `LoopCloseReason`
                Abandoned = 1,

                /// The variant `handed_off` in the enum `LoopCloseReason`
                HandedOff = 2,

                /// The variant `superseded` in the enum `LoopCloseReason`
                Superseded = 3,
            }

            impl LoopCloseReason {
                /// Array containing all valid variants of LoopCloseReason
                pub const ENUM_VALUES: [Self; 4] = [
                    Self::Done,
                    Self::Abandoned,
                    Self::HandedOff,
                    Self::Superseded,
                ];
            }

            impl ::core::convert::TryFrom<u8> for LoopCloseReason {
                type Error = ::planus::errors::UnknownEnumTagKind;
                #[inline]
                fn try_from(
                    value: u8,
                ) -> ::core::result::Result<Self, ::planus::errors::UnknownEnumTagKind>
                {
                    #[allow(clippy::match_single_binding)]
                    match value {
                        0 => ::core::result::Result::Ok(LoopCloseReason::Done),
                        1 => ::core::result::Result::Ok(LoopCloseReason::Abandoned),
                        2 => ::core::result::Result::Ok(LoopCloseReason::HandedOff),
                        3 => ::core::result::Result::Ok(LoopCloseReason::Superseded),

                        _ => ::core::result::Result::Err(::planus::errors::UnknownEnumTagKind {
                            tag: value as i128,
                        }),
                    }
                }
            }

            impl ::core::convert::From<LoopCloseReason> for u8 {
                #[inline]
                fn from(value: LoopCloseReason) -> Self {
                    value as u8
                }
            }

            /// # Safety
            /// The Planus compiler correctly calculates `ALIGNMENT` and `SIZE`.
            unsafe impl ::planus::Primitive for LoopCloseReason {
                const ALIGNMENT: usize = 1;
                const SIZE: usize = 1;
            }

            impl ::planus::WriteAsPrimitive<LoopCloseReason> for LoopCloseReason {
                #[inline]
                fn write<const N: usize>(
                    &self,
                    cursor: ::planus::Cursor<'_, N>,
                    buffer_position: u32,
                ) {
                    (*self as u8).write(cursor, buffer_position);
                }
            }

            impl ::planus::WriteAs<LoopCloseReason> for LoopCloseReason {
                type Prepared = Self;

                #[inline]
                fn prepare(&self, _builder: &mut ::planus::Builder) -> LoopCloseReason {
                    *self
                }
            }

            impl ::planus::WriteAsDefault<LoopCloseReason, LoopCloseReason> for LoopCloseReason {
                type Prepared = Self;

                #[inline]
                fn prepare(
                    &self,
                    _builder: &mut ::planus::Builder,
                    default: &LoopCloseReason,
                ) -> ::core::option::Option<LoopCloseReason> {
                    if self == default {
                        ::core::option::Option::None
                    } else {
                        ::core::option::Option::Some(*self)
                    }
                }
            }

            impl ::planus::WriteAsOptional<LoopCloseReason> for LoopCloseReason {
                type Prepared = Self;

                #[inline]
                fn prepare(
                    &self,
                    _builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<LoopCloseReason> {
                    ::core::option::Option::Some(*self)
                }
            }

            impl<'buf> ::planus::TableRead<'buf> for LoopCloseReason {
                #[inline]
                fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'buf>,
                    offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::ErrorKind> {
                    let n: u8 = ::planus::TableRead::from_buffer(buffer, offset)?;
                    ::core::result::Result::Ok(::core::convert::TryInto::try_into(n)?)
                }
            }

            impl<'buf> ::planus::VectorReadInner<'buf> for LoopCloseReason {
                type Error = ::planus::errors::UnknownEnumTag;
                const STRIDE: usize = 1;
                #[inline]
                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'buf>,
                    offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::UnknownEnumTag>
                {
                    let value = unsafe { *buffer.buffer.get_unchecked(offset) };
                    let value: ::core::result::Result<Self, _> =
                        ::core::convert::TryInto::try_into(value);
                    value.map_err(|error_kind| {
                        error_kind.with_error_location(
                            "LoopCloseReason",
                            "VectorRead::from_buffer",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<LoopCloseReason> for LoopCloseReason {
                const STRIDE: usize = 1;

                type Value = Self;

                #[inline]
                fn prepare(&self, _builder: &mut ::planus::Builder) -> Self {
                    *self
                }

                #[inline]
                unsafe fn write_values(
                    values: &[Self],
                    bytes: *mut ::core::mem::MaybeUninit<u8>,
                    buffer_position: u32,
                ) {
                    let bytes = bytes as *mut [::core::mem::MaybeUninit<u8>; 1];
                    for (i, v) in ::core::iter::Iterator::enumerate(values.iter()) {
                        ::planus::WriteAsPrimitive::write(
                            v,
                            ::planus::Cursor::new(unsafe { &mut *bytes.add(i) }),
                            buffer_position - i as u32,
                        );
                    }
                }
            }

            /// The enum `BeliefType` in the namespace `hypermind.schema`
            ///
            /// Generated from these locations:
            /// * Enum `BeliefType` in the file `schemas/events.fbs:7`
            #[derive(
                Copy,
                Clone,
                Debug,
                PartialEq,
                Eq,
                PartialOrd,
                Ord,
                Hash,
                ::serde::Serialize,
                ::serde::Deserialize,
            )]
            #[repr(u8)]
            pub enum BeliefType {
                /// The variant `fact` in the enum `BeliefType`
                Fact = 0,

                /// The variant `preference` in the enum `BeliefType`
                Preference = 1,

                /// The variant `constraint` in the enum `BeliefType`
                Constraint = 2,

                /// The variant `goal` in the enum `BeliefType`
                Goal = 3,

                /// The variant `identity` in the enum `BeliefType`
                Identity = 4,
            }

            impl BeliefType {
                /// Array containing all valid variants of BeliefType
                pub const ENUM_VALUES: [Self; 5] = [
                    Self::Fact,
                    Self::Preference,
                    Self::Constraint,
                    Self::Goal,
                    Self::Identity,
                ];
            }

            impl ::core::convert::TryFrom<u8> for BeliefType {
                type Error = ::planus::errors::UnknownEnumTagKind;
                #[inline]
                fn try_from(
                    value: u8,
                ) -> ::core::result::Result<Self, ::planus::errors::UnknownEnumTagKind>
                {
                    #[allow(clippy::match_single_binding)]
                    match value {
                        0 => ::core::result::Result::Ok(BeliefType::Fact),
                        1 => ::core::result::Result::Ok(BeliefType::Preference),
                        2 => ::core::result::Result::Ok(BeliefType::Constraint),
                        3 => ::core::result::Result::Ok(BeliefType::Goal),
                        4 => ::core::result::Result::Ok(BeliefType::Identity),

                        _ => ::core::result::Result::Err(::planus::errors::UnknownEnumTagKind {
                            tag: value as i128,
                        }),
                    }
                }
            }

            impl ::core::convert::From<BeliefType> for u8 {
                #[inline]
                fn from(value: BeliefType) -> Self {
                    value as u8
                }
            }

            /// # Safety
            /// The Planus compiler correctly calculates `ALIGNMENT` and `SIZE`.
            unsafe impl ::planus::Primitive for BeliefType {
                const ALIGNMENT: usize = 1;
                const SIZE: usize = 1;
            }

            impl ::planus::WriteAsPrimitive<BeliefType> for BeliefType {
                #[inline]
                fn write<const N: usize>(
                    &self,
                    cursor: ::planus::Cursor<'_, N>,
                    buffer_position: u32,
                ) {
                    (*self as u8).write(cursor, buffer_position);
                }
            }

            impl ::planus::WriteAs<BeliefType> for BeliefType {
                type Prepared = Self;

                #[inline]
                fn prepare(&self, _builder: &mut ::planus::Builder) -> BeliefType {
                    *self
                }
            }

            impl ::planus::WriteAsDefault<BeliefType, BeliefType> for BeliefType {
                type Prepared = Self;

                #[inline]
                fn prepare(
                    &self,
                    _builder: &mut ::planus::Builder,
                    default: &BeliefType,
                ) -> ::core::option::Option<BeliefType> {
                    if self == default {
                        ::core::option::Option::None
                    } else {
                        ::core::option::Option::Some(*self)
                    }
                }
            }

            impl ::planus::WriteAsOptional<BeliefType> for BeliefType {
                type Prepared = Self;

                #[inline]
                fn prepare(
                    &self,
                    _builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<BeliefType> {
                    ::core::option::Option::Some(*self)
                }
            }

            impl<'buf> ::planus::TableRead<'buf> for BeliefType {
                #[inline]
                fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'buf>,
                    offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::ErrorKind> {
                    let n: u8 = ::planus::TableRead::from_buffer(buffer, offset)?;
                    ::core::result::Result::Ok(::core::convert::TryInto::try_into(n)?)
                }
            }

            impl<'buf> ::planus::VectorReadInner<'buf> for BeliefType {
                type Error = ::planus::errors::UnknownEnumTag;
                const STRIDE: usize = 1;
                #[inline]
                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'buf>,
                    offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::UnknownEnumTag>
                {
                    let value = unsafe { *buffer.buffer.get_unchecked(offset) };
                    let value: ::core::result::Result<Self, _> =
                        ::core::convert::TryInto::try_into(value);
                    value.map_err(|error_kind| {
                        error_kind.with_error_location(
                            "BeliefType",
                            "VectorRead::from_buffer",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<BeliefType> for BeliefType {
                const STRIDE: usize = 1;

                type Value = Self;

                #[inline]
                fn prepare(&self, _builder: &mut ::planus::Builder) -> Self {
                    *self
                }

                #[inline]
                unsafe fn write_values(
                    values: &[Self],
                    bytes: *mut ::core::mem::MaybeUninit<u8>,
                    buffer_position: u32,
                ) {
                    let bytes = bytes as *mut [::core::mem::MaybeUninit<u8>; 1];
                    for (i, v) in ::core::iter::Iterator::enumerate(values.iter()) {
                        ::planus::WriteAsPrimitive::write(
                            v,
                            ::planus::Cursor::new(unsafe { &mut *bytes.add(i) }),
                            buffer_position - i as u32,
                        );
                    }
                }
            }

            /// The enum `AssertionClaim` in the namespace `hypermind.schema`
            ///
            /// Generated from these locations:
            /// * Enum `AssertionClaim` in the file `schemas/events.fbs:8`
            #[derive(
                Copy,
                Clone,
                Debug,
                PartialEq,
                Eq,
                PartialOrd,
                Ord,
                Hash,
                ::serde::Serialize,
                ::serde::Deserialize,
            )]
            #[repr(u8)]
            pub enum AssertionClaim {
                /// The variant `affirmative` in the enum `AssertionClaim`
                Affirmative = 0,

                /// The variant `negative_existence` in the enum `AssertionClaim`
                NegativeExistence = 1,
            }

            impl AssertionClaim {
                /// Array containing all valid variants of AssertionClaim
                pub const ENUM_VALUES: [Self; 2] = [Self::Affirmative, Self::NegativeExistence];
            }

            impl ::core::convert::TryFrom<u8> for AssertionClaim {
                type Error = ::planus::errors::UnknownEnumTagKind;
                #[inline]
                fn try_from(
                    value: u8,
                ) -> ::core::result::Result<Self, ::planus::errors::UnknownEnumTagKind>
                {
                    #[allow(clippy::match_single_binding)]
                    match value {
                        0 => ::core::result::Result::Ok(AssertionClaim::Affirmative),
                        1 => ::core::result::Result::Ok(AssertionClaim::NegativeExistence),

                        _ => ::core::result::Result::Err(::planus::errors::UnknownEnumTagKind {
                            tag: value as i128,
                        }),
                    }
                }
            }

            impl ::core::convert::From<AssertionClaim> for u8 {
                #[inline]
                fn from(value: AssertionClaim) -> Self {
                    value as u8
                }
            }

            /// # Safety
            /// The Planus compiler correctly calculates `ALIGNMENT` and `SIZE`.
            unsafe impl ::planus::Primitive for AssertionClaim {
                const ALIGNMENT: usize = 1;
                const SIZE: usize = 1;
            }

            impl ::planus::WriteAsPrimitive<AssertionClaim> for AssertionClaim {
                #[inline]
                fn write<const N: usize>(
                    &self,
                    cursor: ::planus::Cursor<'_, N>,
                    buffer_position: u32,
                ) {
                    (*self as u8).write(cursor, buffer_position);
                }
            }

            impl ::planus::WriteAs<AssertionClaim> for AssertionClaim {
                type Prepared = Self;

                #[inline]
                fn prepare(&self, _builder: &mut ::planus::Builder) -> AssertionClaim {
                    *self
                }
            }

            impl ::planus::WriteAsDefault<AssertionClaim, AssertionClaim> for AssertionClaim {
                type Prepared = Self;

                #[inline]
                fn prepare(
                    &self,
                    _builder: &mut ::planus::Builder,
                    default: &AssertionClaim,
                ) -> ::core::option::Option<AssertionClaim> {
                    if self == default {
                        ::core::option::Option::None
                    } else {
                        ::core::option::Option::Some(*self)
                    }
                }
            }

            impl ::planus::WriteAsOptional<AssertionClaim> for AssertionClaim {
                type Prepared = Self;

                #[inline]
                fn prepare(
                    &self,
                    _builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<AssertionClaim> {
                    ::core::option::Option::Some(*self)
                }
            }

            impl<'buf> ::planus::TableRead<'buf> for AssertionClaim {
                #[inline]
                fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'buf>,
                    offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::ErrorKind> {
                    let n: u8 = ::planus::TableRead::from_buffer(buffer, offset)?;
                    ::core::result::Result::Ok(::core::convert::TryInto::try_into(n)?)
                }
            }

            impl<'buf> ::planus::VectorReadInner<'buf> for AssertionClaim {
                type Error = ::planus::errors::UnknownEnumTag;
                const STRIDE: usize = 1;
                #[inline]
                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'buf>,
                    offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::UnknownEnumTag>
                {
                    let value = unsafe { *buffer.buffer.get_unchecked(offset) };
                    let value: ::core::result::Result<Self, _> =
                        ::core::convert::TryInto::try_into(value);
                    value.map_err(|error_kind| {
                        error_kind.with_error_location(
                            "AssertionClaim",
                            "VectorRead::from_buffer",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<AssertionClaim> for AssertionClaim {
                const STRIDE: usize = 1;

                type Value = Self;

                #[inline]
                fn prepare(&self, _builder: &mut ::planus::Builder) -> Self {
                    *self
                }

                #[inline]
                unsafe fn write_values(
                    values: &[Self],
                    bytes: *mut ::core::mem::MaybeUninit<u8>,
                    buffer_position: u32,
                ) {
                    let bytes = bytes as *mut [::core::mem::MaybeUninit<u8>; 1];
                    for (i, v) in ::core::iter::Iterator::enumerate(values.iter()) {
                        ::planus::WriteAsPrimitive::write(
                            v,
                            ::planus::Cursor::new(unsafe { &mut *bytes.add(i) }),
                            buffer_position - i as u32,
                        );
                    }
                }
            }

            /// The enum `AttestationDisposition` in the namespace `hypermind.schema`
            ///
            /// Generated from these locations:
            /// * Enum `AttestationDisposition` in the file `schemas/events.fbs:9`
            #[derive(
                Copy,
                Clone,
                Debug,
                PartialEq,
                Eq,
                PartialOrd,
                Ord,
                Hash,
                ::serde::Serialize,
                ::serde::Deserialize,
            )]
            #[repr(u8)]
            pub enum AttestationDisposition {
                /// The variant `used` in the enum `AttestationDisposition`
                Used = 0,

                /// The variant `ignored` in the enum `AttestationDisposition`
                Ignored = 1,
            }

            impl AttestationDisposition {
                /// Array containing all valid variants of AttestationDisposition
                pub const ENUM_VALUES: [Self; 2] = [Self::Used, Self::Ignored];
            }

            impl ::core::convert::TryFrom<u8> for AttestationDisposition {
                type Error = ::planus::errors::UnknownEnumTagKind;
                #[inline]
                fn try_from(
                    value: u8,
                ) -> ::core::result::Result<Self, ::planus::errors::UnknownEnumTagKind>
                {
                    #[allow(clippy::match_single_binding)]
                    match value {
                        0 => ::core::result::Result::Ok(AttestationDisposition::Used),
                        1 => ::core::result::Result::Ok(AttestationDisposition::Ignored),

                        _ => ::core::result::Result::Err(::planus::errors::UnknownEnumTagKind {
                            tag: value as i128,
                        }),
                    }
                }
            }

            impl ::core::convert::From<AttestationDisposition> for u8 {
                #[inline]
                fn from(value: AttestationDisposition) -> Self {
                    value as u8
                }
            }

            /// # Safety
            /// The Planus compiler correctly calculates `ALIGNMENT` and `SIZE`.
            unsafe impl ::planus::Primitive for AttestationDisposition {
                const ALIGNMENT: usize = 1;
                const SIZE: usize = 1;
            }

            impl ::planus::WriteAsPrimitive<AttestationDisposition> for AttestationDisposition {
                #[inline]
                fn write<const N: usize>(
                    &self,
                    cursor: ::planus::Cursor<'_, N>,
                    buffer_position: u32,
                ) {
                    (*self as u8).write(cursor, buffer_position);
                }
            }

            impl ::planus::WriteAs<AttestationDisposition> for AttestationDisposition {
                type Prepared = Self;

                #[inline]
                fn prepare(&self, _builder: &mut ::planus::Builder) -> AttestationDisposition {
                    *self
                }
            }

            impl ::planus::WriteAsDefault<AttestationDisposition, AttestationDisposition>
                for AttestationDisposition
            {
                type Prepared = Self;

                #[inline]
                fn prepare(
                    &self,
                    _builder: &mut ::planus::Builder,
                    default: &AttestationDisposition,
                ) -> ::core::option::Option<AttestationDisposition> {
                    if self == default {
                        ::core::option::Option::None
                    } else {
                        ::core::option::Option::Some(*self)
                    }
                }
            }

            impl ::planus::WriteAsOptional<AttestationDisposition> for AttestationDisposition {
                type Prepared = Self;

                #[inline]
                fn prepare(
                    &self,
                    _builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<AttestationDisposition> {
                    ::core::option::Option::Some(*self)
                }
            }

            impl<'buf> ::planus::TableRead<'buf> for AttestationDisposition {
                #[inline]
                fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'buf>,
                    offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::ErrorKind> {
                    let n: u8 = ::planus::TableRead::from_buffer(buffer, offset)?;
                    ::core::result::Result::Ok(::core::convert::TryInto::try_into(n)?)
                }
            }

            impl<'buf> ::planus::VectorReadInner<'buf> for AttestationDisposition {
                type Error = ::planus::errors::UnknownEnumTag;
                const STRIDE: usize = 1;
                #[inline]
                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'buf>,
                    offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::UnknownEnumTag>
                {
                    let value = unsafe { *buffer.buffer.get_unchecked(offset) };
                    let value: ::core::result::Result<Self, _> =
                        ::core::convert::TryInto::try_into(value);
                    value.map_err(|error_kind| {
                        error_kind.with_error_location(
                            "AttestationDisposition",
                            "VectorRead::from_buffer",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<AttestationDisposition> for AttestationDisposition {
                const STRIDE: usize = 1;

                type Value = Self;

                #[inline]
                fn prepare(&self, _builder: &mut ::planus::Builder) -> Self {
                    *self
                }

                #[inline]
                unsafe fn write_values(
                    values: &[Self],
                    bytes: *mut ::core::mem::MaybeUninit<u8>,
                    buffer_position: u32,
                ) {
                    let bytes = bytes as *mut [::core::mem::MaybeUninit<u8>; 1];
                    for (i, v) in ::core::iter::Iterator::enumerate(values.iter()) {
                        ::planus::WriteAsPrimitive::write(
                            v,
                            ::planus::Cursor::new(unsafe { &mut *bytes.add(i) }),
                            buffer_position - i as u32,
                        );
                    }
                }
            }

            /// The enum `Authority` in the namespace `hypermind.schema`
            ///
            /// Generated from these locations:
            /// * Enum `Authority` in the file `schemas/events.fbs:10`
            #[derive(
                Copy,
                Clone,
                Debug,
                PartialEq,
                Eq,
                PartialOrd,
                Ord,
                Hash,
                ::serde::Serialize,
                ::serde::Deserialize,
            )]
            #[repr(u8)]
            pub enum Authority {
                /// The variant `user_asserted` in the enum `Authority`
                UserAsserted = 0,

                /// The variant `external_observed` in the enum `Authority`
                ExternalObserved = 1,

                /// The variant `tool_observed` in the enum `Authority`
                ToolObserved = 2,

                /// The variant `runtime_fact` in the enum `Authority`
                RuntimeFact = 3,

                /// The variant `assistant_generated` in the enum `Authority`
                AssistantGenerated = 4,

                /// The variant `derived_inference` in the enum `Authority`
                DerivedInference = 5,
            }

            impl Authority {
                /// Array containing all valid variants of Authority
                pub const ENUM_VALUES: [Self; 6] = [
                    Self::UserAsserted,
                    Self::ExternalObserved,
                    Self::ToolObserved,
                    Self::RuntimeFact,
                    Self::AssistantGenerated,
                    Self::DerivedInference,
                ];
            }

            impl ::core::convert::TryFrom<u8> for Authority {
                type Error = ::planus::errors::UnknownEnumTagKind;
                #[inline]
                fn try_from(
                    value: u8,
                ) -> ::core::result::Result<Self, ::planus::errors::UnknownEnumTagKind>
                {
                    #[allow(clippy::match_single_binding)]
                    match value {
                        0 => ::core::result::Result::Ok(Authority::UserAsserted),
                        1 => ::core::result::Result::Ok(Authority::ExternalObserved),
                        2 => ::core::result::Result::Ok(Authority::ToolObserved),
                        3 => ::core::result::Result::Ok(Authority::RuntimeFact),
                        4 => ::core::result::Result::Ok(Authority::AssistantGenerated),
                        5 => ::core::result::Result::Ok(Authority::DerivedInference),

                        _ => ::core::result::Result::Err(::planus::errors::UnknownEnumTagKind {
                            tag: value as i128,
                        }),
                    }
                }
            }

            impl ::core::convert::From<Authority> for u8 {
                #[inline]
                fn from(value: Authority) -> Self {
                    value as u8
                }
            }

            /// # Safety
            /// The Planus compiler correctly calculates `ALIGNMENT` and `SIZE`.
            unsafe impl ::planus::Primitive for Authority {
                const ALIGNMENT: usize = 1;
                const SIZE: usize = 1;
            }

            impl ::planus::WriteAsPrimitive<Authority> for Authority {
                #[inline]
                fn write<const N: usize>(
                    &self,
                    cursor: ::planus::Cursor<'_, N>,
                    buffer_position: u32,
                ) {
                    (*self as u8).write(cursor, buffer_position);
                }
            }

            impl ::planus::WriteAs<Authority> for Authority {
                type Prepared = Self;

                #[inline]
                fn prepare(&self, _builder: &mut ::planus::Builder) -> Authority {
                    *self
                }
            }

            impl ::planus::WriteAsDefault<Authority, Authority> for Authority {
                type Prepared = Self;

                #[inline]
                fn prepare(
                    &self,
                    _builder: &mut ::planus::Builder,
                    default: &Authority,
                ) -> ::core::option::Option<Authority> {
                    if self == default {
                        ::core::option::Option::None
                    } else {
                        ::core::option::Option::Some(*self)
                    }
                }
            }

            impl ::planus::WriteAsOptional<Authority> for Authority {
                type Prepared = Self;

                #[inline]
                fn prepare(
                    &self,
                    _builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<Authority> {
                    ::core::option::Option::Some(*self)
                }
            }

            impl<'buf> ::planus::TableRead<'buf> for Authority {
                #[inline]
                fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'buf>,
                    offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::ErrorKind> {
                    let n: u8 = ::planus::TableRead::from_buffer(buffer, offset)?;
                    ::core::result::Result::Ok(::core::convert::TryInto::try_into(n)?)
                }
            }

            impl<'buf> ::planus::VectorReadInner<'buf> for Authority {
                type Error = ::planus::errors::UnknownEnumTag;
                const STRIDE: usize = 1;
                #[inline]
                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'buf>,
                    offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::UnknownEnumTag>
                {
                    let value = unsafe { *buffer.buffer.get_unchecked(offset) };
                    let value: ::core::result::Result<Self, _> =
                        ::core::convert::TryInto::try_into(value);
                    value.map_err(|error_kind| {
                        error_kind.with_error_location(
                            "Authority",
                            "VectorRead::from_buffer",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<Authority> for Authority {
                const STRIDE: usize = 1;

                type Value = Self;

                #[inline]
                fn prepare(&self, _builder: &mut ::planus::Builder) -> Self {
                    *self
                }

                #[inline]
                unsafe fn write_values(
                    values: &[Self],
                    bytes: *mut ::core::mem::MaybeUninit<u8>,
                    buffer_position: u32,
                ) {
                    let bytes = bytes as *mut [::core::mem::MaybeUninit<u8>; 1];
                    for (i, v) in ::core::iter::Iterator::enumerate(values.iter()) {
                        ::planus::WriteAsPrimitive::write(
                            v,
                            ::planus::Cursor::new(unsafe { &mut *bytes.add(i) }),
                            buffer_position - i as u32,
                        );
                    }
                }
            }

            /// The enum `Retention` in the namespace `hypermind.schema`
            ///
            /// Generated from these locations:
            /// * Enum `Retention` in the file `schemas/events.fbs:18`
            #[derive(
                Copy,
                Clone,
                Debug,
                PartialEq,
                Eq,
                PartialOrd,
                Ord,
                Hash,
                ::serde::Serialize,
                ::serde::Deserialize,
            )]
            #[repr(u8)]
            pub enum Retention {
                /// The variant `current_state` in the enum `Retention`
                CurrentState = 0,

                /// The variant `daily` in the enum `Retention`
                Daily = 1,

                /// The variant `durable` in the enum `Retention`
                Durable = 2,

                /// The variant `do_not_store` in the enum `Retention`
                DoNotStore = 3,
            }

            impl Retention {
                /// Array containing all valid variants of Retention
                pub const ENUM_VALUES: [Self; 4] = [
                    Self::CurrentState,
                    Self::Daily,
                    Self::Durable,
                    Self::DoNotStore,
                ];
            }

            impl ::core::convert::TryFrom<u8> for Retention {
                type Error = ::planus::errors::UnknownEnumTagKind;
                #[inline]
                fn try_from(
                    value: u8,
                ) -> ::core::result::Result<Self, ::planus::errors::UnknownEnumTagKind>
                {
                    #[allow(clippy::match_single_binding)]
                    match value {
                        0 => ::core::result::Result::Ok(Retention::CurrentState),
                        1 => ::core::result::Result::Ok(Retention::Daily),
                        2 => ::core::result::Result::Ok(Retention::Durable),
                        3 => ::core::result::Result::Ok(Retention::DoNotStore),

                        _ => ::core::result::Result::Err(::planus::errors::UnknownEnumTagKind {
                            tag: value as i128,
                        }),
                    }
                }
            }

            impl ::core::convert::From<Retention> for u8 {
                #[inline]
                fn from(value: Retention) -> Self {
                    value as u8
                }
            }

            /// # Safety
            /// The Planus compiler correctly calculates `ALIGNMENT` and `SIZE`.
            unsafe impl ::planus::Primitive for Retention {
                const ALIGNMENT: usize = 1;
                const SIZE: usize = 1;
            }

            impl ::planus::WriteAsPrimitive<Retention> for Retention {
                #[inline]
                fn write<const N: usize>(
                    &self,
                    cursor: ::planus::Cursor<'_, N>,
                    buffer_position: u32,
                ) {
                    (*self as u8).write(cursor, buffer_position);
                }
            }

            impl ::planus::WriteAs<Retention> for Retention {
                type Prepared = Self;

                #[inline]
                fn prepare(&self, _builder: &mut ::planus::Builder) -> Retention {
                    *self
                }
            }

            impl ::planus::WriteAsDefault<Retention, Retention> for Retention {
                type Prepared = Self;

                #[inline]
                fn prepare(
                    &self,
                    _builder: &mut ::planus::Builder,
                    default: &Retention,
                ) -> ::core::option::Option<Retention> {
                    if self == default {
                        ::core::option::Option::None
                    } else {
                        ::core::option::Option::Some(*self)
                    }
                }
            }

            impl ::planus::WriteAsOptional<Retention> for Retention {
                type Prepared = Self;

                #[inline]
                fn prepare(
                    &self,
                    _builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<Retention> {
                    ::core::option::Option::Some(*self)
                }
            }

            impl<'buf> ::planus::TableRead<'buf> for Retention {
                #[inline]
                fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'buf>,
                    offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::ErrorKind> {
                    let n: u8 = ::planus::TableRead::from_buffer(buffer, offset)?;
                    ::core::result::Result::Ok(::core::convert::TryInto::try_into(n)?)
                }
            }

            impl<'buf> ::planus::VectorReadInner<'buf> for Retention {
                type Error = ::planus::errors::UnknownEnumTag;
                const STRIDE: usize = 1;
                #[inline]
                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'buf>,
                    offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::UnknownEnumTag>
                {
                    let value = unsafe { *buffer.buffer.get_unchecked(offset) };
                    let value: ::core::result::Result<Self, _> =
                        ::core::convert::TryInto::try_into(value);
                    value.map_err(|error_kind| {
                        error_kind.with_error_location(
                            "Retention",
                            "VectorRead::from_buffer",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<Retention> for Retention {
                const STRIDE: usize = 1;

                type Value = Self;

                #[inline]
                fn prepare(&self, _builder: &mut ::planus::Builder) -> Self {
                    *self
                }

                #[inline]
                unsafe fn write_values(
                    values: &[Self],
                    bytes: *mut ::core::mem::MaybeUninit<u8>,
                    buffer_position: u32,
                ) {
                    let bytes = bytes as *mut [::core::mem::MaybeUninit<u8>; 1];
                    for (i, v) in ::core::iter::Iterator::enumerate(values.iter()) {
                        ::planus::WriteAsPrimitive::write(
                            v,
                            ::planus::Cursor::new(unsafe { &mut *bytes.add(i) }),
                            buffer_position - i as u32,
                        );
                    }
                }
            }

            /// The enum `Sensitivity` in the namespace `hypermind.schema`
            ///
            /// Generated from these locations:
            /// * Enum `Sensitivity` in the file `schemas/events.fbs:19`
            #[derive(
                Copy,
                Clone,
                Debug,
                PartialEq,
                Eq,
                PartialOrd,
                Ord,
                Hash,
                ::serde::Serialize,
                ::serde::Deserialize,
            )]
            #[repr(u8)]
            pub enum Sensitivity {
                /// The variant `public` in the enum `Sensitivity`
                Public = 0,

                /// The variant `personal` in the enum `Sensitivity`
                Personal = 1,

                /// The variant `secret` in the enum `Sensitivity`
                Secret = 2,
            }

            impl Sensitivity {
                /// Array containing all valid variants of Sensitivity
                pub const ENUM_VALUES: [Self; 3] = [Self::Public, Self::Personal, Self::Secret];
            }

            impl ::core::convert::TryFrom<u8> for Sensitivity {
                type Error = ::planus::errors::UnknownEnumTagKind;
                #[inline]
                fn try_from(
                    value: u8,
                ) -> ::core::result::Result<Self, ::planus::errors::UnknownEnumTagKind>
                {
                    #[allow(clippy::match_single_binding)]
                    match value {
                        0 => ::core::result::Result::Ok(Sensitivity::Public),
                        1 => ::core::result::Result::Ok(Sensitivity::Personal),
                        2 => ::core::result::Result::Ok(Sensitivity::Secret),

                        _ => ::core::result::Result::Err(::planus::errors::UnknownEnumTagKind {
                            tag: value as i128,
                        }),
                    }
                }
            }

            impl ::core::convert::From<Sensitivity> for u8 {
                #[inline]
                fn from(value: Sensitivity) -> Self {
                    value as u8
                }
            }

            /// # Safety
            /// The Planus compiler correctly calculates `ALIGNMENT` and `SIZE`.
            unsafe impl ::planus::Primitive for Sensitivity {
                const ALIGNMENT: usize = 1;
                const SIZE: usize = 1;
            }

            impl ::planus::WriteAsPrimitive<Sensitivity> for Sensitivity {
                #[inline]
                fn write<const N: usize>(
                    &self,
                    cursor: ::planus::Cursor<'_, N>,
                    buffer_position: u32,
                ) {
                    (*self as u8).write(cursor, buffer_position);
                }
            }

            impl ::planus::WriteAs<Sensitivity> for Sensitivity {
                type Prepared = Self;

                #[inline]
                fn prepare(&self, _builder: &mut ::planus::Builder) -> Sensitivity {
                    *self
                }
            }

            impl ::planus::WriteAsDefault<Sensitivity, Sensitivity> for Sensitivity {
                type Prepared = Self;

                #[inline]
                fn prepare(
                    &self,
                    _builder: &mut ::planus::Builder,
                    default: &Sensitivity,
                ) -> ::core::option::Option<Sensitivity> {
                    if self == default {
                        ::core::option::Option::None
                    } else {
                        ::core::option::Option::Some(*self)
                    }
                }
            }

            impl ::planus::WriteAsOptional<Sensitivity> for Sensitivity {
                type Prepared = Self;

                #[inline]
                fn prepare(
                    &self,
                    _builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<Sensitivity> {
                    ::core::option::Option::Some(*self)
                }
            }

            impl<'buf> ::planus::TableRead<'buf> for Sensitivity {
                #[inline]
                fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'buf>,
                    offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::ErrorKind> {
                    let n: u8 = ::planus::TableRead::from_buffer(buffer, offset)?;
                    ::core::result::Result::Ok(::core::convert::TryInto::try_into(n)?)
                }
            }

            impl<'buf> ::planus::VectorReadInner<'buf> for Sensitivity {
                type Error = ::planus::errors::UnknownEnumTag;
                const STRIDE: usize = 1;
                #[inline]
                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'buf>,
                    offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::UnknownEnumTag>
                {
                    let value = unsafe { *buffer.buffer.get_unchecked(offset) };
                    let value: ::core::result::Result<Self, _> =
                        ::core::convert::TryInto::try_into(value);
                    value.map_err(|error_kind| {
                        error_kind.with_error_location(
                            "Sensitivity",
                            "VectorRead::from_buffer",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<Sensitivity> for Sensitivity {
                const STRIDE: usize = 1;

                type Value = Self;

                #[inline]
                fn prepare(&self, _builder: &mut ::planus::Builder) -> Self {
                    *self
                }

                #[inline]
                unsafe fn write_values(
                    values: &[Self],
                    bytes: *mut ::core::mem::MaybeUninit<u8>,
                    buffer_position: u32,
                ) {
                    let bytes = bytes as *mut [::core::mem::MaybeUninit<u8>; 1];
                    for (i, v) in ::core::iter::Iterator::enumerate(values.iter()) {
                        ::planus::WriteAsPrimitive::write(
                            v,
                            ::planus::Cursor::new(unsafe { &mut *bytes.add(i) }),
                            buffer_position - i as u32,
                        );
                    }
                }
            }

            /// The struct `ProvenanceRange` in the namespace `hypermind.schema`
            ///
            /// Generated from these locations:
            /// * Struct `ProvenanceRange` in the file `schemas/events.fbs:21`
            #[derive(
                Copy,
                Clone,
                Debug,
                PartialEq,
                PartialOrd,
                Eq,
                Ord,
                Hash,
                Default,
                ::serde::Serialize,
                ::serde::Deserialize,
            )]
            pub struct ProvenanceRange {
                /// The field `first_lsn` in the struct `ProvenanceRange`
                pub first_lsn: u64,

                /// The field `last_lsn` in the struct `ProvenanceRange`
                pub last_lsn: u64,

                /// The field `byte_start` in the struct `ProvenanceRange`
                pub byte_start: u32,

                /// The field `byte_end` in the struct `ProvenanceRange`
                pub byte_end: u32,
            }

            /// # Safety
            /// The Planus compiler correctly calculates `ALIGNMENT` and `SIZE`.
            unsafe impl ::planus::Primitive for ProvenanceRange {
                const ALIGNMENT: usize = 8;
                const SIZE: usize = 24;
            }

            #[allow(clippy::identity_op)]
            impl ::planus::WriteAsPrimitive<ProvenanceRange> for ProvenanceRange {
                #[inline]
                fn write<const N: usize>(
                    &self,
                    cursor: ::planus::Cursor<'_, N>,
                    buffer_position: u32,
                ) {
                    let (cur, cursor) = cursor.split::<8, 16>();
                    self.first_lsn.write(cur, buffer_position - 0);
                    let (cur, cursor) = cursor.split::<8, 8>();
                    self.last_lsn.write(cur, buffer_position - 8);
                    let (cur, cursor) = cursor.split::<4, 4>();
                    self.byte_start.write(cur, buffer_position - 16);
                    let (cur, cursor) = cursor.split::<4, 0>();
                    self.byte_end.write(cur, buffer_position - 20);
                    cursor.finish([]);
                }
            }

            impl ::planus::WriteAsOffset<ProvenanceRange> for ProvenanceRange {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<ProvenanceRange> {
                    unsafe {
                        builder.write_with(24, 7, |buffer_position, bytes| {
                            let bytes = bytes.as_mut_ptr();

                            ::planus::WriteAsPrimitive::write(
                                self,
                                ::planus::Cursor::new(
                                    &mut *(bytes as *mut [::core::mem::MaybeUninit<u8>; 24]),
                                ),
                                buffer_position,
                            );
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<ProvenanceRange> for ProvenanceRange {
                type Prepared = Self;
                #[inline]
                fn prepare(&self, _builder: &mut ::planus::Builder) -> Self {
                    *self
                }
            }

            impl ::planus::WriteAsOptional<ProvenanceRange> for ProvenanceRange {
                type Prepared = Self;
                #[inline]
                fn prepare(
                    &self,
                    _builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<Self> {
                    ::core::option::Option::Some(*self)
                }
            }

            /// Reference to a deserialized [ProvenanceRange].
            #[derive(Copy, Clone)]
            pub struct ProvenanceRangeRef<'a>(::planus::ArrayWithStartOffset<'a, 24>);

            impl<'a> ProvenanceRangeRef<'a> {
                /// Getter for the [`first_lsn` field](ProvenanceRange#structfield.first_lsn).
                pub fn first_lsn(&self) -> u64 {
                    let buffer = self.0.advance_as_array::<8>(0).unwrap();

                    u64::from_le_bytes(*buffer.as_array())
                }

                /// Getter for the [`last_lsn` field](ProvenanceRange#structfield.last_lsn).
                pub fn last_lsn(&self) -> u64 {
                    let buffer = self.0.advance_as_array::<8>(8).unwrap();

                    u64::from_le_bytes(*buffer.as_array())
                }

                /// Getter for the [`byte_start` field](ProvenanceRange#structfield.byte_start).
                pub fn byte_start(&self) -> u32 {
                    let buffer = self.0.advance_as_array::<4>(16).unwrap();

                    u32::from_le_bytes(*buffer.as_array())
                }

                /// Getter for the [`byte_end` field](ProvenanceRange#structfield.byte_end).
                pub fn byte_end(&self) -> u32 {
                    let buffer = self.0.advance_as_array::<4>(20).unwrap();

                    u32::from_le_bytes(*buffer.as_array())
                }
            }

            impl<'a> ::core::fmt::Debug for ProvenanceRangeRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("ProvenanceRangeRef");
                    f.field("first_lsn", &self.first_lsn());
                    f.field("last_lsn", &self.last_lsn());
                    f.field("byte_start", &self.byte_start());
                    f.field("byte_end", &self.byte_end());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::From<::planus::ArrayWithStartOffset<'a, 24>> for ProvenanceRangeRef<'a> {
                fn from(array: ::planus::ArrayWithStartOffset<'a, 24>) -> Self {
                    Self(array)
                }
            }

            impl<'a> ::core::convert::From<ProvenanceRangeRef<'a>> for ProvenanceRange {
                #[allow(unreachable_code)]
                fn from(value: ProvenanceRangeRef<'a>) -> Self {
                    Self {
                        first_lsn: value.first_lsn(),
                        last_lsn: value.last_lsn(),
                        byte_start: value.byte_start(),
                        byte_end: value.byte_end(),
                    }
                }
            }

            impl<'a, 'b> ::core::cmp::PartialEq<ProvenanceRangeRef<'a>> for ProvenanceRangeRef<'b> {
                fn eq(&self, other: &ProvenanceRangeRef<'_>) -> bool {
                    self.first_lsn() == other.first_lsn()
                        && self.last_lsn() == other.last_lsn()
                        && self.byte_start() == other.byte_start()
                        && self.byte_end() == other.byte_end()
                }
            }

            impl<'a> ::core::cmp::Eq for ProvenanceRangeRef<'a> {}
            impl<'a, 'b> ::core::cmp::PartialOrd<ProvenanceRangeRef<'a>> for ProvenanceRangeRef<'b> {
                fn partial_cmp(
                    &self,
                    other: &ProvenanceRangeRef<'_>,
                ) -> ::core::option::Option<::core::cmp::Ordering> {
                    ::core::option::Option::Some(::core::cmp::Ord::cmp(self, other))
                }
            }

            impl<'a> ::core::cmp::Ord for ProvenanceRangeRef<'a> {
                fn cmp(&self, other: &ProvenanceRangeRef<'_>) -> ::core::cmp::Ordering {
                    self.first_lsn()
                        .cmp(&other.first_lsn())
                        .then_with(|| self.last_lsn().cmp(&other.last_lsn()))
                        .then_with(|| self.byte_start().cmp(&other.byte_start()))
                        .then_with(|| self.byte_end().cmp(&other.byte_end()))
                }
            }

            impl<'a> ::core::hash::Hash for ProvenanceRangeRef<'a> {
                fn hash<H: ::core::hash::Hasher>(&self, state: &mut H) {
                    self.first_lsn().hash(state);
                    self.last_lsn().hash(state);
                    self.byte_start().hash(state);
                    self.byte_end().hash(state);
                }
            }

            impl<'a> ::planus::TableRead<'a> for ProvenanceRangeRef<'a> {
                #[inline]
                fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::ErrorKind> {
                    let buffer = buffer.advance_as_array::<24>(offset)?;
                    ::core::result::Result::Ok(Self(buffer))
                }
            }

            impl<'a> ::planus::VectorRead<'a> for ProvenanceRangeRef<'a> {
                const STRIDE: usize = 24;

                #[inline]
                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> Self {
                    Self(unsafe { buffer.unchecked_advance_as_array(offset) })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<ProvenanceRange> for ProvenanceRange {
                const STRIDE: usize = 24;

                type Value = ProvenanceRange;

                #[inline]
                fn prepare(&self, _builder: &mut ::planus::Builder) -> Self::Value {
                    *self
                }

                #[inline]
                unsafe fn write_values(
                    values: &[ProvenanceRange],
                    bytes: *mut ::core::mem::MaybeUninit<u8>,
                    buffer_position: u32,
                ) {
                    let bytes = bytes as *mut [::core::mem::MaybeUninit<u8>; 24];
                    for (i, v) in ::core::iter::Iterator::enumerate(values.iter()) {
                        ::planus::WriteAsPrimitive::write(
                            v,
                            ::planus::Cursor::new(unsafe { &mut *bytes.add(i) }),
                            buffer_position - (24 * i) as u32,
                        );
                    }
                }
            }

            /// The table `ModelProvenance` in the namespace `hypermind.schema`
            ///
            /// Generated from these locations:
            /// * Table `ModelProvenance` in the file `schemas/events.fbs:28`
            #[derive(
                Clone, Debug, PartialEq, PartialOrd, ::serde::Serialize, ::serde::Deserialize,
            )]
            pub struct ModelProvenance {
                /// The field `model_id` in the table `ModelProvenance`
                pub model_id: ::planus::alloc::string::String,
                /// The field `prompt_id` in the table `ModelProvenance`
                pub prompt_id: ::planus::alloc::string::String,
                /// The field `prompt_version` in the table `ModelProvenance`
                pub prompt_version: u16,
                /// The field `temperature` in the table `ModelProvenance`
                pub temperature: f32,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for ModelProvenance {
                fn default() -> Self {
                    Self {
                        model_id: ::core::default::Default::default(),
                        prompt_id: ::core::default::Default::default(),
                        prompt_version: 0,
                        temperature: 0.0,
                    }
                }
            }

            impl ModelProvenance {
                /// Creates a [ModelProvenanceBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> ModelProvenanceBuilder<()> {
                    ModelProvenanceBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_model_id: impl ::planus::WriteAs<::planus::Offset<str>>,
                    field_prompt_id: impl ::planus::WriteAs<::planus::Offset<str>>,
                    field_prompt_version: impl ::planus::WriteAsDefault<u16, u16>,
                    field_temperature: impl ::planus::WriteAsDefault<f32, f32>,
                ) -> ::planus::Offset<Self> {
                    let prepared_model_id = field_model_id.prepare(builder);
                    let prepared_prompt_id = field_prompt_id.prepare(builder);
                    let prepared_prompt_version = field_prompt_version.prepare(builder, &0);
                    let prepared_temperature = field_temperature.prepare(builder, &0.0);

                    let mut table_writer: ::planus::table_writer::TableWriter<12> =
                        ::core::default::Default::default();
                    table_writer.write_entry::<::planus::Offset<str>>(0);
                    table_writer.write_entry::<::planus::Offset<str>>(1);
                    if prepared_temperature.is_some() {
                        table_writer.write_entry::<f32>(3);
                    }
                    if prepared_prompt_version.is_some() {
                        table_writer.write_entry::<u16>(2);
                    }

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            object_writer.write::<_, _, 4>(&prepared_model_id);
                            object_writer.write::<_, _, 4>(&prepared_prompt_id);
                            if let ::core::option::Option::Some(prepared_temperature) =
                                prepared_temperature
                            {
                                object_writer.write::<_, _, 4>(&prepared_temperature);
                            }
                            if let ::core::option::Option::Some(prepared_prompt_version) =
                                prepared_prompt_version
                            {
                                object_writer.write::<_, _, 2>(&prepared_prompt_version);
                            }
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<ModelProvenance>> for ModelProvenance {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<ModelProvenance> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<ModelProvenance>> for ModelProvenance {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<ModelProvenance>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<ModelProvenance> for ModelProvenance {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<ModelProvenance> {
                    ModelProvenance::create(
                        builder,
                        &self.model_id,
                        &self.prompt_id,
                        self.prompt_version,
                        self.temperature,
                    )
                }
            }

            /// Builder for serializing an instance of the [ModelProvenance] type.
            ///
            /// Can be created using the [ModelProvenance::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct ModelProvenanceBuilder<State>(State);

            impl ModelProvenanceBuilder<()> {
                /// Setter for the [`model_id` field](ModelProvenance#structfield.model_id).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn model_id<T0>(self, value: T0) -> ModelProvenanceBuilder<(T0,)>
                where
                    T0: ::planus::WriteAs<::planus::Offset<str>>,
                {
                    ModelProvenanceBuilder((value,))
                }
            }

            impl<T0> ModelProvenanceBuilder<(T0,)> {
                /// Setter for the [`prompt_id` field](ModelProvenance#structfield.prompt_id).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn prompt_id<T1>(self, value: T1) -> ModelProvenanceBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAs<::planus::Offset<str>>,
                {
                    let (v0,) = self.0;
                    ModelProvenanceBuilder((v0, value))
                }
            }

            impl<T0, T1> ModelProvenanceBuilder<(T0, T1)> {
                /// Setter for the [`prompt_version` field](ModelProvenance#structfield.prompt_version).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn prompt_version<T2>(self, value: T2) -> ModelProvenanceBuilder<(T0, T1, T2)>
                where
                    T2: ::planus::WriteAsDefault<u16, u16>,
                {
                    let (v0, v1) = self.0;
                    ModelProvenanceBuilder((v0, v1, value))
                }

                /// Sets the [`prompt_version` field](ModelProvenance#structfield.prompt_version) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn prompt_version_as_default(
                    self,
                ) -> ModelProvenanceBuilder<(T0, T1, ::planus::DefaultValue)> {
                    self.prompt_version(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2> ModelProvenanceBuilder<(T0, T1, T2)> {
                /// Setter for the [`temperature` field](ModelProvenance#structfield.temperature).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn temperature<T3>(self, value: T3) -> ModelProvenanceBuilder<(T0, T1, T2, T3)>
                where
                    T3: ::planus::WriteAsDefault<f32, f32>,
                {
                    let (v0, v1, v2) = self.0;
                    ModelProvenanceBuilder((v0, v1, v2, value))
                }

                /// Sets the [`temperature` field](ModelProvenance#structfield.temperature) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn temperature_as_default(
                    self,
                ) -> ModelProvenanceBuilder<(T0, T1, T2, ::planus::DefaultValue)> {
                    self.temperature(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2, T3> ModelProvenanceBuilder<(T0, T1, T2, T3)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [ModelProvenance].
                #[inline]
                pub fn finish(
                    self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<ModelProvenance>
                where
                    Self: ::planus::WriteAsOffset<ModelProvenance>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<str>>,
                T1: ::planus::WriteAs<::planus::Offset<str>>,
                T2: ::planus::WriteAsDefault<u16, u16>,
                T3: ::planus::WriteAsDefault<f32, f32>,
            > ::planus::WriteAs<::planus::Offset<ModelProvenance>>
                for ModelProvenanceBuilder<(T0, T1, T2, T3)>
            {
                type Prepared = ::planus::Offset<ModelProvenance>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<ModelProvenance> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<str>>,
                T1: ::planus::WriteAs<::planus::Offset<str>>,
                T2: ::planus::WriteAsDefault<u16, u16>,
                T3: ::planus::WriteAsDefault<f32, f32>,
            > ::planus::WriteAsOptional<::planus::Offset<ModelProvenance>>
                for ModelProvenanceBuilder<(T0, T1, T2, T3)>
            {
                type Prepared = ::planus::Offset<ModelProvenance>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<ModelProvenance>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<str>>,
                T1: ::planus::WriteAs<::planus::Offset<str>>,
                T2: ::planus::WriteAsDefault<u16, u16>,
                T3: ::planus::WriteAsDefault<f32, f32>,
            > ::planus::WriteAsOffset<ModelProvenance>
                for ModelProvenanceBuilder<(T0, T1, T2, T3)>
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<ModelProvenance> {
                    let (v0, v1, v2, v3) = &self.0;
                    ModelProvenance::create(builder, v0, v1, v2, v3)
                }
            }

            /// Reference to a deserialized [ModelProvenance].
            #[derive(Copy, Clone)]
            pub struct ModelProvenanceRef<'a>(
                #[allow(dead_code)] ::planus::table_reader::Table<'a>,
            );

            impl<'a> ModelProvenanceRef<'a> {
                /// Getter for the [`model_id` field](ModelProvenance#structfield.model_id).
                #[inline]
                pub fn model_id(&self) -> ::planus::Result<&'a ::core::primitive::str> {
                    self.0.access_required(0, "ModelProvenance", "model_id")
                }

                /// Getter for the [`prompt_id` field](ModelProvenance#structfield.prompt_id).
                #[inline]
                pub fn prompt_id(&self) -> ::planus::Result<&'a ::core::primitive::str> {
                    self.0.access_required(1, "ModelProvenance", "prompt_id")
                }

                /// Getter for the [`prompt_version` field](ModelProvenance#structfield.prompt_version).
                #[inline]
                pub fn prompt_version(&self) -> ::planus::Result<u16> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(2, "ModelProvenance", "prompt_version")?
                            .unwrap_or(0),
                    )
                }

                /// Getter for the [`temperature` field](ModelProvenance#structfield.temperature).
                #[inline]
                pub fn temperature(&self) -> ::planus::Result<f32> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(3, "ModelProvenance", "temperature")?
                            .unwrap_or(0.0),
                    )
                }
            }

            impl<'a> ::core::fmt::Debug for ModelProvenanceRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("ModelProvenanceRef");
                    f.field("model_id", &self.model_id());
                    f.field("prompt_id", &self.prompt_id());
                    f.field("prompt_version", &self.prompt_version());
                    f.field("temperature", &self.temperature());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<ModelProvenanceRef<'a>> for ModelProvenance {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: ModelProvenanceRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        model_id: ::core::convert::Into::into(value.model_id()?),
                        prompt_id: ::core::convert::Into::into(value.prompt_id()?),
                        prompt_version: ::core::convert::TryInto::try_into(
                            value.prompt_version()?,
                        )?,
                        temperature: ::core::convert::TryInto::try_into(value.temperature()?)?,
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for ModelProvenanceRef<'a> {
                #[inline]
                fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::ErrorKind> {
                    ::core::result::Result::Ok(Self(::planus::table_reader::Table::from_buffer(
                        buffer, offset,
                    )?))
                }
            }

            impl<'a> ::planus::VectorReadInner<'a> for ModelProvenanceRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[ModelProvenanceRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<ModelProvenance>> for ModelProvenance {
                type Value = ::planus::Offset<ModelProvenance>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<ModelProvenance>],
                    bytes: *mut ::core::mem::MaybeUninit<u8>,
                    buffer_position: u32,
                ) {
                    let bytes = bytes as *mut [::core::mem::MaybeUninit<u8>; 4];
                    for (i, v) in ::core::iter::Iterator::enumerate(values.iter()) {
                        ::planus::WriteAsPrimitive::write(
                            v,
                            ::planus::Cursor::new(unsafe { &mut *bytes.add(i) }),
                            buffer_position - (Self::STRIDE * i) as u32,
                        );
                    }
                }
            }

            impl<'a> ::planus::ReadAsRoot<'a> for ModelProvenanceRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[ModelProvenanceRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `UserMsg` in the namespace `hypermind.schema`
            ///
            /// Generated from these locations:
            /// * Table `UserMsg` in the file `schemas/events.fbs:35`
            #[derive(
                Clone,
                Debug,
                PartialEq,
                PartialOrd,
                Eq,
                Ord,
                Hash,
                ::serde::Serialize,
                ::serde::Deserialize,
            )]
            pub struct UserMsg {
                /// The field `content` in the table `UserMsg`
                pub content: ::planus::alloc::vec::Vec<u8>,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for UserMsg {
                fn default() -> Self {
                    Self {
                        content: ::core::default::Default::default(),
                    }
                }
            }

            impl UserMsg {
                /// Creates a [UserMsgBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> UserMsgBuilder<()> {
                    UserMsgBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_content: impl ::planus::WriteAs<::planus::Offset<[u8]>>,
                ) -> ::planus::Offset<Self> {
                    let prepared_content = field_content.prepare(builder);

                    let mut table_writer: ::planus::table_writer::TableWriter<6> =
                        ::core::default::Default::default();
                    table_writer.write_entry::<::planus::Offset<[u8]>>(0);

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            object_writer.write::<_, _, 4>(&prepared_content);
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<UserMsg>> for UserMsg {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<UserMsg> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<UserMsg>> for UserMsg {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<UserMsg>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<UserMsg> for UserMsg {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<UserMsg> {
                    UserMsg::create(builder, &self.content)
                }
            }

            /// Builder for serializing an instance of the [UserMsg] type.
            ///
            /// Can be created using the [UserMsg::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct UserMsgBuilder<State>(State);

            impl UserMsgBuilder<()> {
                /// Setter for the [`content` field](UserMsg#structfield.content).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn content<T0>(self, value: T0) -> UserMsgBuilder<(T0,)>
                where
                    T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                {
                    UserMsgBuilder((value,))
                }
            }

            impl<T0> UserMsgBuilder<(T0,)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [UserMsg].
                #[inline]
                pub fn finish(self, builder: &mut ::planus::Builder) -> ::planus::Offset<UserMsg>
                where
                    Self: ::planus::WriteAsOffset<UserMsg>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<T0: ::planus::WriteAs<::planus::Offset<[u8]>>>
                ::planus::WriteAs<::planus::Offset<UserMsg>> for UserMsgBuilder<(T0,)>
            {
                type Prepared = ::planus::Offset<UserMsg>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<UserMsg> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<T0: ::planus::WriteAs<::planus::Offset<[u8]>>>
                ::planus::WriteAsOptional<::planus::Offset<UserMsg>> for UserMsgBuilder<(T0,)>
            {
                type Prepared = ::planus::Offset<UserMsg>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<UserMsg>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<T0: ::planus::WriteAs<::planus::Offset<[u8]>>> ::planus::WriteAsOffset<UserMsg>
                for UserMsgBuilder<(T0,)>
            {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<UserMsg> {
                    let (v0,) = &self.0;
                    UserMsg::create(builder, v0)
                }
            }

            /// Reference to a deserialized [UserMsg].
            #[derive(Copy, Clone)]
            pub struct UserMsgRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> UserMsgRef<'a> {
                /// Getter for the [`content` field](UserMsg#structfield.content).
                #[inline]
                pub fn content(&self) -> ::planus::Result<&'a [u8]> {
                    self.0.access_required(0, "UserMsg", "content")
                }
            }

            impl<'a> ::core::fmt::Debug for UserMsgRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("UserMsgRef");
                    f.field("content", &self.content());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<UserMsgRef<'a>> for UserMsg {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: UserMsgRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        content: value.content()?.to_vec(),
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for UserMsgRef<'a> {
                #[inline]
                fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::ErrorKind> {
                    ::core::result::Result::Ok(Self(::planus::table_reader::Table::from_buffer(
                        buffer, offset,
                    )?))
                }
            }

            impl<'a> ::planus::VectorReadInner<'a> for UserMsgRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[UserMsgRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<UserMsg>> for UserMsg {
                type Value = ::planus::Offset<UserMsg>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<UserMsg>],
                    bytes: *mut ::core::mem::MaybeUninit<u8>,
                    buffer_position: u32,
                ) {
                    let bytes = bytes as *mut [::core::mem::MaybeUninit<u8>; 4];
                    for (i, v) in ::core::iter::Iterator::enumerate(values.iter()) {
                        ::planus::WriteAsPrimitive::write(
                            v,
                            ::planus::Cursor::new(unsafe { &mut *bytes.add(i) }),
                            buffer_position - (Self::STRIDE * i) as u32,
                        );
                    }
                }
            }

            impl<'a> ::planus::ReadAsRoot<'a> for UserMsgRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[UserMsgRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `DeliveredMsg` in the namespace `hypermind.schema`
            ///
            /// Generated from these locations:
            /// * Table `DeliveredMsg` in the file `schemas/events.fbs:39`
            #[derive(
                Clone,
                Debug,
                PartialEq,
                PartialOrd,
                Eq,
                Ord,
                Hash,
                ::serde::Serialize,
                ::serde::Deserialize,
            )]
            pub struct DeliveredMsg {
                /// The field `content` in the table `DeliveredMsg`
                pub content: ::planus::alloc::vec::Vec<u8>,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for DeliveredMsg {
                fn default() -> Self {
                    Self {
                        content: ::core::default::Default::default(),
                    }
                }
            }

            impl DeliveredMsg {
                /// Creates a [DeliveredMsgBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> DeliveredMsgBuilder<()> {
                    DeliveredMsgBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_content: impl ::planus::WriteAs<::planus::Offset<[u8]>>,
                ) -> ::planus::Offset<Self> {
                    let prepared_content = field_content.prepare(builder);

                    let mut table_writer: ::planus::table_writer::TableWriter<6> =
                        ::core::default::Default::default();
                    table_writer.write_entry::<::planus::Offset<[u8]>>(0);

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            object_writer.write::<_, _, 4>(&prepared_content);
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<DeliveredMsg>> for DeliveredMsg {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<DeliveredMsg> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<DeliveredMsg>> for DeliveredMsg {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<DeliveredMsg>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<DeliveredMsg> for DeliveredMsg {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<DeliveredMsg> {
                    DeliveredMsg::create(builder, &self.content)
                }
            }

            /// Builder for serializing an instance of the [DeliveredMsg] type.
            ///
            /// Can be created using the [DeliveredMsg::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct DeliveredMsgBuilder<State>(State);

            impl DeliveredMsgBuilder<()> {
                /// Setter for the [`content` field](DeliveredMsg#structfield.content).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn content<T0>(self, value: T0) -> DeliveredMsgBuilder<(T0,)>
                where
                    T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                {
                    DeliveredMsgBuilder((value,))
                }
            }

            impl<T0> DeliveredMsgBuilder<(T0,)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [DeliveredMsg].
                #[inline]
                pub fn finish(
                    self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<DeliveredMsg>
                where
                    Self: ::planus::WriteAsOffset<DeliveredMsg>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<T0: ::planus::WriteAs<::planus::Offset<[u8]>>>
                ::planus::WriteAs<::planus::Offset<DeliveredMsg>> for DeliveredMsgBuilder<(T0,)>
            {
                type Prepared = ::planus::Offset<DeliveredMsg>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<DeliveredMsg> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<T0: ::planus::WriteAs<::planus::Offset<[u8]>>>
                ::planus::WriteAsOptional<::planus::Offset<DeliveredMsg>>
                for DeliveredMsgBuilder<(T0,)>
            {
                type Prepared = ::planus::Offset<DeliveredMsg>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<DeliveredMsg>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<T0: ::planus::WriteAs<::planus::Offset<[u8]>>>
                ::planus::WriteAsOffset<DeliveredMsg> for DeliveredMsgBuilder<(T0,)>
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<DeliveredMsg> {
                    let (v0,) = &self.0;
                    DeliveredMsg::create(builder, v0)
                }
            }

            /// Reference to a deserialized [DeliveredMsg].
            #[derive(Copy, Clone)]
            pub struct DeliveredMsgRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> DeliveredMsgRef<'a> {
                /// Getter for the [`content` field](DeliveredMsg#structfield.content).
                #[inline]
                pub fn content(&self) -> ::planus::Result<&'a [u8]> {
                    self.0.access_required(0, "DeliveredMsg", "content")
                }
            }

            impl<'a> ::core::fmt::Debug for DeliveredMsgRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("DeliveredMsgRef");
                    f.field("content", &self.content());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<DeliveredMsgRef<'a>> for DeliveredMsg {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: DeliveredMsgRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        content: value.content()?.to_vec(),
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for DeliveredMsgRef<'a> {
                #[inline]
                fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::ErrorKind> {
                    ::core::result::Result::Ok(Self(::planus::table_reader::Table::from_buffer(
                        buffer, offset,
                    )?))
                }
            }

            impl<'a> ::planus::VectorReadInner<'a> for DeliveredMsgRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[DeliveredMsgRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<DeliveredMsg>> for DeliveredMsg {
                type Value = ::planus::Offset<DeliveredMsg>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<DeliveredMsg>],
                    bytes: *mut ::core::mem::MaybeUninit<u8>,
                    buffer_position: u32,
                ) {
                    let bytes = bytes as *mut [::core::mem::MaybeUninit<u8>; 4];
                    for (i, v) in ::core::iter::Iterator::enumerate(values.iter()) {
                        ::planus::WriteAsPrimitive::write(
                            v,
                            ::planus::Cursor::new(unsafe { &mut *bytes.add(i) }),
                            buffer_position - (Self::STRIDE * i) as u32,
                        );
                    }
                }
            }

            impl<'a> ::planus::ReadAsRoot<'a> for DeliveredMsgRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[DeliveredMsgRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `ToolCall` in the namespace `hypermind.schema`
            ///
            /// Generated from these locations:
            /// * Table `ToolCall` in the file `schemas/events.fbs:43`
            #[derive(
                Clone,
                Debug,
                PartialEq,
                PartialOrd,
                Eq,
                Ord,
                Hash,
                ::serde::Serialize,
                ::serde::Deserialize,
            )]
            pub struct ToolCall {
                /// The field `call_id` in the table `ToolCall`
                pub call_id: ::planus::alloc::vec::Vec<u8>,
                /// The field `tool_name` in the table `ToolCall`
                pub tool_name: ::planus::alloc::string::String,
                /// The field `arguments` in the table `ToolCall`
                pub arguments: ::planus::alloc::vec::Vec<u8>,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for ToolCall {
                fn default() -> Self {
                    Self {
                        call_id: ::core::default::Default::default(),
                        tool_name: ::core::default::Default::default(),
                        arguments: ::core::default::Default::default(),
                    }
                }
            }

            impl ToolCall {
                /// Creates a [ToolCallBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> ToolCallBuilder<()> {
                    ToolCallBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_call_id: impl ::planus::WriteAs<::planus::Offset<[u8]>>,
                    field_tool_name: impl ::planus::WriteAs<::planus::Offset<str>>,
                    field_arguments: impl ::planus::WriteAs<::planus::Offset<[u8]>>,
                ) -> ::planus::Offset<Self> {
                    let prepared_call_id = field_call_id.prepare(builder);
                    let prepared_tool_name = field_tool_name.prepare(builder);
                    let prepared_arguments = field_arguments.prepare(builder);

                    let mut table_writer: ::planus::table_writer::TableWriter<10> =
                        ::core::default::Default::default();
                    table_writer.write_entry::<::planus::Offset<[u8]>>(0);
                    table_writer.write_entry::<::planus::Offset<str>>(1);
                    table_writer.write_entry::<::planus::Offset<[u8]>>(2);

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            object_writer.write::<_, _, 4>(&prepared_call_id);
                            object_writer.write::<_, _, 4>(&prepared_tool_name);
                            object_writer.write::<_, _, 4>(&prepared_arguments);
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<ToolCall>> for ToolCall {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<ToolCall> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<ToolCall>> for ToolCall {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<ToolCall>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<ToolCall> for ToolCall {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<ToolCall> {
                    ToolCall::create(builder, &self.call_id, &self.tool_name, &self.arguments)
                }
            }

            /// Builder for serializing an instance of the [ToolCall] type.
            ///
            /// Can be created using the [ToolCall::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct ToolCallBuilder<State>(State);

            impl ToolCallBuilder<()> {
                /// Setter for the [`call_id` field](ToolCall#structfield.call_id).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn call_id<T0>(self, value: T0) -> ToolCallBuilder<(T0,)>
                where
                    T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                {
                    ToolCallBuilder((value,))
                }
            }

            impl<T0> ToolCallBuilder<(T0,)> {
                /// Setter for the [`tool_name` field](ToolCall#structfield.tool_name).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn tool_name<T1>(self, value: T1) -> ToolCallBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAs<::planus::Offset<str>>,
                {
                    let (v0,) = self.0;
                    ToolCallBuilder((v0, value))
                }
            }

            impl<T0, T1> ToolCallBuilder<(T0, T1)> {
                /// Setter for the [`arguments` field](ToolCall#structfield.arguments).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn arguments<T2>(self, value: T2) -> ToolCallBuilder<(T0, T1, T2)>
                where
                    T2: ::planus::WriteAs<::planus::Offset<[u8]>>,
                {
                    let (v0, v1) = self.0;
                    ToolCallBuilder((v0, v1, value))
                }
            }

            impl<T0, T1, T2> ToolCallBuilder<(T0, T1, T2)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [ToolCall].
                #[inline]
                pub fn finish(self, builder: &mut ::planus::Builder) -> ::planus::Offset<ToolCall>
                where
                    Self: ::planus::WriteAsOffset<ToolCall>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T1: ::planus::WriteAs<::planus::Offset<str>>,
                T2: ::planus::WriteAs<::planus::Offset<[u8]>>,
            > ::planus::WriteAs<::planus::Offset<ToolCall>> for ToolCallBuilder<(T0, T1, T2)>
            {
                type Prepared = ::planus::Offset<ToolCall>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<ToolCall> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T1: ::planus::WriteAs<::planus::Offset<str>>,
                T2: ::planus::WriteAs<::planus::Offset<[u8]>>,
            > ::planus::WriteAsOptional<::planus::Offset<ToolCall>>
                for ToolCallBuilder<(T0, T1, T2)>
            {
                type Prepared = ::planus::Offset<ToolCall>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<ToolCall>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T1: ::planus::WriteAs<::planus::Offset<str>>,
                T2: ::planus::WriteAs<::planus::Offset<[u8]>>,
            > ::planus::WriteAsOffset<ToolCall> for ToolCallBuilder<(T0, T1, T2)>
            {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<ToolCall> {
                    let (v0, v1, v2) = &self.0;
                    ToolCall::create(builder, v0, v1, v2)
                }
            }

            /// Reference to a deserialized [ToolCall].
            #[derive(Copy, Clone)]
            pub struct ToolCallRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> ToolCallRef<'a> {
                /// Getter for the [`call_id` field](ToolCall#structfield.call_id).
                #[inline]
                pub fn call_id(&self) -> ::planus::Result<&'a [u8]> {
                    self.0.access_required(0, "ToolCall", "call_id")
                }

                /// Getter for the [`tool_name` field](ToolCall#structfield.tool_name).
                #[inline]
                pub fn tool_name(&self) -> ::planus::Result<&'a ::core::primitive::str> {
                    self.0.access_required(1, "ToolCall", "tool_name")
                }

                /// Getter for the [`arguments` field](ToolCall#structfield.arguments).
                #[inline]
                pub fn arguments(&self) -> ::planus::Result<&'a [u8]> {
                    self.0.access_required(2, "ToolCall", "arguments")
                }
            }

            impl<'a> ::core::fmt::Debug for ToolCallRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("ToolCallRef");
                    f.field("call_id", &self.call_id());
                    f.field("tool_name", &self.tool_name());
                    f.field("arguments", &self.arguments());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<ToolCallRef<'a>> for ToolCall {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: ToolCallRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        call_id: value.call_id()?.to_vec(),
                        tool_name: ::core::convert::Into::into(value.tool_name()?),
                        arguments: value.arguments()?.to_vec(),
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for ToolCallRef<'a> {
                #[inline]
                fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::ErrorKind> {
                    ::core::result::Result::Ok(Self(::planus::table_reader::Table::from_buffer(
                        buffer, offset,
                    )?))
                }
            }

            impl<'a> ::planus::VectorReadInner<'a> for ToolCallRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[ToolCallRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<ToolCall>> for ToolCall {
                type Value = ::planus::Offset<ToolCall>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<ToolCall>],
                    bytes: *mut ::core::mem::MaybeUninit<u8>,
                    buffer_position: u32,
                ) {
                    let bytes = bytes as *mut [::core::mem::MaybeUninit<u8>; 4];
                    for (i, v) in ::core::iter::Iterator::enumerate(values.iter()) {
                        ::planus::WriteAsPrimitive::write(
                            v,
                            ::planus::Cursor::new(unsafe { &mut *bytes.add(i) }),
                            buffer_position - (Self::STRIDE * i) as u32,
                        );
                    }
                }
            }

            impl<'a> ::planus::ReadAsRoot<'a> for ToolCallRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[ToolCallRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `ToolResult` in the namespace `hypermind.schema`
            ///
            /// Generated from these locations:
            /// * Table `ToolResult` in the file `schemas/events.fbs:49`
            #[derive(
                Clone,
                Debug,
                PartialEq,
                PartialOrd,
                Eq,
                Ord,
                Hash,
                ::serde::Serialize,
                ::serde::Deserialize,
            )]
            pub struct ToolResult {
                /// The field `call_id` in the table `ToolResult`
                pub call_id: ::planus::alloc::vec::Vec<u8>,
                /// The field `tool_call_lsn` in the table `ToolResult`
                pub tool_call_lsn: u64,
                /// The field `status` in the table `ToolResult`
                pub status: self::ResultStatus,
                /// The field `result` in the table `ToolResult`
                pub result: ::planus::alloc::vec::Vec<u8>,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for ToolResult {
                fn default() -> Self {
                    Self {
                        call_id: ::core::default::Default::default(),
                        tool_call_lsn: 0,
                        status: self::ResultStatus::Ok,
                        result: ::core::default::Default::default(),
                    }
                }
            }

            impl ToolResult {
                /// Creates a [ToolResultBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> ToolResultBuilder<()> {
                    ToolResultBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_call_id: impl ::planus::WriteAs<::planus::Offset<[u8]>>,
                    field_tool_call_lsn: impl ::planus::WriteAsDefault<u64, u64>,
                    field_status: impl ::planus::WriteAsDefault<self::ResultStatus, self::ResultStatus>,
                    field_result: impl ::planus::WriteAs<::planus::Offset<[u8]>>,
                ) -> ::planus::Offset<Self> {
                    let prepared_call_id = field_call_id.prepare(builder);
                    let prepared_tool_call_lsn = field_tool_call_lsn.prepare(builder, &0);
                    let prepared_status = field_status.prepare(builder, &self::ResultStatus::Ok);
                    let prepared_result = field_result.prepare(builder);

                    let mut table_writer: ::planus::table_writer::TableWriter<12> =
                        ::core::default::Default::default();
                    if prepared_tool_call_lsn.is_some() {
                        table_writer.write_entry::<u64>(1);
                    }
                    table_writer.write_entry::<::planus::Offset<[u8]>>(0);
                    table_writer.write_entry::<::planus::Offset<[u8]>>(3);
                    if prepared_status.is_some() {
                        table_writer.write_entry::<self::ResultStatus>(2);
                    }

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            if let ::core::option::Option::Some(prepared_tool_call_lsn) =
                                prepared_tool_call_lsn
                            {
                                object_writer.write::<_, _, 8>(&prepared_tool_call_lsn);
                            }
                            object_writer.write::<_, _, 4>(&prepared_call_id);
                            object_writer.write::<_, _, 4>(&prepared_result);
                            if let ::core::option::Option::Some(prepared_status) = prepared_status {
                                object_writer.write::<_, _, 1>(&prepared_status);
                            }
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<ToolResult>> for ToolResult {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<ToolResult> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<ToolResult>> for ToolResult {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<ToolResult>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<ToolResult> for ToolResult {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<ToolResult> {
                    ToolResult::create(
                        builder,
                        &self.call_id,
                        self.tool_call_lsn,
                        self.status,
                        &self.result,
                    )
                }
            }

            /// Builder for serializing an instance of the [ToolResult] type.
            ///
            /// Can be created using the [ToolResult::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct ToolResultBuilder<State>(State);

            impl ToolResultBuilder<()> {
                /// Setter for the [`call_id` field](ToolResult#structfield.call_id).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn call_id<T0>(self, value: T0) -> ToolResultBuilder<(T0,)>
                where
                    T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                {
                    ToolResultBuilder((value,))
                }
            }

            impl<T0> ToolResultBuilder<(T0,)> {
                /// Setter for the [`tool_call_lsn` field](ToolResult#structfield.tool_call_lsn).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn tool_call_lsn<T1>(self, value: T1) -> ToolResultBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAsDefault<u64, u64>,
                {
                    let (v0,) = self.0;
                    ToolResultBuilder((v0, value))
                }

                /// Sets the [`tool_call_lsn` field](ToolResult#structfield.tool_call_lsn) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn tool_call_lsn_as_default(
                    self,
                ) -> ToolResultBuilder<(T0, ::planus::DefaultValue)> {
                    self.tool_call_lsn(::planus::DefaultValue)
                }
            }

            impl<T0, T1> ToolResultBuilder<(T0, T1)> {
                /// Setter for the [`status` field](ToolResult#structfield.status).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn status<T2>(self, value: T2) -> ToolResultBuilder<(T0, T1, T2)>
                where
                    T2: ::planus::WriteAsDefault<self::ResultStatus, self::ResultStatus>,
                {
                    let (v0, v1) = self.0;
                    ToolResultBuilder((v0, v1, value))
                }

                /// Sets the [`status` field](ToolResult#structfield.status) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn status_as_default(
                    self,
                ) -> ToolResultBuilder<(T0, T1, ::planus::DefaultValue)> {
                    self.status(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2> ToolResultBuilder<(T0, T1, T2)> {
                /// Setter for the [`result` field](ToolResult#structfield.result).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn result<T3>(self, value: T3) -> ToolResultBuilder<(T0, T1, T2, T3)>
                where
                    T3: ::planus::WriteAs<::planus::Offset<[u8]>>,
                {
                    let (v0, v1, v2) = self.0;
                    ToolResultBuilder((v0, v1, v2, value))
                }
            }

            impl<T0, T1, T2, T3> ToolResultBuilder<(T0, T1, T2, T3)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [ToolResult].
                #[inline]
                pub fn finish(self, builder: &mut ::planus::Builder) -> ::planus::Offset<ToolResult>
                where
                    Self: ::planus::WriteAsOffset<ToolResult>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T1: ::planus::WriteAsDefault<u64, u64>,
                T2: ::planus::WriteAsDefault<self::ResultStatus, self::ResultStatus>,
                T3: ::planus::WriteAs<::planus::Offset<[u8]>>,
            > ::planus::WriteAs<::planus::Offset<ToolResult>>
                for ToolResultBuilder<(T0, T1, T2, T3)>
            {
                type Prepared = ::planus::Offset<ToolResult>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<ToolResult> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T1: ::planus::WriteAsDefault<u64, u64>,
                T2: ::planus::WriteAsDefault<self::ResultStatus, self::ResultStatus>,
                T3: ::planus::WriteAs<::planus::Offset<[u8]>>,
            > ::planus::WriteAsOptional<::planus::Offset<ToolResult>>
                for ToolResultBuilder<(T0, T1, T2, T3)>
            {
                type Prepared = ::planus::Offset<ToolResult>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<ToolResult>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T1: ::planus::WriteAsDefault<u64, u64>,
                T2: ::planus::WriteAsDefault<self::ResultStatus, self::ResultStatus>,
                T3: ::planus::WriteAs<::planus::Offset<[u8]>>,
            > ::planus::WriteAsOffset<ToolResult> for ToolResultBuilder<(T0, T1, T2, T3)>
            {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<ToolResult> {
                    let (v0, v1, v2, v3) = &self.0;
                    ToolResult::create(builder, v0, v1, v2, v3)
                }
            }

            /// Reference to a deserialized [ToolResult].
            #[derive(Copy, Clone)]
            pub struct ToolResultRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> ToolResultRef<'a> {
                /// Getter for the [`call_id` field](ToolResult#structfield.call_id).
                #[inline]
                pub fn call_id(&self) -> ::planus::Result<&'a [u8]> {
                    self.0.access_required(0, "ToolResult", "call_id")
                }

                /// Getter for the [`tool_call_lsn` field](ToolResult#structfield.tool_call_lsn).
                #[inline]
                pub fn tool_call_lsn(&self) -> ::planus::Result<u64> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(1, "ToolResult", "tool_call_lsn")?
                            .unwrap_or(0),
                    )
                }

                /// Getter for the [`status` field](ToolResult#structfield.status).
                #[inline]
                pub fn status(&self) -> ::planus::Result<self::ResultStatus> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(2, "ToolResult", "status")?
                            .unwrap_or(self::ResultStatus::Ok),
                    )
                }

                /// Getter for the [`result` field](ToolResult#structfield.result).
                #[inline]
                pub fn result(&self) -> ::planus::Result<&'a [u8]> {
                    self.0.access_required(3, "ToolResult", "result")
                }
            }

            impl<'a> ::core::fmt::Debug for ToolResultRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("ToolResultRef");
                    f.field("call_id", &self.call_id());
                    f.field("tool_call_lsn", &self.tool_call_lsn());
                    f.field("status", &self.status());
                    f.field("result", &self.result());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<ToolResultRef<'a>> for ToolResult {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: ToolResultRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        call_id: value.call_id()?.to_vec(),
                        tool_call_lsn: ::core::convert::TryInto::try_into(value.tool_call_lsn()?)?,
                        status: ::core::convert::TryInto::try_into(value.status()?)?,
                        result: value.result()?.to_vec(),
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for ToolResultRef<'a> {
                #[inline]
                fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::ErrorKind> {
                    ::core::result::Result::Ok(Self(::planus::table_reader::Table::from_buffer(
                        buffer, offset,
                    )?))
                }
            }

            impl<'a> ::planus::VectorReadInner<'a> for ToolResultRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[ToolResultRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<ToolResult>> for ToolResult {
                type Value = ::planus::Offset<ToolResult>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<ToolResult>],
                    bytes: *mut ::core::mem::MaybeUninit<u8>,
                    buffer_position: u32,
                ) {
                    let bytes = bytes as *mut [::core::mem::MaybeUninit<u8>; 4];
                    for (i, v) in ::core::iter::Iterator::enumerate(values.iter()) {
                        ::planus::WriteAsPrimitive::write(
                            v,
                            ::planus::Cursor::new(unsafe { &mut *bytes.add(i) }),
                            buffer_position - (Self::STRIDE * i) as u32,
                        );
                    }
                }
            }

            impl<'a> ::planus::ReadAsRoot<'a> for ToolResultRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[ToolResultRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `Reasoning` in the namespace `hypermind.schema`
            ///
            /// Generated from these locations:
            /// * Table `Reasoning` in the file `schemas/events.fbs:56`
            #[derive(
                Clone,
                Debug,
                PartialEq,
                PartialOrd,
                Eq,
                Ord,
                Hash,
                ::serde::Serialize,
                ::serde::Deserialize,
            )]
            pub struct Reasoning {
                /// The field `content` in the table `Reasoning`
                pub content: ::planus::alloc::vec::Vec<u8>,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for Reasoning {
                fn default() -> Self {
                    Self {
                        content: ::core::default::Default::default(),
                    }
                }
            }

            impl Reasoning {
                /// Creates a [ReasoningBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> ReasoningBuilder<()> {
                    ReasoningBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_content: impl ::planus::WriteAs<::planus::Offset<[u8]>>,
                ) -> ::planus::Offset<Self> {
                    let prepared_content = field_content.prepare(builder);

                    let mut table_writer: ::planus::table_writer::TableWriter<6> =
                        ::core::default::Default::default();
                    table_writer.write_entry::<::planus::Offset<[u8]>>(0);

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            object_writer.write::<_, _, 4>(&prepared_content);
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<Reasoning>> for Reasoning {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Reasoning> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<Reasoning>> for Reasoning {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Reasoning>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<Reasoning> for Reasoning {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Reasoning> {
                    Reasoning::create(builder, &self.content)
                }
            }

            /// Builder for serializing an instance of the [Reasoning] type.
            ///
            /// Can be created using the [Reasoning::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct ReasoningBuilder<State>(State);

            impl ReasoningBuilder<()> {
                /// Setter for the [`content` field](Reasoning#structfield.content).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn content<T0>(self, value: T0) -> ReasoningBuilder<(T0,)>
                where
                    T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                {
                    ReasoningBuilder((value,))
                }
            }

            impl<T0> ReasoningBuilder<(T0,)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [Reasoning].
                #[inline]
                pub fn finish(self, builder: &mut ::planus::Builder) -> ::planus::Offset<Reasoning>
                where
                    Self: ::planus::WriteAsOffset<Reasoning>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<T0: ::planus::WriteAs<::planus::Offset<[u8]>>>
                ::planus::WriteAs<::planus::Offset<Reasoning>> for ReasoningBuilder<(T0,)>
            {
                type Prepared = ::planus::Offset<Reasoning>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Reasoning> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<T0: ::planus::WriteAs<::planus::Offset<[u8]>>>
                ::planus::WriteAsOptional<::planus::Offset<Reasoning>> for ReasoningBuilder<(T0,)>
            {
                type Prepared = ::planus::Offset<Reasoning>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Reasoning>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<T0: ::planus::WriteAs<::planus::Offset<[u8]>>> ::planus::WriteAsOffset<Reasoning>
                for ReasoningBuilder<(T0,)>
            {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Reasoning> {
                    let (v0,) = &self.0;
                    Reasoning::create(builder, v0)
                }
            }

            /// Reference to a deserialized [Reasoning].
            #[derive(Copy, Clone)]
            pub struct ReasoningRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> ReasoningRef<'a> {
                /// Getter for the [`content` field](Reasoning#structfield.content).
                #[inline]
                pub fn content(&self) -> ::planus::Result<&'a [u8]> {
                    self.0.access_required(0, "Reasoning", "content")
                }
            }

            impl<'a> ::core::fmt::Debug for ReasoningRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("ReasoningRef");
                    f.field("content", &self.content());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<ReasoningRef<'a>> for Reasoning {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: ReasoningRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        content: value.content()?.to_vec(),
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for ReasoningRef<'a> {
                #[inline]
                fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::ErrorKind> {
                    ::core::result::Result::Ok(Self(::planus::table_reader::Table::from_buffer(
                        buffer, offset,
                    )?))
                }
            }

            impl<'a> ::planus::VectorReadInner<'a> for ReasoningRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[ReasoningRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<Reasoning>> for Reasoning {
                type Value = ::planus::Offset<Reasoning>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<Reasoning>],
                    bytes: *mut ::core::mem::MaybeUninit<u8>,
                    buffer_position: u32,
                ) {
                    let bytes = bytes as *mut [::core::mem::MaybeUninit<u8>; 4];
                    for (i, v) in ::core::iter::Iterator::enumerate(values.iter()) {
                        ::planus::WriteAsPrimitive::write(
                            v,
                            ::planus::Cursor::new(unsafe { &mut *bytes.add(i) }),
                            buffer_position - (Self::STRIDE * i) as u32,
                        );
                    }
                }
            }

            impl<'a> ::planus::ReadAsRoot<'a> for ReasoningRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[ReasoningRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `ProviderFrame` in the namespace `hypermind.schema`
            ///
            /// Generated from these locations:
            /// * Table `ProviderFrame` in the file `schemas/events.fbs:60`
            #[derive(
                Clone,
                Debug,
                PartialEq,
                PartialOrd,
                Eq,
                Ord,
                Hash,
                ::serde::Serialize,
                ::serde::Deserialize,
            )]
            pub struct ProviderFrame {
                /// The field `provider` in the table `ProviderFrame`
                pub provider: ::planus::alloc::string::String,
                /// The field `api_content` in the table `ProviderFrame`
                pub api_content: ::planus::alloc::vec::Vec<u8>,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for ProviderFrame {
                fn default() -> Self {
                    Self {
                        provider: ::core::default::Default::default(),
                        api_content: ::core::default::Default::default(),
                    }
                }
            }

            impl ProviderFrame {
                /// Creates a [ProviderFrameBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> ProviderFrameBuilder<()> {
                    ProviderFrameBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_provider: impl ::planus::WriteAs<::planus::Offset<str>>,
                    field_api_content: impl ::planus::WriteAs<::planus::Offset<[u8]>>,
                ) -> ::planus::Offset<Self> {
                    let prepared_provider = field_provider.prepare(builder);
                    let prepared_api_content = field_api_content.prepare(builder);

                    let mut table_writer: ::planus::table_writer::TableWriter<8> =
                        ::core::default::Default::default();
                    table_writer.write_entry::<::planus::Offset<str>>(0);
                    table_writer.write_entry::<::planus::Offset<[u8]>>(1);

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            object_writer.write::<_, _, 4>(&prepared_provider);
                            object_writer.write::<_, _, 4>(&prepared_api_content);
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<ProviderFrame>> for ProviderFrame {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<ProviderFrame> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<ProviderFrame>> for ProviderFrame {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<ProviderFrame>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<ProviderFrame> for ProviderFrame {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<ProviderFrame> {
                    ProviderFrame::create(builder, &self.provider, &self.api_content)
                }
            }

            /// Builder for serializing an instance of the [ProviderFrame] type.
            ///
            /// Can be created using the [ProviderFrame::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct ProviderFrameBuilder<State>(State);

            impl ProviderFrameBuilder<()> {
                /// Setter for the [`provider` field](ProviderFrame#structfield.provider).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn provider<T0>(self, value: T0) -> ProviderFrameBuilder<(T0,)>
                where
                    T0: ::planus::WriteAs<::planus::Offset<str>>,
                {
                    ProviderFrameBuilder((value,))
                }
            }

            impl<T0> ProviderFrameBuilder<(T0,)> {
                /// Setter for the [`api_content` field](ProviderFrame#structfield.api_content).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn api_content<T1>(self, value: T1) -> ProviderFrameBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAs<::planus::Offset<[u8]>>,
                {
                    let (v0,) = self.0;
                    ProviderFrameBuilder((v0, value))
                }
            }

            impl<T0, T1> ProviderFrameBuilder<(T0, T1)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [ProviderFrame].
                #[inline]
                pub fn finish(
                    self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<ProviderFrame>
                where
                    Self: ::planus::WriteAsOffset<ProviderFrame>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<str>>,
                T1: ::planus::WriteAs<::planus::Offset<[u8]>>,
            > ::planus::WriteAs<::planus::Offset<ProviderFrame>>
                for ProviderFrameBuilder<(T0, T1)>
            {
                type Prepared = ::planus::Offset<ProviderFrame>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<ProviderFrame> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<str>>,
                T1: ::planus::WriteAs<::planus::Offset<[u8]>>,
            > ::planus::WriteAsOptional<::planus::Offset<ProviderFrame>>
                for ProviderFrameBuilder<(T0, T1)>
            {
                type Prepared = ::planus::Offset<ProviderFrame>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<ProviderFrame>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<str>>,
                T1: ::planus::WriteAs<::planus::Offset<[u8]>>,
            > ::planus::WriteAsOffset<ProviderFrame> for ProviderFrameBuilder<(T0, T1)>
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<ProviderFrame> {
                    let (v0, v1) = &self.0;
                    ProviderFrame::create(builder, v0, v1)
                }
            }

            /// Reference to a deserialized [ProviderFrame].
            #[derive(Copy, Clone)]
            pub struct ProviderFrameRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> ProviderFrameRef<'a> {
                /// Getter for the [`provider` field](ProviderFrame#structfield.provider).
                #[inline]
                pub fn provider(&self) -> ::planus::Result<&'a ::core::primitive::str> {
                    self.0.access_required(0, "ProviderFrame", "provider")
                }

                /// Getter for the [`api_content` field](ProviderFrame#structfield.api_content).
                #[inline]
                pub fn api_content(&self) -> ::planus::Result<&'a [u8]> {
                    self.0.access_required(1, "ProviderFrame", "api_content")
                }
            }

            impl<'a> ::core::fmt::Debug for ProviderFrameRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("ProviderFrameRef");
                    f.field("provider", &self.provider());
                    f.field("api_content", &self.api_content());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<ProviderFrameRef<'a>> for ProviderFrame {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: ProviderFrameRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        provider: ::core::convert::Into::into(value.provider()?),
                        api_content: value.api_content()?.to_vec(),
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for ProviderFrameRef<'a> {
                #[inline]
                fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::ErrorKind> {
                    ::core::result::Result::Ok(Self(::planus::table_reader::Table::from_buffer(
                        buffer, offset,
                    )?))
                }
            }

            impl<'a> ::planus::VectorReadInner<'a> for ProviderFrameRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[ProviderFrameRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<ProviderFrame>> for ProviderFrame {
                type Value = ::planus::Offset<ProviderFrame>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<ProviderFrame>],
                    bytes: *mut ::core::mem::MaybeUninit<u8>,
                    buffer_position: u32,
                ) {
                    let bytes = bytes as *mut [::core::mem::MaybeUninit<u8>; 4];
                    for (i, v) in ::core::iter::Iterator::enumerate(values.iter()) {
                        ::planus::WriteAsPrimitive::write(
                            v,
                            ::planus::Cursor::new(unsafe { &mut *bytes.add(i) }),
                            buffer_position - (Self::STRIDE * i) as u32,
                        );
                    }
                }
            }

            impl<'a> ::planus::ReadAsRoot<'a> for ProviderFrameRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[ProviderFrameRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `MediaRef` in the namespace `hypermind.schema`
            ///
            /// Generated from these locations:
            /// * Table `MediaRef` in the file `schemas/events.fbs:65`
            #[derive(
                Clone,
                Debug,
                PartialEq,
                PartialOrd,
                Eq,
                Ord,
                Hash,
                ::serde::Serialize,
                ::serde::Deserialize,
            )]
            pub struct MediaRef {
                /// The field `uri` in the table `MediaRef`
                pub uri: ::planus::alloc::string::String,
                /// The field `media_type` in the table `MediaRef`
                pub media_type: ::planus::alloc::string::String,
                /// The field `digest` in the table `MediaRef`
                pub digest: ::planus::alloc::vec::Vec<u8>,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for MediaRef {
                fn default() -> Self {
                    Self {
                        uri: ::core::default::Default::default(),
                        media_type: ::core::default::Default::default(),
                        digest: ::core::default::Default::default(),
                    }
                }
            }

            impl MediaRef {
                /// Creates a [MediaRefBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> MediaRefBuilder<()> {
                    MediaRefBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_uri: impl ::planus::WriteAs<::planus::Offset<str>>,
                    field_media_type: impl ::planus::WriteAs<::planus::Offset<str>>,
                    field_digest: impl ::planus::WriteAs<::planus::Offset<[u8]>>,
                ) -> ::planus::Offset<Self> {
                    let prepared_uri = field_uri.prepare(builder);
                    let prepared_media_type = field_media_type.prepare(builder);
                    let prepared_digest = field_digest.prepare(builder);

                    let mut table_writer: ::planus::table_writer::TableWriter<10> =
                        ::core::default::Default::default();
                    table_writer.write_entry::<::planus::Offset<str>>(0);
                    table_writer.write_entry::<::planus::Offset<str>>(1);
                    table_writer.write_entry::<::planus::Offset<[u8]>>(2);

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            object_writer.write::<_, _, 4>(&prepared_uri);
                            object_writer.write::<_, _, 4>(&prepared_media_type);
                            object_writer.write::<_, _, 4>(&prepared_digest);
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<MediaRef>> for MediaRef {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<MediaRef> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<MediaRef>> for MediaRef {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<MediaRef>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<MediaRef> for MediaRef {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<MediaRef> {
                    MediaRef::create(builder, &self.uri, &self.media_type, &self.digest)
                }
            }

            /// Builder for serializing an instance of the [MediaRef] type.
            ///
            /// Can be created using the [MediaRef::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct MediaRefBuilder<State>(State);

            impl MediaRefBuilder<()> {
                /// Setter for the [`uri` field](MediaRef#structfield.uri).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn uri<T0>(self, value: T0) -> MediaRefBuilder<(T0,)>
                where
                    T0: ::planus::WriteAs<::planus::Offset<str>>,
                {
                    MediaRefBuilder((value,))
                }
            }

            impl<T0> MediaRefBuilder<(T0,)> {
                /// Setter for the [`media_type` field](MediaRef#structfield.media_type).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn media_type<T1>(self, value: T1) -> MediaRefBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAs<::planus::Offset<str>>,
                {
                    let (v0,) = self.0;
                    MediaRefBuilder((v0, value))
                }
            }

            impl<T0, T1> MediaRefBuilder<(T0, T1)> {
                /// Setter for the [`digest` field](MediaRef#structfield.digest).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn digest<T2>(self, value: T2) -> MediaRefBuilder<(T0, T1, T2)>
                where
                    T2: ::planus::WriteAs<::planus::Offset<[u8]>>,
                {
                    let (v0, v1) = self.0;
                    MediaRefBuilder((v0, v1, value))
                }
            }

            impl<T0, T1, T2> MediaRefBuilder<(T0, T1, T2)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [MediaRef].
                #[inline]
                pub fn finish(self, builder: &mut ::planus::Builder) -> ::planus::Offset<MediaRef>
                where
                    Self: ::planus::WriteAsOffset<MediaRef>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<str>>,
                T1: ::planus::WriteAs<::planus::Offset<str>>,
                T2: ::planus::WriteAs<::planus::Offset<[u8]>>,
            > ::planus::WriteAs<::planus::Offset<MediaRef>> for MediaRefBuilder<(T0, T1, T2)>
            {
                type Prepared = ::planus::Offset<MediaRef>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<MediaRef> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<str>>,
                T1: ::planus::WriteAs<::planus::Offset<str>>,
                T2: ::planus::WriteAs<::planus::Offset<[u8]>>,
            > ::planus::WriteAsOptional<::planus::Offset<MediaRef>>
                for MediaRefBuilder<(T0, T1, T2)>
            {
                type Prepared = ::planus::Offset<MediaRef>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<MediaRef>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<str>>,
                T1: ::planus::WriteAs<::planus::Offset<str>>,
                T2: ::planus::WriteAs<::planus::Offset<[u8]>>,
            > ::planus::WriteAsOffset<MediaRef> for MediaRefBuilder<(T0, T1, T2)>
            {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<MediaRef> {
                    let (v0, v1, v2) = &self.0;
                    MediaRef::create(builder, v0, v1, v2)
                }
            }

            /// Reference to a deserialized [MediaRef].
            #[derive(Copy, Clone)]
            pub struct MediaRefRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> MediaRefRef<'a> {
                /// Getter for the [`uri` field](MediaRef#structfield.uri).
                #[inline]
                pub fn uri(&self) -> ::planus::Result<&'a ::core::primitive::str> {
                    self.0.access_required(0, "MediaRef", "uri")
                }

                /// Getter for the [`media_type` field](MediaRef#structfield.media_type).
                #[inline]
                pub fn media_type(&self) -> ::planus::Result<&'a ::core::primitive::str> {
                    self.0.access_required(1, "MediaRef", "media_type")
                }

                /// Getter for the [`digest` field](MediaRef#structfield.digest).
                #[inline]
                pub fn digest(&self) -> ::planus::Result<&'a [u8]> {
                    self.0.access_required(2, "MediaRef", "digest")
                }
            }

            impl<'a> ::core::fmt::Debug for MediaRefRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("MediaRefRef");
                    f.field("uri", &self.uri());
                    f.field("media_type", &self.media_type());
                    f.field("digest", &self.digest());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<MediaRefRef<'a>> for MediaRef {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: MediaRefRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        uri: ::core::convert::Into::into(value.uri()?),
                        media_type: ::core::convert::Into::into(value.media_type()?),
                        digest: value.digest()?.to_vec(),
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for MediaRefRef<'a> {
                #[inline]
                fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::ErrorKind> {
                    ::core::result::Result::Ok(Self(::planus::table_reader::Table::from_buffer(
                        buffer, offset,
                    )?))
                }
            }

            impl<'a> ::planus::VectorReadInner<'a> for MediaRefRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[MediaRefRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<MediaRef>> for MediaRef {
                type Value = ::planus::Offset<MediaRef>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<MediaRef>],
                    bytes: *mut ::core::mem::MaybeUninit<u8>,
                    buffer_position: u32,
                ) {
                    let bytes = bytes as *mut [::core::mem::MaybeUninit<u8>; 4];
                    for (i, v) in ::core::iter::Iterator::enumerate(values.iter()) {
                        ::planus::WriteAsPrimitive::write(
                            v,
                            ::planus::Cursor::new(unsafe { &mut *bytes.add(i) }),
                            buffer_position - (Self::STRIDE * i) as u32,
                        );
                    }
                }
            }

            impl<'a> ::planus::ReadAsRoot<'a> for MediaRefRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[MediaRefRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `Effect` in the namespace `hypermind.schema`
            ///
            /// Generated from these locations:
            /// * Table `Effect` in the file `schemas/events.fbs:71`
            #[derive(
                Clone,
                Debug,
                PartialEq,
                PartialOrd,
                Eq,
                Ord,
                Hash,
                ::serde::Serialize,
                ::serde::Deserialize,
            )]
            pub struct Effect {
                /// The field `effect_id` in the table `Effect`
                pub effect_id: ::planus::alloc::vec::Vec<u8>,
                /// The field `tool_call_lsn` in the table `Effect`
                pub tool_call_lsn: u64,
                /// The field `state` in the table `Effect`
                pub state: self::EffectState,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for Effect {
                fn default() -> Self {
                    Self {
                        effect_id: ::core::default::Default::default(),
                        tool_call_lsn: 0,
                        state: self::EffectState::Dispatched,
                    }
                }
            }

            impl Effect {
                /// Creates a [EffectBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> EffectBuilder<()> {
                    EffectBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_effect_id: impl ::planus::WriteAs<::planus::Offset<[u8]>>,
                    field_tool_call_lsn: impl ::planus::WriteAsDefault<u64, u64>,
                    field_state: impl ::planus::WriteAsDefault<self::EffectState, self::EffectState>,
                ) -> ::planus::Offset<Self> {
                    let prepared_effect_id = field_effect_id.prepare(builder);
                    let prepared_tool_call_lsn = field_tool_call_lsn.prepare(builder, &0);
                    let prepared_state =
                        field_state.prepare(builder, &self::EffectState::Dispatched);

                    let mut table_writer: ::planus::table_writer::TableWriter<10> =
                        ::core::default::Default::default();
                    if prepared_tool_call_lsn.is_some() {
                        table_writer.write_entry::<u64>(1);
                    }
                    table_writer.write_entry::<::planus::Offset<[u8]>>(0);
                    if prepared_state.is_some() {
                        table_writer.write_entry::<self::EffectState>(2);
                    }

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            if let ::core::option::Option::Some(prepared_tool_call_lsn) =
                                prepared_tool_call_lsn
                            {
                                object_writer.write::<_, _, 8>(&prepared_tool_call_lsn);
                            }
                            object_writer.write::<_, _, 4>(&prepared_effect_id);
                            if let ::core::option::Option::Some(prepared_state) = prepared_state {
                                object_writer.write::<_, _, 1>(&prepared_state);
                            }
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<Effect>> for Effect {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Effect> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<Effect>> for Effect {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Effect>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<Effect> for Effect {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Effect> {
                    Effect::create(builder, &self.effect_id, self.tool_call_lsn, self.state)
                }
            }

            /// Builder for serializing an instance of the [Effect] type.
            ///
            /// Can be created using the [Effect::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct EffectBuilder<State>(State);

            impl EffectBuilder<()> {
                /// Setter for the [`effect_id` field](Effect#structfield.effect_id).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn effect_id<T0>(self, value: T0) -> EffectBuilder<(T0,)>
                where
                    T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                {
                    EffectBuilder((value,))
                }
            }

            impl<T0> EffectBuilder<(T0,)> {
                /// Setter for the [`tool_call_lsn` field](Effect#structfield.tool_call_lsn).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn tool_call_lsn<T1>(self, value: T1) -> EffectBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAsDefault<u64, u64>,
                {
                    let (v0,) = self.0;
                    EffectBuilder((v0, value))
                }

                /// Sets the [`tool_call_lsn` field](Effect#structfield.tool_call_lsn) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn tool_call_lsn_as_default(
                    self,
                ) -> EffectBuilder<(T0, ::planus::DefaultValue)> {
                    self.tool_call_lsn(::planus::DefaultValue)
                }
            }

            impl<T0, T1> EffectBuilder<(T0, T1)> {
                /// Setter for the [`state` field](Effect#structfield.state).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn state<T2>(self, value: T2) -> EffectBuilder<(T0, T1, T2)>
                where
                    T2: ::planus::WriteAsDefault<self::EffectState, self::EffectState>,
                {
                    let (v0, v1) = self.0;
                    EffectBuilder((v0, v1, value))
                }

                /// Sets the [`state` field](Effect#structfield.state) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn state_as_default(self) -> EffectBuilder<(T0, T1, ::planus::DefaultValue)> {
                    self.state(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2> EffectBuilder<(T0, T1, T2)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [Effect].
                #[inline]
                pub fn finish(self, builder: &mut ::planus::Builder) -> ::planus::Offset<Effect>
                where
                    Self: ::planus::WriteAsOffset<Effect>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T1: ::planus::WriteAsDefault<u64, u64>,
                T2: ::planus::WriteAsDefault<self::EffectState, self::EffectState>,
            > ::planus::WriteAs<::planus::Offset<Effect>> for EffectBuilder<(T0, T1, T2)>
            {
                type Prepared = ::planus::Offset<Effect>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Effect> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T1: ::planus::WriteAsDefault<u64, u64>,
                T2: ::planus::WriteAsDefault<self::EffectState, self::EffectState>,
            > ::planus::WriteAsOptional<::planus::Offset<Effect>> for EffectBuilder<(T0, T1, T2)>
            {
                type Prepared = ::planus::Offset<Effect>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Effect>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T1: ::planus::WriteAsDefault<u64, u64>,
                T2: ::planus::WriteAsDefault<self::EffectState, self::EffectState>,
            > ::planus::WriteAsOffset<Effect> for EffectBuilder<(T0, T1, T2)>
            {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Effect> {
                    let (v0, v1, v2) = &self.0;
                    Effect::create(builder, v0, v1, v2)
                }
            }

            /// Reference to a deserialized [Effect].
            #[derive(Copy, Clone)]
            pub struct EffectRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> EffectRef<'a> {
                /// Getter for the [`effect_id` field](Effect#structfield.effect_id).
                #[inline]
                pub fn effect_id(&self) -> ::planus::Result<&'a [u8]> {
                    self.0.access_required(0, "Effect", "effect_id")
                }

                /// Getter for the [`tool_call_lsn` field](Effect#structfield.tool_call_lsn).
                #[inline]
                pub fn tool_call_lsn(&self) -> ::planus::Result<u64> {
                    ::core::result::Result::Ok(
                        self.0.access(1, "Effect", "tool_call_lsn")?.unwrap_or(0),
                    )
                }

                /// Getter for the [`state` field](Effect#structfield.state).
                #[inline]
                pub fn state(&self) -> ::planus::Result<self::EffectState> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(2, "Effect", "state")?
                            .unwrap_or(self::EffectState::Dispatched),
                    )
                }
            }

            impl<'a> ::core::fmt::Debug for EffectRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("EffectRef");
                    f.field("effect_id", &self.effect_id());
                    f.field("tool_call_lsn", &self.tool_call_lsn());
                    f.field("state", &self.state());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<EffectRef<'a>> for Effect {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: EffectRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        effect_id: value.effect_id()?.to_vec(),
                        tool_call_lsn: ::core::convert::TryInto::try_into(value.tool_call_lsn()?)?,
                        state: ::core::convert::TryInto::try_into(value.state()?)?,
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for EffectRef<'a> {
                #[inline]
                fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::ErrorKind> {
                    ::core::result::Result::Ok(Self(::planus::table_reader::Table::from_buffer(
                        buffer, offset,
                    )?))
                }
            }

            impl<'a> ::planus::VectorReadInner<'a> for EffectRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[EffectRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<Effect>> for Effect {
                type Value = ::planus::Offset<Effect>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<Effect>],
                    bytes: *mut ::core::mem::MaybeUninit<u8>,
                    buffer_position: u32,
                ) {
                    let bytes = bytes as *mut [::core::mem::MaybeUninit<u8>; 4];
                    for (i, v) in ::core::iter::Iterator::enumerate(values.iter()) {
                        ::planus::WriteAsPrimitive::write(
                            v,
                            ::planus::Cursor::new(unsafe { &mut *bytes.add(i) }),
                            buffer_position - (Self::STRIDE * i) as u32,
                        );
                    }
                }
            }

            impl<'a> ::planus::ReadAsRoot<'a> for EffectRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[EffectRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `Approval` in the namespace `hypermind.schema`
            ///
            /// Generated from these locations:
            /// * Table `Approval` in the file `schemas/events.fbs:77`
            #[derive(
                Clone,
                Debug,
                PartialEq,
                PartialOrd,
                Eq,
                Ord,
                Hash,
                ::serde::Serialize,
                ::serde::Deserialize,
            )]
            pub struct Approval {
                /// The field `effect_id` in the table `Approval`
                pub effect_id: ::planus::alloc::vec::Vec<u8>,
                /// The field `decision` in the table `Approval`
                pub decision: self::ApprovalDecision,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for Approval {
                fn default() -> Self {
                    Self {
                        effect_id: ::core::default::Default::default(),
                        decision: self::ApprovalDecision::Approved,
                    }
                }
            }

            impl Approval {
                /// Creates a [ApprovalBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> ApprovalBuilder<()> {
                    ApprovalBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_effect_id: impl ::planus::WriteAs<::planus::Offset<[u8]>>,
                    field_decision: impl ::planus::WriteAsDefault<
                        self::ApprovalDecision,
                        self::ApprovalDecision,
                    >,
                ) -> ::planus::Offset<Self> {
                    let prepared_effect_id = field_effect_id.prepare(builder);
                    let prepared_decision =
                        field_decision.prepare(builder, &self::ApprovalDecision::Approved);

                    let mut table_writer: ::planus::table_writer::TableWriter<8> =
                        ::core::default::Default::default();
                    table_writer.write_entry::<::planus::Offset<[u8]>>(0);
                    if prepared_decision.is_some() {
                        table_writer.write_entry::<self::ApprovalDecision>(1);
                    }

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            object_writer.write::<_, _, 4>(&prepared_effect_id);
                            if let ::core::option::Option::Some(prepared_decision) =
                                prepared_decision
                            {
                                object_writer.write::<_, _, 1>(&prepared_decision);
                            }
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<Approval>> for Approval {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Approval> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<Approval>> for Approval {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Approval>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<Approval> for Approval {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Approval> {
                    Approval::create(builder, &self.effect_id, self.decision)
                }
            }

            /// Builder for serializing an instance of the [Approval] type.
            ///
            /// Can be created using the [Approval::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct ApprovalBuilder<State>(State);

            impl ApprovalBuilder<()> {
                /// Setter for the [`effect_id` field](Approval#structfield.effect_id).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn effect_id<T0>(self, value: T0) -> ApprovalBuilder<(T0,)>
                where
                    T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                {
                    ApprovalBuilder((value,))
                }
            }

            impl<T0> ApprovalBuilder<(T0,)> {
                /// Setter for the [`decision` field](Approval#structfield.decision).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn decision<T1>(self, value: T1) -> ApprovalBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAsDefault<self::ApprovalDecision, self::ApprovalDecision>,
                {
                    let (v0,) = self.0;
                    ApprovalBuilder((v0, value))
                }

                /// Sets the [`decision` field](Approval#structfield.decision) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn decision_as_default(self) -> ApprovalBuilder<(T0, ::planus::DefaultValue)> {
                    self.decision(::planus::DefaultValue)
                }
            }

            impl<T0, T1> ApprovalBuilder<(T0, T1)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [Approval].
                #[inline]
                pub fn finish(self, builder: &mut ::planus::Builder) -> ::planus::Offset<Approval>
                where
                    Self: ::planus::WriteAsOffset<Approval>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T1: ::planus::WriteAsDefault<self::ApprovalDecision, self::ApprovalDecision>,
            > ::planus::WriteAs<::planus::Offset<Approval>> for ApprovalBuilder<(T0, T1)>
            {
                type Prepared = ::planus::Offset<Approval>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Approval> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T1: ::planus::WriteAsDefault<self::ApprovalDecision, self::ApprovalDecision>,
            > ::planus::WriteAsOptional<::planus::Offset<Approval>> for ApprovalBuilder<(T0, T1)>
            {
                type Prepared = ::planus::Offset<Approval>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Approval>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T1: ::planus::WriteAsDefault<self::ApprovalDecision, self::ApprovalDecision>,
            > ::planus::WriteAsOffset<Approval> for ApprovalBuilder<(T0, T1)>
            {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Approval> {
                    let (v0, v1) = &self.0;
                    Approval::create(builder, v0, v1)
                }
            }

            /// Reference to a deserialized [Approval].
            #[derive(Copy, Clone)]
            pub struct ApprovalRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> ApprovalRef<'a> {
                /// Getter for the [`effect_id` field](Approval#structfield.effect_id).
                #[inline]
                pub fn effect_id(&self) -> ::planus::Result<&'a [u8]> {
                    self.0.access_required(0, "Approval", "effect_id")
                }

                /// Getter for the [`decision` field](Approval#structfield.decision).
                #[inline]
                pub fn decision(&self) -> ::planus::Result<self::ApprovalDecision> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(1, "Approval", "decision")?
                            .unwrap_or(self::ApprovalDecision::Approved),
                    )
                }
            }

            impl<'a> ::core::fmt::Debug for ApprovalRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("ApprovalRef");
                    f.field("effect_id", &self.effect_id());
                    f.field("decision", &self.decision());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<ApprovalRef<'a>> for Approval {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: ApprovalRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        effect_id: value.effect_id()?.to_vec(),
                        decision: ::core::convert::TryInto::try_into(value.decision()?)?,
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for ApprovalRef<'a> {
                #[inline]
                fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::ErrorKind> {
                    ::core::result::Result::Ok(Self(::planus::table_reader::Table::from_buffer(
                        buffer, offset,
                    )?))
                }
            }

            impl<'a> ::planus::VectorReadInner<'a> for ApprovalRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[ApprovalRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<Approval>> for Approval {
                type Value = ::planus::Offset<Approval>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<Approval>],
                    bytes: *mut ::core::mem::MaybeUninit<u8>,
                    buffer_position: u32,
                ) {
                    let bytes = bytes as *mut [::core::mem::MaybeUninit<u8>; 4];
                    for (i, v) in ::core::iter::Iterator::enumerate(values.iter()) {
                        ::planus::WriteAsPrimitive::write(
                            v,
                            ::planus::Cursor::new(unsafe { &mut *bytes.add(i) }),
                            buffer_position - (Self::STRIDE * i) as u32,
                        );
                    }
                }
            }

            impl<'a> ::planus::ReadAsRoot<'a> for ApprovalRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[ApprovalRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `Outcome` in the namespace `hypermind.schema`
            ///
            /// Generated from these locations:
            /// * Table `Outcome` in the file `schemas/events.fbs:82`
            #[derive(
                Clone,
                Debug,
                PartialEq,
                PartialOrd,
                Eq,
                Ord,
                Hash,
                ::serde::Serialize,
                ::serde::Deserialize,
            )]
            pub struct Outcome {
                /// The field `effect_id` in the table `Outcome`
                pub effect_id: ::planus::alloc::vec::Vec<u8>,
                /// The field `status` in the table `Outcome`
                pub status: self::ResultStatus,
                /// The field `detail` in the table `Outcome`
                pub detail: ::planus::alloc::vec::Vec<u8>,
                /// The field `evidence_lsns` in the table `Outcome`
                pub evidence_lsns: ::core::option::Option<::planus::alloc::vec::Vec<u64>>,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for Outcome {
                fn default() -> Self {
                    Self {
                        effect_id: ::core::default::Default::default(),
                        status: self::ResultStatus::Ok,
                        detail: ::core::default::Default::default(),
                        evidence_lsns: ::core::default::Default::default(),
                    }
                }
            }

            impl Outcome {
                /// Creates a [OutcomeBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> OutcomeBuilder<()> {
                    OutcomeBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_effect_id: impl ::planus::WriteAs<::planus::Offset<[u8]>>,
                    field_status: impl ::planus::WriteAsDefault<self::ResultStatus, self::ResultStatus>,
                    field_detail: impl ::planus::WriteAs<::planus::Offset<[u8]>>,
                    field_evidence_lsns: impl ::planus::WriteAsOptional<::planus::Offset<[u64]>>,
                ) -> ::planus::Offset<Self> {
                    let prepared_effect_id = field_effect_id.prepare(builder);
                    let prepared_status = field_status.prepare(builder, &self::ResultStatus::Ok);
                    let prepared_detail = field_detail.prepare(builder);
                    let prepared_evidence_lsns = field_evidence_lsns.prepare(builder);

                    let mut table_writer: ::planus::table_writer::TableWriter<12> =
                        ::core::default::Default::default();
                    table_writer.write_entry::<::planus::Offset<[u8]>>(0);
                    table_writer.write_entry::<::planus::Offset<[u8]>>(2);
                    if prepared_evidence_lsns.is_some() {
                        table_writer.write_entry::<::planus::Offset<[u64]>>(3);
                    }
                    if prepared_status.is_some() {
                        table_writer.write_entry::<self::ResultStatus>(1);
                    }

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            object_writer.write::<_, _, 4>(&prepared_effect_id);
                            object_writer.write::<_, _, 4>(&prepared_detail);
                            if let ::core::option::Option::Some(prepared_evidence_lsns) =
                                prepared_evidence_lsns
                            {
                                object_writer.write::<_, _, 4>(&prepared_evidence_lsns);
                            }
                            if let ::core::option::Option::Some(prepared_status) = prepared_status {
                                object_writer.write::<_, _, 1>(&prepared_status);
                            }
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<Outcome>> for Outcome {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Outcome> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<Outcome>> for Outcome {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Outcome>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<Outcome> for Outcome {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Outcome> {
                    Outcome::create(
                        builder,
                        &self.effect_id,
                        self.status,
                        &self.detail,
                        &self.evidence_lsns,
                    )
                }
            }

            /// Builder for serializing an instance of the [Outcome] type.
            ///
            /// Can be created using the [Outcome::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct OutcomeBuilder<State>(State);

            impl OutcomeBuilder<()> {
                /// Setter for the [`effect_id` field](Outcome#structfield.effect_id).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn effect_id<T0>(self, value: T0) -> OutcomeBuilder<(T0,)>
                where
                    T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                {
                    OutcomeBuilder((value,))
                }
            }

            impl<T0> OutcomeBuilder<(T0,)> {
                /// Setter for the [`status` field](Outcome#structfield.status).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn status<T1>(self, value: T1) -> OutcomeBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAsDefault<self::ResultStatus, self::ResultStatus>,
                {
                    let (v0,) = self.0;
                    OutcomeBuilder((v0, value))
                }

                /// Sets the [`status` field](Outcome#structfield.status) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn status_as_default(self) -> OutcomeBuilder<(T0, ::planus::DefaultValue)> {
                    self.status(::planus::DefaultValue)
                }
            }

            impl<T0, T1> OutcomeBuilder<(T0, T1)> {
                /// Setter for the [`detail` field](Outcome#structfield.detail).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn detail<T2>(self, value: T2) -> OutcomeBuilder<(T0, T1, T2)>
                where
                    T2: ::planus::WriteAs<::planus::Offset<[u8]>>,
                {
                    let (v0, v1) = self.0;
                    OutcomeBuilder((v0, v1, value))
                }
            }

            impl<T0, T1, T2> OutcomeBuilder<(T0, T1, T2)> {
                /// Setter for the [`evidence_lsns` field](Outcome#structfield.evidence_lsns).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn evidence_lsns<T3>(self, value: T3) -> OutcomeBuilder<(T0, T1, T2, T3)>
                where
                    T3: ::planus::WriteAsOptional<::planus::Offset<[u64]>>,
                {
                    let (v0, v1, v2) = self.0;
                    OutcomeBuilder((v0, v1, v2, value))
                }

                /// Sets the [`evidence_lsns` field](Outcome#structfield.evidence_lsns) to null.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn evidence_lsns_as_null(self) -> OutcomeBuilder<(T0, T1, T2, ())> {
                    self.evidence_lsns(())
                }
            }

            impl<T0, T1, T2, T3> OutcomeBuilder<(T0, T1, T2, T3)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [Outcome].
                #[inline]
                pub fn finish(self, builder: &mut ::planus::Builder) -> ::planus::Offset<Outcome>
                where
                    Self: ::planus::WriteAsOffset<Outcome>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T1: ::planus::WriteAsDefault<self::ResultStatus, self::ResultStatus>,
                T2: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T3: ::planus::WriteAsOptional<::planus::Offset<[u64]>>,
            > ::planus::WriteAs<::planus::Offset<Outcome>> for OutcomeBuilder<(T0, T1, T2, T3)>
            {
                type Prepared = ::planus::Offset<Outcome>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Outcome> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T1: ::planus::WriteAsDefault<self::ResultStatus, self::ResultStatus>,
                T2: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T3: ::planus::WriteAsOptional<::planus::Offset<[u64]>>,
            > ::planus::WriteAsOptional<::planus::Offset<Outcome>>
                for OutcomeBuilder<(T0, T1, T2, T3)>
            {
                type Prepared = ::planus::Offset<Outcome>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Outcome>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T1: ::planus::WriteAsDefault<self::ResultStatus, self::ResultStatus>,
                T2: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T3: ::planus::WriteAsOptional<::planus::Offset<[u64]>>,
            > ::planus::WriteAsOffset<Outcome> for OutcomeBuilder<(T0, T1, T2, T3)>
            {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Outcome> {
                    let (v0, v1, v2, v3) = &self.0;
                    Outcome::create(builder, v0, v1, v2, v3)
                }
            }

            /// Reference to a deserialized [Outcome].
            #[derive(Copy, Clone)]
            pub struct OutcomeRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> OutcomeRef<'a> {
                /// Getter for the [`effect_id` field](Outcome#structfield.effect_id).
                #[inline]
                pub fn effect_id(&self) -> ::planus::Result<&'a [u8]> {
                    self.0.access_required(0, "Outcome", "effect_id")
                }

                /// Getter for the [`status` field](Outcome#structfield.status).
                #[inline]
                pub fn status(&self) -> ::planus::Result<self::ResultStatus> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(1, "Outcome", "status")?
                            .unwrap_or(self::ResultStatus::Ok),
                    )
                }

                /// Getter for the [`detail` field](Outcome#structfield.detail).
                #[inline]
                pub fn detail(&self) -> ::planus::Result<&'a [u8]> {
                    self.0.access_required(2, "Outcome", "detail")
                }

                /// Getter for the [`evidence_lsns` field](Outcome#structfield.evidence_lsns).
                #[inline]
                pub fn evidence_lsns(
                    &self,
                ) -> ::planus::Result<::core::option::Option<::planus::Vector<'a, u64>>>
                {
                    self.0.access(3, "Outcome", "evidence_lsns")
                }
            }

            impl<'a> ::core::fmt::Debug for OutcomeRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("OutcomeRef");
                    f.field("effect_id", &self.effect_id());
                    f.field("status", &self.status());
                    f.field("detail", &self.detail());
                    if let ::core::option::Option::Some(field_evidence_lsns) =
                        self.evidence_lsns().transpose()
                    {
                        f.field("evidence_lsns", &field_evidence_lsns);
                    }
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<OutcomeRef<'a>> for Outcome {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: OutcomeRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        effect_id: value.effect_id()?.to_vec(),
                        status: ::core::convert::TryInto::try_into(value.status()?)?,
                        detail: value.detail()?.to_vec(),
                        evidence_lsns: if let ::core::option::Option::Some(evidence_lsns) =
                            value.evidence_lsns()?
                        {
                            ::core::option::Option::Some(evidence_lsns.to_vec()?)
                        } else {
                            ::core::option::Option::None
                        },
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for OutcomeRef<'a> {
                #[inline]
                fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::ErrorKind> {
                    ::core::result::Result::Ok(Self(::planus::table_reader::Table::from_buffer(
                        buffer, offset,
                    )?))
                }
            }

            impl<'a> ::planus::VectorReadInner<'a> for OutcomeRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[OutcomeRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<Outcome>> for Outcome {
                type Value = ::planus::Offset<Outcome>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<Outcome>],
                    bytes: *mut ::core::mem::MaybeUninit<u8>,
                    buffer_position: u32,
                ) {
                    let bytes = bytes as *mut [::core::mem::MaybeUninit<u8>; 4];
                    for (i, v) in ::core::iter::Iterator::enumerate(values.iter()) {
                        ::planus::WriteAsPrimitive::write(
                            v,
                            ::planus::Cursor::new(unsafe { &mut *bytes.add(i) }),
                            buffer_position - (Self::STRIDE * i) as u32,
                        );
                    }
                }
            }

            impl<'a> ::planus::ReadAsRoot<'a> for OutcomeRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[OutcomeRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `Checkpoint` in the namespace `hypermind.schema`
            ///
            /// Generated from these locations:
            /// * Table `Checkpoint` in the file `schemas/events.fbs:89`
            #[derive(
                Clone,
                Debug,
                PartialEq,
                PartialOrd,
                Eq,
                Ord,
                Hash,
                ::serde::Serialize,
                ::serde::Deserialize,
            )]
            pub struct Checkpoint {
                /// The field `cursor` in the table `Checkpoint`
                pub cursor: ::planus::alloc::vec::Vec<u8>,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for Checkpoint {
                fn default() -> Self {
                    Self {
                        cursor: ::core::default::Default::default(),
                    }
                }
            }

            impl Checkpoint {
                /// Creates a [CheckpointBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> CheckpointBuilder<()> {
                    CheckpointBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_cursor: impl ::planus::WriteAs<::planus::Offset<[u8]>>,
                ) -> ::planus::Offset<Self> {
                    let prepared_cursor = field_cursor.prepare(builder);

                    let mut table_writer: ::planus::table_writer::TableWriter<6> =
                        ::core::default::Default::default();
                    table_writer.write_entry::<::planus::Offset<[u8]>>(0);

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            object_writer.write::<_, _, 4>(&prepared_cursor);
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<Checkpoint>> for Checkpoint {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Checkpoint> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<Checkpoint>> for Checkpoint {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Checkpoint>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<Checkpoint> for Checkpoint {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Checkpoint> {
                    Checkpoint::create(builder, &self.cursor)
                }
            }

            /// Builder for serializing an instance of the [Checkpoint] type.
            ///
            /// Can be created using the [Checkpoint::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct CheckpointBuilder<State>(State);

            impl CheckpointBuilder<()> {
                /// Setter for the [`cursor` field](Checkpoint#structfield.cursor).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn cursor<T0>(self, value: T0) -> CheckpointBuilder<(T0,)>
                where
                    T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                {
                    CheckpointBuilder((value,))
                }
            }

            impl<T0> CheckpointBuilder<(T0,)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [Checkpoint].
                #[inline]
                pub fn finish(self, builder: &mut ::planus::Builder) -> ::planus::Offset<Checkpoint>
                where
                    Self: ::planus::WriteAsOffset<Checkpoint>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<T0: ::planus::WriteAs<::planus::Offset<[u8]>>>
                ::planus::WriteAs<::planus::Offset<Checkpoint>> for CheckpointBuilder<(T0,)>
            {
                type Prepared = ::planus::Offset<Checkpoint>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Checkpoint> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<T0: ::planus::WriteAs<::planus::Offset<[u8]>>>
                ::planus::WriteAsOptional<::planus::Offset<Checkpoint>>
                for CheckpointBuilder<(T0,)>
            {
                type Prepared = ::planus::Offset<Checkpoint>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Checkpoint>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<T0: ::planus::WriteAs<::planus::Offset<[u8]>>> ::planus::WriteAsOffset<Checkpoint>
                for CheckpointBuilder<(T0,)>
            {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Checkpoint> {
                    let (v0,) = &self.0;
                    Checkpoint::create(builder, v0)
                }
            }

            /// Reference to a deserialized [Checkpoint].
            #[derive(Copy, Clone)]
            pub struct CheckpointRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> CheckpointRef<'a> {
                /// Getter for the [`cursor` field](Checkpoint#structfield.cursor).
                #[inline]
                pub fn cursor(&self) -> ::planus::Result<&'a [u8]> {
                    self.0.access_required(0, "Checkpoint", "cursor")
                }
            }

            impl<'a> ::core::fmt::Debug for CheckpointRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("CheckpointRef");
                    f.field("cursor", &self.cursor());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<CheckpointRef<'a>> for Checkpoint {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: CheckpointRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        cursor: value.cursor()?.to_vec(),
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for CheckpointRef<'a> {
                #[inline]
                fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::ErrorKind> {
                    ::core::result::Result::Ok(Self(::planus::table_reader::Table::from_buffer(
                        buffer, offset,
                    )?))
                }
            }

            impl<'a> ::planus::VectorReadInner<'a> for CheckpointRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[CheckpointRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<Checkpoint>> for Checkpoint {
                type Value = ::planus::Offset<Checkpoint>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<Checkpoint>],
                    bytes: *mut ::core::mem::MaybeUninit<u8>,
                    buffer_position: u32,
                ) {
                    let bytes = bytes as *mut [::core::mem::MaybeUninit<u8>; 4];
                    for (i, v) in ::core::iter::Iterator::enumerate(values.iter()) {
                        ::planus::WriteAsPrimitive::write(
                            v,
                            ::planus::Cursor::new(unsafe { &mut *bytes.add(i) }),
                            buffer_position - (Self::STRIDE * i) as u32,
                        );
                    }
                }
            }

            impl<'a> ::planus::ReadAsRoot<'a> for CheckpointRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[CheckpointRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `Supervisor` in the namespace `hypermind.schema`
            ///
            /// Generated from these locations:
            /// * Table `Supervisor` in the file `schemas/events.fbs:93`
            #[derive(
                Clone,
                Debug,
                PartialEq,
                PartialOrd,
                Eq,
                Ord,
                Hash,
                ::serde::Serialize,
                ::serde::Deserialize,
            )]
            pub struct Supervisor {
                /// The field `code` in the table `Supervisor`
                pub code: ::planus::alloc::string::String,
                /// The field `evidence` in the table `Supervisor`
                pub evidence: ::planus::alloc::vec::Vec<u8>,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for Supervisor {
                fn default() -> Self {
                    Self {
                        code: ::core::default::Default::default(),
                        evidence: ::core::default::Default::default(),
                    }
                }
            }

            impl Supervisor {
                /// Creates a [SupervisorBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> SupervisorBuilder<()> {
                    SupervisorBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_code: impl ::planus::WriteAs<::planus::Offset<str>>,
                    field_evidence: impl ::planus::WriteAs<::planus::Offset<[u8]>>,
                ) -> ::planus::Offset<Self> {
                    let prepared_code = field_code.prepare(builder);
                    let prepared_evidence = field_evidence.prepare(builder);

                    let mut table_writer: ::planus::table_writer::TableWriter<8> =
                        ::core::default::Default::default();
                    table_writer.write_entry::<::planus::Offset<str>>(0);
                    table_writer.write_entry::<::planus::Offset<[u8]>>(1);

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            object_writer.write::<_, _, 4>(&prepared_code);
                            object_writer.write::<_, _, 4>(&prepared_evidence);
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<Supervisor>> for Supervisor {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Supervisor> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<Supervisor>> for Supervisor {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Supervisor>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<Supervisor> for Supervisor {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Supervisor> {
                    Supervisor::create(builder, &self.code, &self.evidence)
                }
            }

            /// Builder for serializing an instance of the [Supervisor] type.
            ///
            /// Can be created using the [Supervisor::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct SupervisorBuilder<State>(State);

            impl SupervisorBuilder<()> {
                /// Setter for the [`code` field](Supervisor#structfield.code).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn code<T0>(self, value: T0) -> SupervisorBuilder<(T0,)>
                where
                    T0: ::planus::WriteAs<::planus::Offset<str>>,
                {
                    SupervisorBuilder((value,))
                }
            }

            impl<T0> SupervisorBuilder<(T0,)> {
                /// Setter for the [`evidence` field](Supervisor#structfield.evidence).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn evidence<T1>(self, value: T1) -> SupervisorBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAs<::planus::Offset<[u8]>>,
                {
                    let (v0,) = self.0;
                    SupervisorBuilder((v0, value))
                }
            }

            impl<T0, T1> SupervisorBuilder<(T0, T1)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [Supervisor].
                #[inline]
                pub fn finish(self, builder: &mut ::planus::Builder) -> ::planus::Offset<Supervisor>
                where
                    Self: ::planus::WriteAsOffset<Supervisor>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<str>>,
                T1: ::planus::WriteAs<::planus::Offset<[u8]>>,
            > ::planus::WriteAs<::planus::Offset<Supervisor>> for SupervisorBuilder<(T0, T1)>
            {
                type Prepared = ::planus::Offset<Supervisor>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Supervisor> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<str>>,
                T1: ::planus::WriteAs<::planus::Offset<[u8]>>,
            > ::planus::WriteAsOptional<::planus::Offset<Supervisor>>
                for SupervisorBuilder<(T0, T1)>
            {
                type Prepared = ::planus::Offset<Supervisor>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Supervisor>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<str>>,
                T1: ::planus::WriteAs<::planus::Offset<[u8]>>,
            > ::planus::WriteAsOffset<Supervisor> for SupervisorBuilder<(T0, T1)>
            {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Supervisor> {
                    let (v0, v1) = &self.0;
                    Supervisor::create(builder, v0, v1)
                }
            }

            /// Reference to a deserialized [Supervisor].
            #[derive(Copy, Clone)]
            pub struct SupervisorRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> SupervisorRef<'a> {
                /// Getter for the [`code` field](Supervisor#structfield.code).
                #[inline]
                pub fn code(&self) -> ::planus::Result<&'a ::core::primitive::str> {
                    self.0.access_required(0, "Supervisor", "code")
                }

                /// Getter for the [`evidence` field](Supervisor#structfield.evidence).
                #[inline]
                pub fn evidence(&self) -> ::planus::Result<&'a [u8]> {
                    self.0.access_required(1, "Supervisor", "evidence")
                }
            }

            impl<'a> ::core::fmt::Debug for SupervisorRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("SupervisorRef");
                    f.field("code", &self.code());
                    f.field("evidence", &self.evidence());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<SupervisorRef<'a>> for Supervisor {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: SupervisorRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        code: ::core::convert::Into::into(value.code()?),
                        evidence: value.evidence()?.to_vec(),
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for SupervisorRef<'a> {
                #[inline]
                fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::ErrorKind> {
                    ::core::result::Result::Ok(Self(::planus::table_reader::Table::from_buffer(
                        buffer, offset,
                    )?))
                }
            }

            impl<'a> ::planus::VectorReadInner<'a> for SupervisorRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[SupervisorRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<Supervisor>> for Supervisor {
                type Value = ::planus::Offset<Supervisor>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<Supervisor>],
                    bytes: *mut ::core::mem::MaybeUninit<u8>,
                    buffer_position: u32,
                ) {
                    let bytes = bytes as *mut [::core::mem::MaybeUninit<u8>; 4];
                    for (i, v) in ::core::iter::Iterator::enumerate(values.iter()) {
                        ::planus::WriteAsPrimitive::write(
                            v,
                            ::planus::Cursor::new(unsafe { &mut *bytes.add(i) }),
                            buffer_position - (Self::STRIDE * i) as u32,
                        );
                    }
                }
            }

            impl<'a> ::planus::ReadAsRoot<'a> for SupervisorRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[SupervisorRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `Recovery` in the namespace `hypermind.schema`
            ///
            /// Generated from these locations:
            /// * Table `Recovery` in the file `schemas/events.fbs:98`
            #[derive(
                Clone,
                Debug,
                PartialEq,
                PartialOrd,
                Eq,
                Ord,
                Hash,
                ::serde::Serialize,
                ::serde::Deserialize,
            )]
            pub struct Recovery {
                /// The field `code` in the table `Recovery`
                pub code: ::planus::alloc::string::String,
                /// The field `target_lsn` in the table `Recovery`
                pub target_lsn: u64,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for Recovery {
                fn default() -> Self {
                    Self {
                        code: ::core::default::Default::default(),
                        target_lsn: 0,
                    }
                }
            }

            impl Recovery {
                /// Creates a [RecoveryBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> RecoveryBuilder<()> {
                    RecoveryBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_code: impl ::planus::WriteAs<::planus::Offset<str>>,
                    field_target_lsn: impl ::planus::WriteAsDefault<u64, u64>,
                ) -> ::planus::Offset<Self> {
                    let prepared_code = field_code.prepare(builder);
                    let prepared_target_lsn = field_target_lsn.prepare(builder, &0);

                    let mut table_writer: ::planus::table_writer::TableWriter<8> =
                        ::core::default::Default::default();
                    if prepared_target_lsn.is_some() {
                        table_writer.write_entry::<u64>(1);
                    }
                    table_writer.write_entry::<::planus::Offset<str>>(0);

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            if let ::core::option::Option::Some(prepared_target_lsn) =
                                prepared_target_lsn
                            {
                                object_writer.write::<_, _, 8>(&prepared_target_lsn);
                            }
                            object_writer.write::<_, _, 4>(&prepared_code);
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<Recovery>> for Recovery {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Recovery> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<Recovery>> for Recovery {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Recovery>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<Recovery> for Recovery {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Recovery> {
                    Recovery::create(builder, &self.code, self.target_lsn)
                }
            }

            /// Builder for serializing an instance of the [Recovery] type.
            ///
            /// Can be created using the [Recovery::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct RecoveryBuilder<State>(State);

            impl RecoveryBuilder<()> {
                /// Setter for the [`code` field](Recovery#structfield.code).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn code<T0>(self, value: T0) -> RecoveryBuilder<(T0,)>
                where
                    T0: ::planus::WriteAs<::planus::Offset<str>>,
                {
                    RecoveryBuilder((value,))
                }
            }

            impl<T0> RecoveryBuilder<(T0,)> {
                /// Setter for the [`target_lsn` field](Recovery#structfield.target_lsn).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn target_lsn<T1>(self, value: T1) -> RecoveryBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAsDefault<u64, u64>,
                {
                    let (v0,) = self.0;
                    RecoveryBuilder((v0, value))
                }

                /// Sets the [`target_lsn` field](Recovery#structfield.target_lsn) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn target_lsn_as_default(
                    self,
                ) -> RecoveryBuilder<(T0, ::planus::DefaultValue)> {
                    self.target_lsn(::planus::DefaultValue)
                }
            }

            impl<T0, T1> RecoveryBuilder<(T0, T1)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [Recovery].
                #[inline]
                pub fn finish(self, builder: &mut ::planus::Builder) -> ::planus::Offset<Recovery>
                where
                    Self: ::planus::WriteAsOffset<Recovery>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<str>>,
                T1: ::planus::WriteAsDefault<u64, u64>,
            > ::planus::WriteAs<::planus::Offset<Recovery>> for RecoveryBuilder<(T0, T1)>
            {
                type Prepared = ::planus::Offset<Recovery>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Recovery> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<str>>,
                T1: ::planus::WriteAsDefault<u64, u64>,
            > ::planus::WriteAsOptional<::planus::Offset<Recovery>> for RecoveryBuilder<(T0, T1)>
            {
                type Prepared = ::planus::Offset<Recovery>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Recovery>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<str>>,
                T1: ::planus::WriteAsDefault<u64, u64>,
            > ::planus::WriteAsOffset<Recovery> for RecoveryBuilder<(T0, T1)>
            {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Recovery> {
                    let (v0, v1) = &self.0;
                    Recovery::create(builder, v0, v1)
                }
            }

            /// Reference to a deserialized [Recovery].
            #[derive(Copy, Clone)]
            pub struct RecoveryRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> RecoveryRef<'a> {
                /// Getter for the [`code` field](Recovery#structfield.code).
                #[inline]
                pub fn code(&self) -> ::planus::Result<&'a ::core::primitive::str> {
                    self.0.access_required(0, "Recovery", "code")
                }

                /// Getter for the [`target_lsn` field](Recovery#structfield.target_lsn).
                #[inline]
                pub fn target_lsn(&self) -> ::planus::Result<u64> {
                    ::core::result::Result::Ok(
                        self.0.access(1, "Recovery", "target_lsn")?.unwrap_or(0),
                    )
                }
            }

            impl<'a> ::core::fmt::Debug for RecoveryRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("RecoveryRef");
                    f.field("code", &self.code());
                    f.field("target_lsn", &self.target_lsn());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<RecoveryRef<'a>> for Recovery {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: RecoveryRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        code: ::core::convert::Into::into(value.code()?),
                        target_lsn: ::core::convert::TryInto::try_into(value.target_lsn()?)?,
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for RecoveryRef<'a> {
                #[inline]
                fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::ErrorKind> {
                    ::core::result::Result::Ok(Self(::planus::table_reader::Table::from_buffer(
                        buffer, offset,
                    )?))
                }
            }

            impl<'a> ::planus::VectorReadInner<'a> for RecoveryRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[RecoveryRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<Recovery>> for Recovery {
                type Value = ::planus::Offset<Recovery>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<Recovery>],
                    bytes: *mut ::core::mem::MaybeUninit<u8>,
                    buffer_position: u32,
                ) {
                    let bytes = bytes as *mut [::core::mem::MaybeUninit<u8>; 4];
                    for (i, v) in ::core::iter::Iterator::enumerate(values.iter()) {
                        ::planus::WriteAsPrimitive::write(
                            v,
                            ::planus::Cursor::new(unsafe { &mut *bytes.add(i) }),
                            buffer_position - (Self::STRIDE * i) as u32,
                        );
                    }
                }
            }

            impl<'a> ::planus::ReadAsRoot<'a> for RecoveryRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[RecoveryRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `IntentSet` in the namespace `hypermind.schema`
            ///
            /// Generated from these locations:
            /// * Table `IntentSet` in the file `schemas/events.fbs:103`
            #[derive(
                Clone,
                Debug,
                PartialEq,
                PartialOrd,
                Eq,
                Ord,
                Hash,
                ::serde::Serialize,
                ::serde::Deserialize,
            )]
            pub struct IntentSet {
                /// The field `objective` in the table `IntentSet`
                pub objective: ::planus::alloc::vec::Vec<u8>,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for IntentSet {
                fn default() -> Self {
                    Self {
                        objective: ::core::default::Default::default(),
                    }
                }
            }

            impl IntentSet {
                /// Creates a [IntentSetBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> IntentSetBuilder<()> {
                    IntentSetBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_objective: impl ::planus::WriteAs<::planus::Offset<[u8]>>,
                ) -> ::planus::Offset<Self> {
                    let prepared_objective = field_objective.prepare(builder);

                    let mut table_writer: ::planus::table_writer::TableWriter<6> =
                        ::core::default::Default::default();
                    table_writer.write_entry::<::planus::Offset<[u8]>>(0);

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            object_writer.write::<_, _, 4>(&prepared_objective);
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<IntentSet>> for IntentSet {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<IntentSet> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<IntentSet>> for IntentSet {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<IntentSet>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<IntentSet> for IntentSet {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<IntentSet> {
                    IntentSet::create(builder, &self.objective)
                }
            }

            /// Builder for serializing an instance of the [IntentSet] type.
            ///
            /// Can be created using the [IntentSet::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct IntentSetBuilder<State>(State);

            impl IntentSetBuilder<()> {
                /// Setter for the [`objective` field](IntentSet#structfield.objective).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn objective<T0>(self, value: T0) -> IntentSetBuilder<(T0,)>
                where
                    T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                {
                    IntentSetBuilder((value,))
                }
            }

            impl<T0> IntentSetBuilder<(T0,)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [IntentSet].
                #[inline]
                pub fn finish(self, builder: &mut ::planus::Builder) -> ::planus::Offset<IntentSet>
                where
                    Self: ::planus::WriteAsOffset<IntentSet>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<T0: ::planus::WriteAs<::planus::Offset<[u8]>>>
                ::planus::WriteAs<::planus::Offset<IntentSet>> for IntentSetBuilder<(T0,)>
            {
                type Prepared = ::planus::Offset<IntentSet>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<IntentSet> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<T0: ::planus::WriteAs<::planus::Offset<[u8]>>>
                ::planus::WriteAsOptional<::planus::Offset<IntentSet>> for IntentSetBuilder<(T0,)>
            {
                type Prepared = ::planus::Offset<IntentSet>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<IntentSet>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<T0: ::planus::WriteAs<::planus::Offset<[u8]>>> ::planus::WriteAsOffset<IntentSet>
                for IntentSetBuilder<(T0,)>
            {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<IntentSet> {
                    let (v0,) = &self.0;
                    IntentSet::create(builder, v0)
                }
            }

            /// Reference to a deserialized [IntentSet].
            #[derive(Copy, Clone)]
            pub struct IntentSetRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> IntentSetRef<'a> {
                /// Getter for the [`objective` field](IntentSet#structfield.objective).
                #[inline]
                pub fn objective(&self) -> ::planus::Result<&'a [u8]> {
                    self.0.access_required(0, "IntentSet", "objective")
                }
            }

            impl<'a> ::core::fmt::Debug for IntentSetRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("IntentSetRef");
                    f.field("objective", &self.objective());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<IntentSetRef<'a>> for IntentSet {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: IntentSetRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        objective: value.objective()?.to_vec(),
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for IntentSetRef<'a> {
                #[inline]
                fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::ErrorKind> {
                    ::core::result::Result::Ok(Self(::planus::table_reader::Table::from_buffer(
                        buffer, offset,
                    )?))
                }
            }

            impl<'a> ::planus::VectorReadInner<'a> for IntentSetRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[IntentSetRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<IntentSet>> for IntentSet {
                type Value = ::planus::Offset<IntentSet>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<IntentSet>],
                    bytes: *mut ::core::mem::MaybeUninit<u8>,
                    buffer_position: u32,
                ) {
                    let bytes = bytes as *mut [::core::mem::MaybeUninit<u8>; 4];
                    for (i, v) in ::core::iter::Iterator::enumerate(values.iter()) {
                        ::planus::WriteAsPrimitive::write(
                            v,
                            ::planus::Cursor::new(unsafe { &mut *bytes.add(i) }),
                            buffer_position - (Self::STRIDE * i) as u32,
                        );
                    }
                }
            }

            impl<'a> ::planus::ReadAsRoot<'a> for IntentSetRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[IntentSetRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `LoopOpened` in the namespace `hypermind.schema`
            ///
            /// Generated from these locations:
            /// * Table `LoopOpened` in the file `schemas/events.fbs:107`
            #[derive(
                Clone,
                Debug,
                PartialEq,
                PartialOrd,
                Eq,
                Ord,
                Hash,
                ::serde::Serialize,
                ::serde::Deserialize,
            )]
            pub struct LoopOpened {
                /// The field `loop_id` in the table `LoopOpened`
                pub loop_id: ::planus::alloc::vec::Vec<u8>,
                /// The field `objective` in the table `LoopOpened`
                pub objective: ::planus::alloc::vec::Vec<u8>,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for LoopOpened {
                fn default() -> Self {
                    Self {
                        loop_id: ::core::default::Default::default(),
                        objective: ::core::default::Default::default(),
                    }
                }
            }

            impl LoopOpened {
                /// Creates a [LoopOpenedBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> LoopOpenedBuilder<()> {
                    LoopOpenedBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_loop_id: impl ::planus::WriteAs<::planus::Offset<[u8]>>,
                    field_objective: impl ::planus::WriteAs<::planus::Offset<[u8]>>,
                ) -> ::planus::Offset<Self> {
                    let prepared_loop_id = field_loop_id.prepare(builder);
                    let prepared_objective = field_objective.prepare(builder);

                    let mut table_writer: ::planus::table_writer::TableWriter<8> =
                        ::core::default::Default::default();
                    table_writer.write_entry::<::planus::Offset<[u8]>>(0);
                    table_writer.write_entry::<::planus::Offset<[u8]>>(1);

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            object_writer.write::<_, _, 4>(&prepared_loop_id);
                            object_writer.write::<_, _, 4>(&prepared_objective);
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<LoopOpened>> for LoopOpened {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<LoopOpened> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<LoopOpened>> for LoopOpened {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<LoopOpened>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<LoopOpened> for LoopOpened {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<LoopOpened> {
                    LoopOpened::create(builder, &self.loop_id, &self.objective)
                }
            }

            /// Builder for serializing an instance of the [LoopOpened] type.
            ///
            /// Can be created using the [LoopOpened::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct LoopOpenedBuilder<State>(State);

            impl LoopOpenedBuilder<()> {
                /// Setter for the [`loop_id` field](LoopOpened#structfield.loop_id).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn loop_id<T0>(self, value: T0) -> LoopOpenedBuilder<(T0,)>
                where
                    T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                {
                    LoopOpenedBuilder((value,))
                }
            }

            impl<T0> LoopOpenedBuilder<(T0,)> {
                /// Setter for the [`objective` field](LoopOpened#structfield.objective).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn objective<T1>(self, value: T1) -> LoopOpenedBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAs<::planus::Offset<[u8]>>,
                {
                    let (v0,) = self.0;
                    LoopOpenedBuilder((v0, value))
                }
            }

            impl<T0, T1> LoopOpenedBuilder<(T0, T1)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [LoopOpened].
                #[inline]
                pub fn finish(self, builder: &mut ::planus::Builder) -> ::planus::Offset<LoopOpened>
                where
                    Self: ::planus::WriteAsOffset<LoopOpened>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T1: ::planus::WriteAs<::planus::Offset<[u8]>>,
            > ::planus::WriteAs<::planus::Offset<LoopOpened>> for LoopOpenedBuilder<(T0, T1)>
            {
                type Prepared = ::planus::Offset<LoopOpened>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<LoopOpened> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T1: ::planus::WriteAs<::planus::Offset<[u8]>>,
            > ::planus::WriteAsOptional<::planus::Offset<LoopOpened>>
                for LoopOpenedBuilder<(T0, T1)>
            {
                type Prepared = ::planus::Offset<LoopOpened>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<LoopOpened>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T1: ::planus::WriteAs<::planus::Offset<[u8]>>,
            > ::planus::WriteAsOffset<LoopOpened> for LoopOpenedBuilder<(T0, T1)>
            {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<LoopOpened> {
                    let (v0, v1) = &self.0;
                    LoopOpened::create(builder, v0, v1)
                }
            }

            /// Reference to a deserialized [LoopOpened].
            #[derive(Copy, Clone)]
            pub struct LoopOpenedRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> LoopOpenedRef<'a> {
                /// Getter for the [`loop_id` field](LoopOpened#structfield.loop_id).
                #[inline]
                pub fn loop_id(&self) -> ::planus::Result<&'a [u8]> {
                    self.0.access_required(0, "LoopOpened", "loop_id")
                }

                /// Getter for the [`objective` field](LoopOpened#structfield.objective).
                #[inline]
                pub fn objective(&self) -> ::planus::Result<&'a [u8]> {
                    self.0.access_required(1, "LoopOpened", "objective")
                }
            }

            impl<'a> ::core::fmt::Debug for LoopOpenedRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("LoopOpenedRef");
                    f.field("loop_id", &self.loop_id());
                    f.field("objective", &self.objective());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<LoopOpenedRef<'a>> for LoopOpened {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: LoopOpenedRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        loop_id: value.loop_id()?.to_vec(),
                        objective: value.objective()?.to_vec(),
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for LoopOpenedRef<'a> {
                #[inline]
                fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::ErrorKind> {
                    ::core::result::Result::Ok(Self(::planus::table_reader::Table::from_buffer(
                        buffer, offset,
                    )?))
                }
            }

            impl<'a> ::planus::VectorReadInner<'a> for LoopOpenedRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[LoopOpenedRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<LoopOpened>> for LoopOpened {
                type Value = ::planus::Offset<LoopOpened>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<LoopOpened>],
                    bytes: *mut ::core::mem::MaybeUninit<u8>,
                    buffer_position: u32,
                ) {
                    let bytes = bytes as *mut [::core::mem::MaybeUninit<u8>; 4];
                    for (i, v) in ::core::iter::Iterator::enumerate(values.iter()) {
                        ::planus::WriteAsPrimitive::write(
                            v,
                            ::planus::Cursor::new(unsafe { &mut *bytes.add(i) }),
                            buffer_position - (Self::STRIDE * i) as u32,
                        );
                    }
                }
            }

            impl<'a> ::planus::ReadAsRoot<'a> for LoopOpenedRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[LoopOpenedRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `LoopClosed` in the namespace `hypermind.schema`
            ///
            /// Generated from these locations:
            /// * Table `LoopClosed` in the file `schemas/events.fbs:112`
            #[derive(
                Clone,
                Debug,
                PartialEq,
                PartialOrd,
                Eq,
                Ord,
                Hash,
                ::serde::Serialize,
                ::serde::Deserialize,
            )]
            pub struct LoopClosed {
                /// The field `loop_id` in the table `LoopClosed`
                pub loop_id: ::planus::alloc::vec::Vec<u8>,
                /// The field `reason` in the table `LoopClosed`
                pub reason: self::LoopCloseReason,
                /// The field `cause` in the table `LoopClosed`
                pub cause: ::planus::alloc::vec::Vec<u8>,
                /// The field `evidence_lsns` in the table `LoopClosed`
                pub evidence_lsns: ::core::option::Option<::planus::alloc::vec::Vec<u64>>,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for LoopClosed {
                fn default() -> Self {
                    Self {
                        loop_id: ::core::default::Default::default(),
                        reason: self::LoopCloseReason::Done,
                        cause: ::core::default::Default::default(),
                        evidence_lsns: ::core::default::Default::default(),
                    }
                }
            }

            impl LoopClosed {
                /// Creates a [LoopClosedBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> LoopClosedBuilder<()> {
                    LoopClosedBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_loop_id: impl ::planus::WriteAs<::planus::Offset<[u8]>>,
                    field_reason: impl ::planus::WriteAsDefault<
                        self::LoopCloseReason,
                        self::LoopCloseReason,
                    >,
                    field_cause: impl ::planus::WriteAs<::planus::Offset<[u8]>>,
                    field_evidence_lsns: impl ::planus::WriteAsOptional<::planus::Offset<[u64]>>,
                ) -> ::planus::Offset<Self> {
                    let prepared_loop_id = field_loop_id.prepare(builder);
                    let prepared_reason =
                        field_reason.prepare(builder, &self::LoopCloseReason::Done);
                    let prepared_cause = field_cause.prepare(builder);
                    let prepared_evidence_lsns = field_evidence_lsns.prepare(builder);

                    let mut table_writer: ::planus::table_writer::TableWriter<12> =
                        ::core::default::Default::default();
                    table_writer.write_entry::<::planus::Offset<[u8]>>(0);
                    table_writer.write_entry::<::planus::Offset<[u8]>>(2);
                    if prepared_evidence_lsns.is_some() {
                        table_writer.write_entry::<::planus::Offset<[u64]>>(3);
                    }
                    if prepared_reason.is_some() {
                        table_writer.write_entry::<self::LoopCloseReason>(1);
                    }

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            object_writer.write::<_, _, 4>(&prepared_loop_id);
                            object_writer.write::<_, _, 4>(&prepared_cause);
                            if let ::core::option::Option::Some(prepared_evidence_lsns) =
                                prepared_evidence_lsns
                            {
                                object_writer.write::<_, _, 4>(&prepared_evidence_lsns);
                            }
                            if let ::core::option::Option::Some(prepared_reason) = prepared_reason {
                                object_writer.write::<_, _, 1>(&prepared_reason);
                            }
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<LoopClosed>> for LoopClosed {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<LoopClosed> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<LoopClosed>> for LoopClosed {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<LoopClosed>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<LoopClosed> for LoopClosed {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<LoopClosed> {
                    LoopClosed::create(
                        builder,
                        &self.loop_id,
                        self.reason,
                        &self.cause,
                        &self.evidence_lsns,
                    )
                }
            }

            /// Builder for serializing an instance of the [LoopClosed] type.
            ///
            /// Can be created using the [LoopClosed::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct LoopClosedBuilder<State>(State);

            impl LoopClosedBuilder<()> {
                /// Setter for the [`loop_id` field](LoopClosed#structfield.loop_id).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn loop_id<T0>(self, value: T0) -> LoopClosedBuilder<(T0,)>
                where
                    T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                {
                    LoopClosedBuilder((value,))
                }
            }

            impl<T0> LoopClosedBuilder<(T0,)> {
                /// Setter for the [`reason` field](LoopClosed#structfield.reason).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn reason<T1>(self, value: T1) -> LoopClosedBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAsDefault<self::LoopCloseReason, self::LoopCloseReason>,
                {
                    let (v0,) = self.0;
                    LoopClosedBuilder((v0, value))
                }

                /// Sets the [`reason` field](LoopClosed#structfield.reason) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn reason_as_default(self) -> LoopClosedBuilder<(T0, ::planus::DefaultValue)> {
                    self.reason(::planus::DefaultValue)
                }
            }

            impl<T0, T1> LoopClosedBuilder<(T0, T1)> {
                /// Setter for the [`cause` field](LoopClosed#structfield.cause).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn cause<T2>(self, value: T2) -> LoopClosedBuilder<(T0, T1, T2)>
                where
                    T2: ::planus::WriteAs<::planus::Offset<[u8]>>,
                {
                    let (v0, v1) = self.0;
                    LoopClosedBuilder((v0, v1, value))
                }
            }

            impl<T0, T1, T2> LoopClosedBuilder<(T0, T1, T2)> {
                /// Setter for the [`evidence_lsns` field](LoopClosed#structfield.evidence_lsns).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn evidence_lsns<T3>(self, value: T3) -> LoopClosedBuilder<(T0, T1, T2, T3)>
                where
                    T3: ::planus::WriteAsOptional<::planus::Offset<[u64]>>,
                {
                    let (v0, v1, v2) = self.0;
                    LoopClosedBuilder((v0, v1, v2, value))
                }

                /// Sets the [`evidence_lsns` field](LoopClosed#structfield.evidence_lsns) to null.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn evidence_lsns_as_null(self) -> LoopClosedBuilder<(T0, T1, T2, ())> {
                    self.evidence_lsns(())
                }
            }

            impl<T0, T1, T2, T3> LoopClosedBuilder<(T0, T1, T2, T3)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [LoopClosed].
                #[inline]
                pub fn finish(self, builder: &mut ::planus::Builder) -> ::planus::Offset<LoopClosed>
                where
                    Self: ::planus::WriteAsOffset<LoopClosed>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T1: ::planus::WriteAsDefault<self::LoopCloseReason, self::LoopCloseReason>,
                T2: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T3: ::planus::WriteAsOptional<::planus::Offset<[u64]>>,
            > ::planus::WriteAs<::planus::Offset<LoopClosed>>
                for LoopClosedBuilder<(T0, T1, T2, T3)>
            {
                type Prepared = ::planus::Offset<LoopClosed>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<LoopClosed> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T1: ::planus::WriteAsDefault<self::LoopCloseReason, self::LoopCloseReason>,
                T2: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T3: ::planus::WriteAsOptional<::planus::Offset<[u64]>>,
            > ::planus::WriteAsOptional<::planus::Offset<LoopClosed>>
                for LoopClosedBuilder<(T0, T1, T2, T3)>
            {
                type Prepared = ::planus::Offset<LoopClosed>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<LoopClosed>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T1: ::planus::WriteAsDefault<self::LoopCloseReason, self::LoopCloseReason>,
                T2: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T3: ::planus::WriteAsOptional<::planus::Offset<[u64]>>,
            > ::planus::WriteAsOffset<LoopClosed> for LoopClosedBuilder<(T0, T1, T2, T3)>
            {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<LoopClosed> {
                    let (v0, v1, v2, v3) = &self.0;
                    LoopClosed::create(builder, v0, v1, v2, v3)
                }
            }

            /// Reference to a deserialized [LoopClosed].
            #[derive(Copy, Clone)]
            pub struct LoopClosedRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> LoopClosedRef<'a> {
                /// Getter for the [`loop_id` field](LoopClosed#structfield.loop_id).
                #[inline]
                pub fn loop_id(&self) -> ::planus::Result<&'a [u8]> {
                    self.0.access_required(0, "LoopClosed", "loop_id")
                }

                /// Getter for the [`reason` field](LoopClosed#structfield.reason).
                #[inline]
                pub fn reason(&self) -> ::planus::Result<self::LoopCloseReason> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(1, "LoopClosed", "reason")?
                            .unwrap_or(self::LoopCloseReason::Done),
                    )
                }

                /// Getter for the [`cause` field](LoopClosed#structfield.cause).
                #[inline]
                pub fn cause(&self) -> ::planus::Result<&'a [u8]> {
                    self.0.access_required(2, "LoopClosed", "cause")
                }

                /// Getter for the [`evidence_lsns` field](LoopClosed#structfield.evidence_lsns).
                #[inline]
                pub fn evidence_lsns(
                    &self,
                ) -> ::planus::Result<::core::option::Option<::planus::Vector<'a, u64>>>
                {
                    self.0.access(3, "LoopClosed", "evidence_lsns")
                }
            }

            impl<'a> ::core::fmt::Debug for LoopClosedRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("LoopClosedRef");
                    f.field("loop_id", &self.loop_id());
                    f.field("reason", &self.reason());
                    f.field("cause", &self.cause());
                    if let ::core::option::Option::Some(field_evidence_lsns) =
                        self.evidence_lsns().transpose()
                    {
                        f.field("evidence_lsns", &field_evidence_lsns);
                    }
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<LoopClosedRef<'a>> for LoopClosed {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: LoopClosedRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        loop_id: value.loop_id()?.to_vec(),
                        reason: ::core::convert::TryInto::try_into(value.reason()?)?,
                        cause: value.cause()?.to_vec(),
                        evidence_lsns: if let ::core::option::Option::Some(evidence_lsns) =
                            value.evidence_lsns()?
                        {
                            ::core::option::Option::Some(evidence_lsns.to_vec()?)
                        } else {
                            ::core::option::Option::None
                        },
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for LoopClosedRef<'a> {
                #[inline]
                fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::ErrorKind> {
                    ::core::result::Result::Ok(Self(::planus::table_reader::Table::from_buffer(
                        buffer, offset,
                    )?))
                }
            }

            impl<'a> ::planus::VectorReadInner<'a> for LoopClosedRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[LoopClosedRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<LoopClosed>> for LoopClosed {
                type Value = ::planus::Offset<LoopClosed>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<LoopClosed>],
                    bytes: *mut ::core::mem::MaybeUninit<u8>,
                    buffer_position: u32,
                ) {
                    let bytes = bytes as *mut [::core::mem::MaybeUninit<u8>; 4];
                    for (i, v) in ::core::iter::Iterator::enumerate(values.iter()) {
                        ::planus::WriteAsPrimitive::write(
                            v,
                            ::planus::Cursor::new(unsafe { &mut *bytes.add(i) }),
                            buffer_position - (Self::STRIDE * i) as u32,
                        );
                    }
                }
            }

            impl<'a> ::planus::ReadAsRoot<'a> for LoopClosedRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[LoopClosedRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `Binding` in the namespace `hypermind.schema`
            ///
            /// Generated from these locations:
            /// * Table `Binding` in the file `schemas/events.fbs:119`
            #[derive(
                Clone,
                Debug,
                PartialEq,
                PartialOrd,
                Eq,
                Ord,
                Hash,
                ::serde::Serialize,
                ::serde::Deserialize,
            )]
            pub struct Binding {
                /// The field `task` in the table `Binding`
                pub task: ::core::option::Option<::planus::alloc::vec::Vec<u8>>,
                /// The field `scope` in the table `Binding`
                pub scope: ::core::option::Option<::planus::alloc::vec::Vec<u8>>,
                /// The field `canonical_entity` in the table `Binding`
                pub canonical_entity: ::planus::alloc::string::String,
                /// The field `property` in the table `Binding`
                pub property: ::planus::alloc::string::String,
                /// The field `evidence_lsn` in the table `Binding`
                pub evidence_lsn: u64,
                /// The field `revision` in the table `Binding`
                pub revision: ::planus::alloc::vec::Vec<u8>,
                /// The field `freshness_requirement_ns` in the table `Binding`
                pub freshness_requirement_ns: u64,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for Binding {
                fn default() -> Self {
                    Self {
                        task: ::core::default::Default::default(),
                        scope: ::core::default::Default::default(),
                        canonical_entity: ::core::default::Default::default(),
                        property: ::core::default::Default::default(),
                        evidence_lsn: 0,
                        revision: ::core::default::Default::default(),
                        freshness_requirement_ns: 0,
                    }
                }
            }

            impl Binding {
                /// Creates a [BindingBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> BindingBuilder<()> {
                    BindingBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_task: impl ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                    field_scope: impl ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                    field_canonical_entity: impl ::planus::WriteAs<::planus::Offset<str>>,
                    field_property: impl ::planus::WriteAs<::planus::Offset<str>>,
                    field_evidence_lsn: impl ::planus::WriteAsDefault<u64, u64>,
                    field_revision: impl ::planus::WriteAs<::planus::Offset<[u8]>>,
                    field_freshness_requirement_ns: impl ::planus::WriteAsDefault<u64, u64>,
                ) -> ::planus::Offset<Self> {
                    let prepared_task = field_task.prepare(builder);
                    let prepared_scope = field_scope.prepare(builder);
                    let prepared_canonical_entity = field_canonical_entity.prepare(builder);
                    let prepared_property = field_property.prepare(builder);
                    let prepared_evidence_lsn = field_evidence_lsn.prepare(builder, &0);
                    let prepared_revision = field_revision.prepare(builder);
                    let prepared_freshness_requirement_ns =
                        field_freshness_requirement_ns.prepare(builder, &0);

                    let mut table_writer: ::planus::table_writer::TableWriter<18> =
                        ::core::default::Default::default();
                    if prepared_evidence_lsn.is_some() {
                        table_writer.write_entry::<u64>(4);
                    }
                    if prepared_freshness_requirement_ns.is_some() {
                        table_writer.write_entry::<u64>(6);
                    }
                    if prepared_task.is_some() {
                        table_writer.write_entry::<::planus::Offset<[u8]>>(0);
                    }
                    if prepared_scope.is_some() {
                        table_writer.write_entry::<::planus::Offset<[u8]>>(1);
                    }
                    table_writer.write_entry::<::planus::Offset<str>>(2);
                    table_writer.write_entry::<::planus::Offset<str>>(3);
                    table_writer.write_entry::<::planus::Offset<[u8]>>(5);

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            if let ::core::option::Option::Some(prepared_evidence_lsn) =
                                prepared_evidence_lsn
                            {
                                object_writer.write::<_, _, 8>(&prepared_evidence_lsn);
                            }
                            if let ::core::option::Option::Some(prepared_freshness_requirement_ns) =
                                prepared_freshness_requirement_ns
                            {
                                object_writer.write::<_, _, 8>(&prepared_freshness_requirement_ns);
                            }
                            if let ::core::option::Option::Some(prepared_task) = prepared_task {
                                object_writer.write::<_, _, 4>(&prepared_task);
                            }
                            if let ::core::option::Option::Some(prepared_scope) = prepared_scope {
                                object_writer.write::<_, _, 4>(&prepared_scope);
                            }
                            object_writer.write::<_, _, 4>(&prepared_canonical_entity);
                            object_writer.write::<_, _, 4>(&prepared_property);
                            object_writer.write::<_, _, 4>(&prepared_revision);
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<Binding>> for Binding {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Binding> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<Binding>> for Binding {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Binding>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<Binding> for Binding {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Binding> {
                    Binding::create(
                        builder,
                        &self.task,
                        &self.scope,
                        &self.canonical_entity,
                        &self.property,
                        self.evidence_lsn,
                        &self.revision,
                        self.freshness_requirement_ns,
                    )
                }
            }

            /// Builder for serializing an instance of the [Binding] type.
            ///
            /// Can be created using the [Binding::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct BindingBuilder<State>(State);

            impl BindingBuilder<()> {
                /// Setter for the [`task` field](Binding#structfield.task).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn task<T0>(self, value: T0) -> BindingBuilder<(T0,)>
                where
                    T0: ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                {
                    BindingBuilder((value,))
                }

                /// Sets the [`task` field](Binding#structfield.task) to null.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn task_as_null(self) -> BindingBuilder<((),)> {
                    self.task(())
                }
            }

            impl<T0> BindingBuilder<(T0,)> {
                /// Setter for the [`scope` field](Binding#structfield.scope).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn scope<T1>(self, value: T1) -> BindingBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                {
                    let (v0,) = self.0;
                    BindingBuilder((v0, value))
                }

                /// Sets the [`scope` field](Binding#structfield.scope) to null.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn scope_as_null(self) -> BindingBuilder<(T0, ())> {
                    self.scope(())
                }
            }

            impl<T0, T1> BindingBuilder<(T0, T1)> {
                /// Setter for the [`canonical_entity` field](Binding#structfield.canonical_entity).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn canonical_entity<T2>(self, value: T2) -> BindingBuilder<(T0, T1, T2)>
                where
                    T2: ::planus::WriteAs<::planus::Offset<str>>,
                {
                    let (v0, v1) = self.0;
                    BindingBuilder((v0, v1, value))
                }
            }

            impl<T0, T1, T2> BindingBuilder<(T0, T1, T2)> {
                /// Setter for the [`property` field](Binding#structfield.property).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn property<T3>(self, value: T3) -> BindingBuilder<(T0, T1, T2, T3)>
                where
                    T3: ::planus::WriteAs<::planus::Offset<str>>,
                {
                    let (v0, v1, v2) = self.0;
                    BindingBuilder((v0, v1, v2, value))
                }
            }

            impl<T0, T1, T2, T3> BindingBuilder<(T0, T1, T2, T3)> {
                /// Setter for the [`evidence_lsn` field](Binding#structfield.evidence_lsn).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn evidence_lsn<T4>(self, value: T4) -> BindingBuilder<(T0, T1, T2, T3, T4)>
                where
                    T4: ::planus::WriteAsDefault<u64, u64>,
                {
                    let (v0, v1, v2, v3) = self.0;
                    BindingBuilder((v0, v1, v2, v3, value))
                }

                /// Sets the [`evidence_lsn` field](Binding#structfield.evidence_lsn) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn evidence_lsn_as_default(
                    self,
                ) -> BindingBuilder<(T0, T1, T2, T3, ::planus::DefaultValue)> {
                    self.evidence_lsn(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2, T3, T4> BindingBuilder<(T0, T1, T2, T3, T4)> {
                /// Setter for the [`revision` field](Binding#structfield.revision).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn revision<T5>(self, value: T5) -> BindingBuilder<(T0, T1, T2, T3, T4, T5)>
                where
                    T5: ::planus::WriteAs<::planus::Offset<[u8]>>,
                {
                    let (v0, v1, v2, v3, v4) = self.0;
                    BindingBuilder((v0, v1, v2, v3, v4, value))
                }
            }

            impl<T0, T1, T2, T3, T4, T5> BindingBuilder<(T0, T1, T2, T3, T4, T5)> {
                /// Setter for the [`freshness_requirement_ns` field](Binding#structfield.freshness_requirement_ns).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn freshness_requirement_ns<T6>(
                    self,
                    value: T6,
                ) -> BindingBuilder<(T0, T1, T2, T3, T4, T5, T6)>
                where
                    T6: ::planus::WriteAsDefault<u64, u64>,
                {
                    let (v0, v1, v2, v3, v4, v5) = self.0;
                    BindingBuilder((v0, v1, v2, v3, v4, v5, value))
                }

                /// Sets the [`freshness_requirement_ns` field](Binding#structfield.freshness_requirement_ns) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn freshness_requirement_ns_as_default(
                    self,
                ) -> BindingBuilder<(T0, T1, T2, T3, T4, T5, ::planus::DefaultValue)>
                {
                    self.freshness_requirement_ns(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2, T3, T4, T5, T6> BindingBuilder<(T0, T1, T2, T3, T4, T5, T6)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [Binding].
                #[inline]
                pub fn finish(self, builder: &mut ::planus::Builder) -> ::planus::Offset<Binding>
                where
                    Self: ::planus::WriteAsOffset<Binding>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                T1: ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                T2: ::planus::WriteAs<::planus::Offset<str>>,
                T3: ::planus::WriteAs<::planus::Offset<str>>,
                T4: ::planus::WriteAsDefault<u64, u64>,
                T5: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T6: ::planus::WriteAsDefault<u64, u64>,
            > ::planus::WriteAs<::planus::Offset<Binding>>
                for BindingBuilder<(T0, T1, T2, T3, T4, T5, T6)>
            {
                type Prepared = ::planus::Offset<Binding>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Binding> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                T1: ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                T2: ::planus::WriteAs<::planus::Offset<str>>,
                T3: ::planus::WriteAs<::planus::Offset<str>>,
                T4: ::planus::WriteAsDefault<u64, u64>,
                T5: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T6: ::planus::WriteAsDefault<u64, u64>,
            > ::planus::WriteAsOptional<::planus::Offset<Binding>>
                for BindingBuilder<(T0, T1, T2, T3, T4, T5, T6)>
            {
                type Prepared = ::planus::Offset<Binding>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Binding>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                T1: ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                T2: ::planus::WriteAs<::planus::Offset<str>>,
                T3: ::planus::WriteAs<::planus::Offset<str>>,
                T4: ::planus::WriteAsDefault<u64, u64>,
                T5: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T6: ::planus::WriteAsDefault<u64, u64>,
            > ::planus::WriteAsOffset<Binding> for BindingBuilder<(T0, T1, T2, T3, T4, T5, T6)>
            {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Binding> {
                    let (v0, v1, v2, v3, v4, v5, v6) = &self.0;
                    Binding::create(builder, v0, v1, v2, v3, v4, v5, v6)
                }
            }

            /// Reference to a deserialized [Binding].
            #[derive(Copy, Clone)]
            pub struct BindingRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> BindingRef<'a> {
                /// Getter for the [`task` field](Binding#structfield.task).
                #[inline]
                pub fn task(&self) -> ::planus::Result<::core::option::Option<&'a [u8]>> {
                    self.0.access(0, "Binding", "task")
                }

                /// Getter for the [`scope` field](Binding#structfield.scope).
                #[inline]
                pub fn scope(&self) -> ::planus::Result<::core::option::Option<&'a [u8]>> {
                    self.0.access(1, "Binding", "scope")
                }

                /// Getter for the [`canonical_entity` field](Binding#structfield.canonical_entity).
                #[inline]
                pub fn canonical_entity(&self) -> ::planus::Result<&'a ::core::primitive::str> {
                    self.0.access_required(2, "Binding", "canonical_entity")
                }

                /// Getter for the [`property` field](Binding#structfield.property).
                #[inline]
                pub fn property(&self) -> ::planus::Result<&'a ::core::primitive::str> {
                    self.0.access_required(3, "Binding", "property")
                }

                /// Getter for the [`evidence_lsn` field](Binding#structfield.evidence_lsn).
                #[inline]
                pub fn evidence_lsn(&self) -> ::planus::Result<u64> {
                    ::core::result::Result::Ok(
                        self.0.access(4, "Binding", "evidence_lsn")?.unwrap_or(0),
                    )
                }

                /// Getter for the [`revision` field](Binding#structfield.revision).
                #[inline]
                pub fn revision(&self) -> ::planus::Result<&'a [u8]> {
                    self.0.access_required(5, "Binding", "revision")
                }

                /// Getter for the [`freshness_requirement_ns` field](Binding#structfield.freshness_requirement_ns).
                #[inline]
                pub fn freshness_requirement_ns(&self) -> ::planus::Result<u64> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(6, "Binding", "freshness_requirement_ns")?
                            .unwrap_or(0),
                    )
                }
            }

            impl<'a> ::core::fmt::Debug for BindingRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("BindingRef");
                    if let ::core::option::Option::Some(field_task) = self.task().transpose() {
                        f.field("task", &field_task);
                    }
                    if let ::core::option::Option::Some(field_scope) = self.scope().transpose() {
                        f.field("scope", &field_scope);
                    }
                    f.field("canonical_entity", &self.canonical_entity());
                    f.field("property", &self.property());
                    f.field("evidence_lsn", &self.evidence_lsn());
                    f.field("revision", &self.revision());
                    f.field("freshness_requirement_ns", &self.freshness_requirement_ns());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<BindingRef<'a>> for Binding {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: BindingRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        task: value.task()?.map(|v| v.to_vec()),
                        scope: value.scope()?.map(|v| v.to_vec()),
                        canonical_entity: ::core::convert::Into::into(value.canonical_entity()?),
                        property: ::core::convert::Into::into(value.property()?),
                        evidence_lsn: ::core::convert::TryInto::try_into(value.evidence_lsn()?)?,
                        revision: value.revision()?.to_vec(),
                        freshness_requirement_ns: ::core::convert::TryInto::try_into(
                            value.freshness_requirement_ns()?,
                        )?,
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for BindingRef<'a> {
                #[inline]
                fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::ErrorKind> {
                    ::core::result::Result::Ok(Self(::planus::table_reader::Table::from_buffer(
                        buffer, offset,
                    )?))
                }
            }

            impl<'a> ::planus::VectorReadInner<'a> for BindingRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[BindingRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<Binding>> for Binding {
                type Value = ::planus::Offset<Binding>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<Binding>],
                    bytes: *mut ::core::mem::MaybeUninit<u8>,
                    buffer_position: u32,
                ) {
                    let bytes = bytes as *mut [::core::mem::MaybeUninit<u8>; 4];
                    for (i, v) in ::core::iter::Iterator::enumerate(values.iter()) {
                        ::planus::WriteAsPrimitive::write(
                            v,
                            ::planus::Cursor::new(unsafe { &mut *bytes.add(i) }),
                            buffer_position - (Self::STRIDE * i) as u32,
                        );
                    }
                }
            }

            impl<'a> ::planus::ReadAsRoot<'a> for BindingRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[BindingRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `Assertion` in the namespace `hypermind.schema`
            ///
            /// Generated from these locations:
            /// * Table `Assertion` in the file `schemas/events.fbs:129`
            #[derive(
                Clone,
                Debug,
                PartialEq,
                PartialOrd,
                Eq,
                Ord,
                Hash,
                ::serde::Serialize,
                ::serde::Deserialize,
            )]
            pub struct Assertion {
                /// The field `belief_id` in the table `Assertion`
                pub belief_id: ::planus::alloc::vec::Vec<u8>,
                /// The field `belief_type` in the table `Assertion`
                pub belief_type: self::BeliefType,
                /// The field `canonical_identity` in the table `Assertion`
                pub canonical_identity: ::planus::alloc::string::String,
                /// The field `value` in the table `Assertion`
                pub value: ::planus::alloc::vec::Vec<u8>,
                /// The field `valid_from_ns` in the table `Assertion`
                pub valid_from_ns: i64,
                /// The field `valid_to_ns` in the table `Assertion`
                pub valid_to_ns: i64,
                /// The field `provenance` in the table `Assertion`
                pub provenance: ::planus::alloc::vec::Vec<self::ProvenanceRange>,
                /// The field `conflict_domain` in the table `Assertion`
                pub conflict_domain: ::core::option::Option<::planus::alloc::string::String>,
                /// The field `claim` in the table `Assertion`
                pub claim: self::AssertionClaim,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for Assertion {
                fn default() -> Self {
                    Self {
                        belief_id: ::core::default::Default::default(),
                        belief_type: self::BeliefType::Fact,
                        canonical_identity: ::core::default::Default::default(),
                        value: ::core::default::Default::default(),
                        valid_from_ns: 0,
                        valid_to_ns: 0,
                        provenance: ::core::default::Default::default(),
                        conflict_domain: ::core::default::Default::default(),
                        claim: self::AssertionClaim::Affirmative,
                    }
                }
            }

            impl Assertion {
                /// Creates a [AssertionBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> AssertionBuilder<()> {
                    AssertionBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_belief_id: impl ::planus::WriteAs<::planus::Offset<[u8]>>,
                    field_belief_type: impl ::planus::WriteAsDefault<self::BeliefType, self::BeliefType>,
                    field_canonical_identity: impl ::planus::WriteAs<::planus::Offset<str>>,
                    field_value: impl ::planus::WriteAs<::planus::Offset<[u8]>>,
                    field_valid_from_ns: impl ::planus::WriteAsDefault<i64, i64>,
                    field_valid_to_ns: impl ::planus::WriteAsDefault<i64, i64>,
                    field_provenance: impl ::planus::WriteAs<::planus::Offset<[self::ProvenanceRange]>>,
                    field_conflict_domain: impl ::planus::WriteAsOptional<
                        ::planus::Offset<::core::primitive::str>,
                    >,
                    field_claim: impl ::planus::WriteAsDefault<
                        self::AssertionClaim,
                        self::AssertionClaim,
                    >,
                ) -> ::planus::Offset<Self> {
                    let prepared_belief_id = field_belief_id.prepare(builder);
                    let prepared_belief_type =
                        field_belief_type.prepare(builder, &self::BeliefType::Fact);
                    let prepared_canonical_identity = field_canonical_identity.prepare(builder);
                    let prepared_value = field_value.prepare(builder);
                    let prepared_valid_from_ns = field_valid_from_ns.prepare(builder, &0);
                    let prepared_valid_to_ns = field_valid_to_ns.prepare(builder, &0);
                    let prepared_provenance = field_provenance.prepare(builder);
                    let prepared_conflict_domain = field_conflict_domain.prepare(builder);
                    let prepared_claim =
                        field_claim.prepare(builder, &self::AssertionClaim::Affirmative);

                    let mut table_writer: ::planus::table_writer::TableWriter<22> =
                        ::core::default::Default::default();
                    if prepared_valid_from_ns.is_some() {
                        table_writer.write_entry::<i64>(4);
                    }
                    if prepared_valid_to_ns.is_some() {
                        table_writer.write_entry::<i64>(5);
                    }
                    table_writer.write_entry::<::planus::Offset<[u8]>>(0);
                    table_writer.write_entry::<::planus::Offset<str>>(2);
                    table_writer.write_entry::<::planus::Offset<[u8]>>(3);
                    table_writer.write_entry::<::planus::Offset<[self::ProvenanceRange]>>(6);
                    if prepared_conflict_domain.is_some() {
                        table_writer.write_entry::<::planus::Offset<str>>(7);
                    }
                    if prepared_belief_type.is_some() {
                        table_writer.write_entry::<self::BeliefType>(1);
                    }
                    if prepared_claim.is_some() {
                        table_writer.write_entry::<self::AssertionClaim>(8);
                    }

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            if let ::core::option::Option::Some(prepared_valid_from_ns) =
                                prepared_valid_from_ns
                            {
                                object_writer.write::<_, _, 8>(&prepared_valid_from_ns);
                            }
                            if let ::core::option::Option::Some(prepared_valid_to_ns) =
                                prepared_valid_to_ns
                            {
                                object_writer.write::<_, _, 8>(&prepared_valid_to_ns);
                            }
                            object_writer.write::<_, _, 4>(&prepared_belief_id);
                            object_writer.write::<_, _, 4>(&prepared_canonical_identity);
                            object_writer.write::<_, _, 4>(&prepared_value);
                            object_writer.write::<_, _, 4>(&prepared_provenance);
                            if let ::core::option::Option::Some(prepared_conflict_domain) =
                                prepared_conflict_domain
                            {
                                object_writer.write::<_, _, 4>(&prepared_conflict_domain);
                            }
                            if let ::core::option::Option::Some(prepared_belief_type) =
                                prepared_belief_type
                            {
                                object_writer.write::<_, _, 1>(&prepared_belief_type);
                            }
                            if let ::core::option::Option::Some(prepared_claim) = prepared_claim {
                                object_writer.write::<_, _, 1>(&prepared_claim);
                            }
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<Assertion>> for Assertion {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Assertion> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<Assertion>> for Assertion {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Assertion>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<Assertion> for Assertion {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Assertion> {
                    Assertion::create(
                        builder,
                        &self.belief_id,
                        self.belief_type,
                        &self.canonical_identity,
                        &self.value,
                        self.valid_from_ns,
                        self.valid_to_ns,
                        &self.provenance,
                        &self.conflict_domain,
                        self.claim,
                    )
                }
            }

            /// Builder for serializing an instance of the [Assertion] type.
            ///
            /// Can be created using the [Assertion::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct AssertionBuilder<State>(State);

            impl AssertionBuilder<()> {
                /// Setter for the [`belief_id` field](Assertion#structfield.belief_id).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn belief_id<T0>(self, value: T0) -> AssertionBuilder<(T0,)>
                where
                    T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                {
                    AssertionBuilder((value,))
                }
            }

            impl<T0> AssertionBuilder<(T0,)> {
                /// Setter for the [`belief_type` field](Assertion#structfield.belief_type).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn belief_type<T1>(self, value: T1) -> AssertionBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAsDefault<self::BeliefType, self::BeliefType>,
                {
                    let (v0,) = self.0;
                    AssertionBuilder((v0, value))
                }

                /// Sets the [`belief_type` field](Assertion#structfield.belief_type) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn belief_type_as_default(
                    self,
                ) -> AssertionBuilder<(T0, ::planus::DefaultValue)> {
                    self.belief_type(::planus::DefaultValue)
                }
            }

            impl<T0, T1> AssertionBuilder<(T0, T1)> {
                /// Setter for the [`canonical_identity` field](Assertion#structfield.canonical_identity).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn canonical_identity<T2>(self, value: T2) -> AssertionBuilder<(T0, T1, T2)>
                where
                    T2: ::planus::WriteAs<::planus::Offset<str>>,
                {
                    let (v0, v1) = self.0;
                    AssertionBuilder((v0, v1, value))
                }
            }

            impl<T0, T1, T2> AssertionBuilder<(T0, T1, T2)> {
                /// Setter for the [`value` field](Assertion#structfield.value).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn value<T3>(self, value: T3) -> AssertionBuilder<(T0, T1, T2, T3)>
                where
                    T3: ::planus::WriteAs<::planus::Offset<[u8]>>,
                {
                    let (v0, v1, v2) = self.0;
                    AssertionBuilder((v0, v1, v2, value))
                }
            }

            impl<T0, T1, T2, T3> AssertionBuilder<(T0, T1, T2, T3)> {
                /// Setter for the [`valid_from_ns` field](Assertion#structfield.valid_from_ns).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn valid_from_ns<T4>(self, value: T4) -> AssertionBuilder<(T0, T1, T2, T3, T4)>
                where
                    T4: ::planus::WriteAsDefault<i64, i64>,
                {
                    let (v0, v1, v2, v3) = self.0;
                    AssertionBuilder((v0, v1, v2, v3, value))
                }

                /// Sets the [`valid_from_ns` field](Assertion#structfield.valid_from_ns) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn valid_from_ns_as_default(
                    self,
                ) -> AssertionBuilder<(T0, T1, T2, T3, ::planus::DefaultValue)> {
                    self.valid_from_ns(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2, T3, T4> AssertionBuilder<(T0, T1, T2, T3, T4)> {
                /// Setter for the [`valid_to_ns` field](Assertion#structfield.valid_to_ns).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn valid_to_ns<T5>(
                    self,
                    value: T5,
                ) -> AssertionBuilder<(T0, T1, T2, T3, T4, T5)>
                where
                    T5: ::planus::WriteAsDefault<i64, i64>,
                {
                    let (v0, v1, v2, v3, v4) = self.0;
                    AssertionBuilder((v0, v1, v2, v3, v4, value))
                }

                /// Sets the [`valid_to_ns` field](Assertion#structfield.valid_to_ns) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn valid_to_ns_as_default(
                    self,
                ) -> AssertionBuilder<(T0, T1, T2, T3, T4, ::planus::DefaultValue)>
                {
                    self.valid_to_ns(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2, T3, T4, T5> AssertionBuilder<(T0, T1, T2, T3, T4, T5)> {
                /// Setter for the [`provenance` field](Assertion#structfield.provenance).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn provenance<T6>(
                    self,
                    value: T6,
                ) -> AssertionBuilder<(T0, T1, T2, T3, T4, T5, T6)>
                where
                    T6: ::planus::WriteAs<::planus::Offset<[self::ProvenanceRange]>>,
                {
                    let (v0, v1, v2, v3, v4, v5) = self.0;
                    AssertionBuilder((v0, v1, v2, v3, v4, v5, value))
                }
            }

            impl<T0, T1, T2, T3, T4, T5, T6> AssertionBuilder<(T0, T1, T2, T3, T4, T5, T6)> {
                /// Setter for the [`conflict_domain` field](Assertion#structfield.conflict_domain).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn conflict_domain<T7>(
                    self,
                    value: T7,
                ) -> AssertionBuilder<(T0, T1, T2, T3, T4, T5, T6, T7)>
                where
                    T7: ::planus::WriteAsOptional<::planus::Offset<::core::primitive::str>>,
                {
                    let (v0, v1, v2, v3, v4, v5, v6) = self.0;
                    AssertionBuilder((v0, v1, v2, v3, v4, v5, v6, value))
                }

                /// Sets the [`conflict_domain` field](Assertion#structfield.conflict_domain) to null.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn conflict_domain_as_null(
                    self,
                ) -> AssertionBuilder<(T0, T1, T2, T3, T4, T5, T6, ())> {
                    self.conflict_domain(())
                }
            }

            impl<T0, T1, T2, T3, T4, T5, T6, T7> AssertionBuilder<(T0, T1, T2, T3, T4, T5, T6, T7)> {
                /// Setter for the [`claim` field](Assertion#structfield.claim).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn claim<T8>(
                    self,
                    value: T8,
                ) -> AssertionBuilder<(T0, T1, T2, T3, T4, T5, T6, T7, T8)>
                where
                    T8: ::planus::WriteAsDefault<self::AssertionClaim, self::AssertionClaim>,
                {
                    let (v0, v1, v2, v3, v4, v5, v6, v7) = self.0;
                    AssertionBuilder((v0, v1, v2, v3, v4, v5, v6, v7, value))
                }

                /// Sets the [`claim` field](Assertion#structfield.claim) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn claim_as_default(
                    self,
                ) -> AssertionBuilder<(T0, T1, T2, T3, T4, T5, T6, T7, ::planus::DefaultValue)>
                {
                    self.claim(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2, T3, T4, T5, T6, T7, T8> AssertionBuilder<(T0, T1, T2, T3, T4, T5, T6, T7, T8)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [Assertion].
                #[inline]
                pub fn finish(self, builder: &mut ::planus::Builder) -> ::planus::Offset<Assertion>
                where
                    Self: ::planus::WriteAsOffset<Assertion>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T1: ::planus::WriteAsDefault<self::BeliefType, self::BeliefType>,
                T2: ::planus::WriteAs<::planus::Offset<str>>,
                T3: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T4: ::planus::WriteAsDefault<i64, i64>,
                T5: ::planus::WriteAsDefault<i64, i64>,
                T6: ::planus::WriteAs<::planus::Offset<[self::ProvenanceRange]>>,
                T7: ::planus::WriteAsOptional<::planus::Offset<::core::primitive::str>>,
                T8: ::planus::WriteAsDefault<self::AssertionClaim, self::AssertionClaim>,
            > ::planus::WriteAs<::planus::Offset<Assertion>>
                for AssertionBuilder<(T0, T1, T2, T3, T4, T5, T6, T7, T8)>
            {
                type Prepared = ::planus::Offset<Assertion>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Assertion> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T1: ::planus::WriteAsDefault<self::BeliefType, self::BeliefType>,
                T2: ::planus::WriteAs<::planus::Offset<str>>,
                T3: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T4: ::planus::WriteAsDefault<i64, i64>,
                T5: ::planus::WriteAsDefault<i64, i64>,
                T6: ::planus::WriteAs<::planus::Offset<[self::ProvenanceRange]>>,
                T7: ::planus::WriteAsOptional<::planus::Offset<::core::primitive::str>>,
                T8: ::planus::WriteAsDefault<self::AssertionClaim, self::AssertionClaim>,
            > ::planus::WriteAsOptional<::planus::Offset<Assertion>>
                for AssertionBuilder<(T0, T1, T2, T3, T4, T5, T6, T7, T8)>
            {
                type Prepared = ::planus::Offset<Assertion>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Assertion>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T1: ::planus::WriteAsDefault<self::BeliefType, self::BeliefType>,
                T2: ::planus::WriteAs<::planus::Offset<str>>,
                T3: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T4: ::planus::WriteAsDefault<i64, i64>,
                T5: ::planus::WriteAsDefault<i64, i64>,
                T6: ::planus::WriteAs<::planus::Offset<[self::ProvenanceRange]>>,
                T7: ::planus::WriteAsOptional<::planus::Offset<::core::primitive::str>>,
                T8: ::planus::WriteAsDefault<self::AssertionClaim, self::AssertionClaim>,
            > ::planus::WriteAsOffset<Assertion>
                for AssertionBuilder<(T0, T1, T2, T3, T4, T5, T6, T7, T8)>
            {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Assertion> {
                    let (v0, v1, v2, v3, v4, v5, v6, v7, v8) = &self.0;
                    Assertion::create(builder, v0, v1, v2, v3, v4, v5, v6, v7, v8)
                }
            }

            /// Reference to a deserialized [Assertion].
            #[derive(Copy, Clone)]
            pub struct AssertionRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> AssertionRef<'a> {
                /// Getter for the [`belief_id` field](Assertion#structfield.belief_id).
                #[inline]
                pub fn belief_id(&self) -> ::planus::Result<&'a [u8]> {
                    self.0.access_required(0, "Assertion", "belief_id")
                }

                /// Getter for the [`belief_type` field](Assertion#structfield.belief_type).
                #[inline]
                pub fn belief_type(&self) -> ::planus::Result<self::BeliefType> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(1, "Assertion", "belief_type")?
                            .unwrap_or(self::BeliefType::Fact),
                    )
                }

                /// Getter for the [`canonical_identity` field](Assertion#structfield.canonical_identity).
                #[inline]
                pub fn canonical_identity(&self) -> ::planus::Result<&'a ::core::primitive::str> {
                    self.0.access_required(2, "Assertion", "canonical_identity")
                }

                /// Getter for the [`value` field](Assertion#structfield.value).
                #[inline]
                pub fn value(&self) -> ::planus::Result<&'a [u8]> {
                    self.0.access_required(3, "Assertion", "value")
                }

                /// Getter for the [`valid_from_ns` field](Assertion#structfield.valid_from_ns).
                #[inline]
                pub fn valid_from_ns(&self) -> ::planus::Result<i64> {
                    ::core::result::Result::Ok(
                        self.0.access(4, "Assertion", "valid_from_ns")?.unwrap_or(0),
                    )
                }

                /// Getter for the [`valid_to_ns` field](Assertion#structfield.valid_to_ns).
                #[inline]
                pub fn valid_to_ns(&self) -> ::planus::Result<i64> {
                    ::core::result::Result::Ok(
                        self.0.access(5, "Assertion", "valid_to_ns")?.unwrap_or(0),
                    )
                }

                /// Getter for the [`provenance` field](Assertion#structfield.provenance).
                #[inline]
                pub fn provenance(
                    &self,
                ) -> ::planus::Result<::planus::Vector<'a, self::ProvenanceRangeRef<'a>>>
                {
                    self.0.access_required(6, "Assertion", "provenance")
                }

                /// Getter for the [`conflict_domain` field](Assertion#structfield.conflict_domain).
                #[inline]
                pub fn conflict_domain(
                    &self,
                ) -> ::planus::Result<::core::option::Option<&'a ::core::primitive::str>>
                {
                    self.0.access(7, "Assertion", "conflict_domain")
                }

                /// Getter for the [`claim` field](Assertion#structfield.claim).
                #[inline]
                pub fn claim(&self) -> ::planus::Result<self::AssertionClaim> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(8, "Assertion", "claim")?
                            .unwrap_or(self::AssertionClaim::Affirmative),
                    )
                }
            }

            impl<'a> ::core::fmt::Debug for AssertionRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("AssertionRef");
                    f.field("belief_id", &self.belief_id());
                    f.field("belief_type", &self.belief_type());
                    f.field("canonical_identity", &self.canonical_identity());
                    f.field("value", &self.value());
                    f.field("valid_from_ns", &self.valid_from_ns());
                    f.field("valid_to_ns", &self.valid_to_ns());
                    f.field("provenance", &self.provenance());
                    if let ::core::option::Option::Some(field_conflict_domain) =
                        self.conflict_domain().transpose()
                    {
                        f.field("conflict_domain", &field_conflict_domain);
                    }
                    f.field("claim", &self.claim());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<AssertionRef<'a>> for Assertion {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: AssertionRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        belief_id: value.belief_id()?.to_vec(),
                        belief_type: ::core::convert::TryInto::try_into(value.belief_type()?)?,
                        canonical_identity: ::core::convert::Into::into(
                            value.canonical_identity()?,
                        ),
                        value: value.value()?.to_vec(),
                        valid_from_ns: ::core::convert::TryInto::try_into(value.valid_from_ns()?)?,
                        valid_to_ns: ::core::convert::TryInto::try_into(value.valid_to_ns()?)?,
                        provenance: value.provenance()?.to_vec()?,
                        conflict_domain: value.conflict_domain()?.map(::core::convert::Into::into),
                        claim: ::core::convert::TryInto::try_into(value.claim()?)?,
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for AssertionRef<'a> {
                #[inline]
                fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::ErrorKind> {
                    ::core::result::Result::Ok(Self(::planus::table_reader::Table::from_buffer(
                        buffer, offset,
                    )?))
                }
            }

            impl<'a> ::planus::VectorReadInner<'a> for AssertionRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[AssertionRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<Assertion>> for Assertion {
                type Value = ::planus::Offset<Assertion>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<Assertion>],
                    bytes: *mut ::core::mem::MaybeUninit<u8>,
                    buffer_position: u32,
                ) {
                    let bytes = bytes as *mut [::core::mem::MaybeUninit<u8>; 4];
                    for (i, v) in ::core::iter::Iterator::enumerate(values.iter()) {
                        ::planus::WriteAsPrimitive::write(
                            v,
                            ::planus::Cursor::new(unsafe { &mut *bytes.add(i) }),
                            buffer_position - (Self::STRIDE * i) as u32,
                        );
                    }
                }
            }

            impl<'a> ::planus::ReadAsRoot<'a> for AssertionRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[AssertionRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `Consolidation` in the namespace `hypermind.schema`
            ///
            /// Generated from these locations:
            /// * Table `Consolidation` in the file `schemas/events.fbs:141`
            #[derive(
                Clone,
                Debug,
                PartialEq,
                PartialOrd,
                Eq,
                Ord,
                Hash,
                ::serde::Serialize,
                ::serde::Deserialize,
            )]
            pub struct Consolidation {
                /// The field `assertions` in the table `Consolidation`
                pub assertions: ::planus::alloc::vec::Vec<self::Assertion>,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for Consolidation {
                fn default() -> Self {
                    Self {
                        assertions: ::core::default::Default::default(),
                    }
                }
            }

            impl Consolidation {
                /// Creates a [ConsolidationBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> ConsolidationBuilder<()> {
                    ConsolidationBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_assertions: impl ::planus::WriteAs<
                        ::planus::Offset<[::planus::Offset<self::Assertion>]>,
                    >,
                ) -> ::planus::Offset<Self> {
                    let prepared_assertions = field_assertions.prepare(builder);

                    let mut table_writer: ::planus::table_writer::TableWriter<6> =
                        ::core::default::Default::default();
                    table_writer
                        .write_entry::<::planus::Offset<[::planus::Offset<self::Assertion>]>>(0);

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            object_writer.write::<_, _, 4>(&prepared_assertions);
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<Consolidation>> for Consolidation {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<Consolidation> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<Consolidation>> for Consolidation {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Consolidation>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<Consolidation> for Consolidation {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<Consolidation> {
                    Consolidation::create(builder, &self.assertions)
                }
            }

            /// Builder for serializing an instance of the [Consolidation] type.
            ///
            /// Can be created using the [Consolidation::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct ConsolidationBuilder<State>(State);

            impl ConsolidationBuilder<()> {
                /// Setter for the [`assertions` field](Consolidation#structfield.assertions).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn assertions<T0>(self, value: T0) -> ConsolidationBuilder<(T0,)>
                where
                    T0: ::planus::WriteAs<::planus::Offset<[::planus::Offset<self::Assertion>]>>,
                {
                    ConsolidationBuilder((value,))
                }
            }

            impl<T0> ConsolidationBuilder<(T0,)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [Consolidation].
                #[inline]
                pub fn finish(
                    self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<Consolidation>
                where
                    Self: ::planus::WriteAsOffset<Consolidation>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<T0: ::planus::WriteAs<::planus::Offset<[::planus::Offset<self::Assertion>]>>>
                ::planus::WriteAs<::planus::Offset<Consolidation>> for ConsolidationBuilder<(T0,)>
            {
                type Prepared = ::planus::Offset<Consolidation>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<Consolidation> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<T0: ::planus::WriteAs<::planus::Offset<[::planus::Offset<self::Assertion>]>>>
                ::planus::WriteAsOptional<::planus::Offset<Consolidation>>
                for ConsolidationBuilder<(T0,)>
            {
                type Prepared = ::planus::Offset<Consolidation>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Consolidation>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<T0: ::planus::WriteAs<::planus::Offset<[::planus::Offset<self::Assertion>]>>>
                ::planus::WriteAsOffset<Consolidation> for ConsolidationBuilder<(T0,)>
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<Consolidation> {
                    let (v0,) = &self.0;
                    Consolidation::create(builder, v0)
                }
            }

            /// Reference to a deserialized [Consolidation].
            #[derive(Copy, Clone)]
            pub struct ConsolidationRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> ConsolidationRef<'a> {
                /// Getter for the [`assertions` field](Consolidation#structfield.assertions).
                #[inline]
                pub fn assertions(
                    &self,
                ) -> ::planus::Result<::planus::Vector<'a, ::planus::Result<self::AssertionRef<'a>>>>
                {
                    self.0.access_required(0, "Consolidation", "assertions")
                }
            }

            impl<'a> ::core::fmt::Debug for ConsolidationRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("ConsolidationRef");
                    f.field("assertions", &self.assertions());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<ConsolidationRef<'a>> for Consolidation {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: ConsolidationRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        assertions: value.assertions()?.to_vec_result()?,
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for ConsolidationRef<'a> {
                #[inline]
                fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::ErrorKind> {
                    ::core::result::Result::Ok(Self(::planus::table_reader::Table::from_buffer(
                        buffer, offset,
                    )?))
                }
            }

            impl<'a> ::planus::VectorReadInner<'a> for ConsolidationRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[ConsolidationRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<Consolidation>> for Consolidation {
                type Value = ::planus::Offset<Consolidation>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<Consolidation>],
                    bytes: *mut ::core::mem::MaybeUninit<u8>,
                    buffer_position: u32,
                ) {
                    let bytes = bytes as *mut [::core::mem::MaybeUninit<u8>; 4];
                    for (i, v) in ::core::iter::Iterator::enumerate(values.iter()) {
                        ::planus::WriteAsPrimitive::write(
                            v,
                            ::planus::Cursor::new(unsafe { &mut *bytes.add(i) }),
                            buffer_position - (Self::STRIDE * i) as u32,
                        );
                    }
                }
            }

            impl<'a> ::planus::ReadAsRoot<'a> for ConsolidationRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[ConsolidationRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `ProposedAssertion` in the namespace `hypermind.schema`
            ///
            /// Generated from these locations:
            /// * Table `ProposedAssertion` in the file `schemas/events.fbs:145`
            #[derive(
                Clone,
                Debug,
                PartialEq,
                PartialOrd,
                Eq,
                Ord,
                Hash,
                ::serde::Serialize,
                ::serde::Deserialize,
            )]
            pub struct ProposedAssertion {
                /// The field `belief_id` in the table `ProposedAssertion`
                pub belief_id: ::planus::alloc::vec::Vec<u8>,
                /// The field `belief_type` in the table `ProposedAssertion`
                pub belief_type: self::BeliefType,
                /// The field `canonical_identity` in the table `ProposedAssertion`
                pub canonical_identity: ::planus::alloc::string::String,
                /// The field `value` in the table `ProposedAssertion`
                pub value: ::planus::alloc::vec::Vec<u8>,
                /// The field `valid_from_ns` in the table `ProposedAssertion`
                pub valid_from_ns: i64,
                /// The field `valid_to_ns` in the table `ProposedAssertion`
                pub valid_to_ns: i64,
                /// The field `provenance` in the table `ProposedAssertion`
                pub provenance: ::planus::alloc::vec::Vec<self::ProvenanceRange>,
                /// The field `conflict_domain` in the table `ProposedAssertion`
                pub conflict_domain: ::core::option::Option<::planus::alloc::string::String>,
                /// The field `claim` in the table `ProposedAssertion`
                pub claim: self::AssertionClaim,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for ProposedAssertion {
                fn default() -> Self {
                    Self {
                        belief_id: ::core::default::Default::default(),
                        belief_type: self::BeliefType::Fact,
                        canonical_identity: ::core::default::Default::default(),
                        value: ::core::default::Default::default(),
                        valid_from_ns: 0,
                        valid_to_ns: 0,
                        provenance: ::core::default::Default::default(),
                        conflict_domain: ::core::default::Default::default(),
                        claim: self::AssertionClaim::Affirmative,
                    }
                }
            }

            impl ProposedAssertion {
                /// Creates a [ProposedAssertionBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> ProposedAssertionBuilder<()> {
                    ProposedAssertionBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_belief_id: impl ::planus::WriteAs<::planus::Offset<[u8]>>,
                    field_belief_type: impl ::planus::WriteAsDefault<self::BeliefType, self::BeliefType>,
                    field_canonical_identity: impl ::planus::WriteAs<::planus::Offset<str>>,
                    field_value: impl ::planus::WriteAs<::planus::Offset<[u8]>>,
                    field_valid_from_ns: impl ::planus::WriteAsDefault<i64, i64>,
                    field_valid_to_ns: impl ::planus::WriteAsDefault<i64, i64>,
                    field_provenance: impl ::planus::WriteAs<::planus::Offset<[self::ProvenanceRange]>>,
                    field_conflict_domain: impl ::planus::WriteAsOptional<
                        ::planus::Offset<::core::primitive::str>,
                    >,
                    field_claim: impl ::planus::WriteAsDefault<
                        self::AssertionClaim,
                        self::AssertionClaim,
                    >,
                ) -> ::planus::Offset<Self> {
                    let prepared_belief_id = field_belief_id.prepare(builder);
                    let prepared_belief_type =
                        field_belief_type.prepare(builder, &self::BeliefType::Fact);
                    let prepared_canonical_identity = field_canonical_identity.prepare(builder);
                    let prepared_value = field_value.prepare(builder);
                    let prepared_valid_from_ns = field_valid_from_ns.prepare(builder, &0);
                    let prepared_valid_to_ns = field_valid_to_ns.prepare(builder, &0);
                    let prepared_provenance = field_provenance.prepare(builder);
                    let prepared_conflict_domain = field_conflict_domain.prepare(builder);
                    let prepared_claim =
                        field_claim.prepare(builder, &self::AssertionClaim::Affirmative);

                    let mut table_writer: ::planus::table_writer::TableWriter<22> =
                        ::core::default::Default::default();
                    if prepared_valid_from_ns.is_some() {
                        table_writer.write_entry::<i64>(4);
                    }
                    if prepared_valid_to_ns.is_some() {
                        table_writer.write_entry::<i64>(5);
                    }
                    table_writer.write_entry::<::planus::Offset<[u8]>>(0);
                    table_writer.write_entry::<::planus::Offset<str>>(2);
                    table_writer.write_entry::<::planus::Offset<[u8]>>(3);
                    table_writer.write_entry::<::planus::Offset<[self::ProvenanceRange]>>(6);
                    if prepared_conflict_domain.is_some() {
                        table_writer.write_entry::<::planus::Offset<str>>(7);
                    }
                    if prepared_belief_type.is_some() {
                        table_writer.write_entry::<self::BeliefType>(1);
                    }
                    if prepared_claim.is_some() {
                        table_writer.write_entry::<self::AssertionClaim>(8);
                    }

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            if let ::core::option::Option::Some(prepared_valid_from_ns) =
                                prepared_valid_from_ns
                            {
                                object_writer.write::<_, _, 8>(&prepared_valid_from_ns);
                            }
                            if let ::core::option::Option::Some(prepared_valid_to_ns) =
                                prepared_valid_to_ns
                            {
                                object_writer.write::<_, _, 8>(&prepared_valid_to_ns);
                            }
                            object_writer.write::<_, _, 4>(&prepared_belief_id);
                            object_writer.write::<_, _, 4>(&prepared_canonical_identity);
                            object_writer.write::<_, _, 4>(&prepared_value);
                            object_writer.write::<_, _, 4>(&prepared_provenance);
                            if let ::core::option::Option::Some(prepared_conflict_domain) =
                                prepared_conflict_domain
                            {
                                object_writer.write::<_, _, 4>(&prepared_conflict_domain);
                            }
                            if let ::core::option::Option::Some(prepared_belief_type) =
                                prepared_belief_type
                            {
                                object_writer.write::<_, _, 1>(&prepared_belief_type);
                            }
                            if let ::core::option::Option::Some(prepared_claim) = prepared_claim {
                                object_writer.write::<_, _, 1>(&prepared_claim);
                            }
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<ProposedAssertion>> for ProposedAssertion {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<ProposedAssertion> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<ProposedAssertion>> for ProposedAssertion {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<ProposedAssertion>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<ProposedAssertion> for ProposedAssertion {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<ProposedAssertion> {
                    ProposedAssertion::create(
                        builder,
                        &self.belief_id,
                        self.belief_type,
                        &self.canonical_identity,
                        &self.value,
                        self.valid_from_ns,
                        self.valid_to_ns,
                        &self.provenance,
                        &self.conflict_domain,
                        self.claim,
                    )
                }
            }

            /// Builder for serializing an instance of the [ProposedAssertion] type.
            ///
            /// Can be created using the [ProposedAssertion::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct ProposedAssertionBuilder<State>(State);

            impl ProposedAssertionBuilder<()> {
                /// Setter for the [`belief_id` field](ProposedAssertion#structfield.belief_id).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn belief_id<T0>(self, value: T0) -> ProposedAssertionBuilder<(T0,)>
                where
                    T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                {
                    ProposedAssertionBuilder((value,))
                }
            }

            impl<T0> ProposedAssertionBuilder<(T0,)> {
                /// Setter for the [`belief_type` field](ProposedAssertion#structfield.belief_type).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn belief_type<T1>(self, value: T1) -> ProposedAssertionBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAsDefault<self::BeliefType, self::BeliefType>,
                {
                    let (v0,) = self.0;
                    ProposedAssertionBuilder((v0, value))
                }

                /// Sets the [`belief_type` field](ProposedAssertion#structfield.belief_type) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn belief_type_as_default(
                    self,
                ) -> ProposedAssertionBuilder<(T0, ::planus::DefaultValue)> {
                    self.belief_type(::planus::DefaultValue)
                }
            }

            impl<T0, T1> ProposedAssertionBuilder<(T0, T1)> {
                /// Setter for the [`canonical_identity` field](ProposedAssertion#structfield.canonical_identity).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn canonical_identity<T2>(
                    self,
                    value: T2,
                ) -> ProposedAssertionBuilder<(T0, T1, T2)>
                where
                    T2: ::planus::WriteAs<::planus::Offset<str>>,
                {
                    let (v0, v1) = self.0;
                    ProposedAssertionBuilder((v0, v1, value))
                }
            }

            impl<T0, T1, T2> ProposedAssertionBuilder<(T0, T1, T2)> {
                /// Setter for the [`value` field](ProposedAssertion#structfield.value).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn value<T3>(self, value: T3) -> ProposedAssertionBuilder<(T0, T1, T2, T3)>
                where
                    T3: ::planus::WriteAs<::planus::Offset<[u8]>>,
                {
                    let (v0, v1, v2) = self.0;
                    ProposedAssertionBuilder((v0, v1, v2, value))
                }
            }

            impl<T0, T1, T2, T3> ProposedAssertionBuilder<(T0, T1, T2, T3)> {
                /// Setter for the [`valid_from_ns` field](ProposedAssertion#structfield.valid_from_ns).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn valid_from_ns<T4>(
                    self,
                    value: T4,
                ) -> ProposedAssertionBuilder<(T0, T1, T2, T3, T4)>
                where
                    T4: ::planus::WriteAsDefault<i64, i64>,
                {
                    let (v0, v1, v2, v3) = self.0;
                    ProposedAssertionBuilder((v0, v1, v2, v3, value))
                }

                /// Sets the [`valid_from_ns` field](ProposedAssertion#structfield.valid_from_ns) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn valid_from_ns_as_default(
                    self,
                ) -> ProposedAssertionBuilder<(T0, T1, T2, T3, ::planus::DefaultValue)>
                {
                    self.valid_from_ns(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2, T3, T4> ProposedAssertionBuilder<(T0, T1, T2, T3, T4)> {
                /// Setter for the [`valid_to_ns` field](ProposedAssertion#structfield.valid_to_ns).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn valid_to_ns<T5>(
                    self,
                    value: T5,
                ) -> ProposedAssertionBuilder<(T0, T1, T2, T3, T4, T5)>
                where
                    T5: ::planus::WriteAsDefault<i64, i64>,
                {
                    let (v0, v1, v2, v3, v4) = self.0;
                    ProposedAssertionBuilder((v0, v1, v2, v3, v4, value))
                }

                /// Sets the [`valid_to_ns` field](ProposedAssertion#structfield.valid_to_ns) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn valid_to_ns_as_default(
                    self,
                ) -> ProposedAssertionBuilder<(T0, T1, T2, T3, T4, ::planus::DefaultValue)>
                {
                    self.valid_to_ns(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2, T3, T4, T5> ProposedAssertionBuilder<(T0, T1, T2, T3, T4, T5)> {
                /// Setter for the [`provenance` field](ProposedAssertion#structfield.provenance).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn provenance<T6>(
                    self,
                    value: T6,
                ) -> ProposedAssertionBuilder<(T0, T1, T2, T3, T4, T5, T6)>
                where
                    T6: ::planus::WriteAs<::planus::Offset<[self::ProvenanceRange]>>,
                {
                    let (v0, v1, v2, v3, v4, v5) = self.0;
                    ProposedAssertionBuilder((v0, v1, v2, v3, v4, v5, value))
                }
            }

            impl<T0, T1, T2, T3, T4, T5, T6> ProposedAssertionBuilder<(T0, T1, T2, T3, T4, T5, T6)> {
                /// Setter for the [`conflict_domain` field](ProposedAssertion#structfield.conflict_domain).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn conflict_domain<T7>(
                    self,
                    value: T7,
                ) -> ProposedAssertionBuilder<(T0, T1, T2, T3, T4, T5, T6, T7)>
                where
                    T7: ::planus::WriteAsOptional<::planus::Offset<::core::primitive::str>>,
                {
                    let (v0, v1, v2, v3, v4, v5, v6) = self.0;
                    ProposedAssertionBuilder((v0, v1, v2, v3, v4, v5, v6, value))
                }

                /// Sets the [`conflict_domain` field](ProposedAssertion#structfield.conflict_domain) to null.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn conflict_domain_as_null(
                    self,
                ) -> ProposedAssertionBuilder<(T0, T1, T2, T3, T4, T5, T6, ())> {
                    self.conflict_domain(())
                }
            }

            impl<T0, T1, T2, T3, T4, T5, T6, T7> ProposedAssertionBuilder<(T0, T1, T2, T3, T4, T5, T6, T7)> {
                /// Setter for the [`claim` field](ProposedAssertion#structfield.claim).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn claim<T8>(
                    self,
                    value: T8,
                ) -> ProposedAssertionBuilder<(T0, T1, T2, T3, T4, T5, T6, T7, T8)>
                where
                    T8: ::planus::WriteAsDefault<self::AssertionClaim, self::AssertionClaim>,
                {
                    let (v0, v1, v2, v3, v4, v5, v6, v7) = self.0;
                    ProposedAssertionBuilder((v0, v1, v2, v3, v4, v5, v6, v7, value))
                }

                /// Sets the [`claim` field](ProposedAssertion#structfield.claim) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn claim_as_default(
                    self,
                ) -> ProposedAssertionBuilder<(
                    T0,
                    T1,
                    T2,
                    T3,
                    T4,
                    T5,
                    T6,
                    T7,
                    ::planus::DefaultValue,
                )> {
                    self.claim(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2, T3, T4, T5, T6, T7, T8>
                ProposedAssertionBuilder<(T0, T1, T2, T3, T4, T5, T6, T7, T8)>
            {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [ProposedAssertion].
                #[inline]
                pub fn finish(
                    self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<ProposedAssertion>
                where
                    Self: ::planus::WriteAsOffset<ProposedAssertion>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T1: ::planus::WriteAsDefault<self::BeliefType, self::BeliefType>,
                T2: ::planus::WriteAs<::planus::Offset<str>>,
                T3: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T4: ::planus::WriteAsDefault<i64, i64>,
                T5: ::planus::WriteAsDefault<i64, i64>,
                T6: ::planus::WriteAs<::planus::Offset<[self::ProvenanceRange]>>,
                T7: ::planus::WriteAsOptional<::planus::Offset<::core::primitive::str>>,
                T8: ::planus::WriteAsDefault<self::AssertionClaim, self::AssertionClaim>,
            > ::planus::WriteAs<::planus::Offset<ProposedAssertion>>
                for ProposedAssertionBuilder<(T0, T1, T2, T3, T4, T5, T6, T7, T8)>
            {
                type Prepared = ::planus::Offset<ProposedAssertion>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<ProposedAssertion> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T1: ::planus::WriteAsDefault<self::BeliefType, self::BeliefType>,
                T2: ::planus::WriteAs<::planus::Offset<str>>,
                T3: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T4: ::planus::WriteAsDefault<i64, i64>,
                T5: ::planus::WriteAsDefault<i64, i64>,
                T6: ::planus::WriteAs<::planus::Offset<[self::ProvenanceRange]>>,
                T7: ::planus::WriteAsOptional<::planus::Offset<::core::primitive::str>>,
                T8: ::planus::WriteAsDefault<self::AssertionClaim, self::AssertionClaim>,
            > ::planus::WriteAsOptional<::planus::Offset<ProposedAssertion>>
                for ProposedAssertionBuilder<(T0, T1, T2, T3, T4, T5, T6, T7, T8)>
            {
                type Prepared = ::planus::Offset<ProposedAssertion>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<ProposedAssertion>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T1: ::planus::WriteAsDefault<self::BeliefType, self::BeliefType>,
                T2: ::planus::WriteAs<::planus::Offset<str>>,
                T3: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T4: ::planus::WriteAsDefault<i64, i64>,
                T5: ::planus::WriteAsDefault<i64, i64>,
                T6: ::planus::WriteAs<::planus::Offset<[self::ProvenanceRange]>>,
                T7: ::planus::WriteAsOptional<::planus::Offset<::core::primitive::str>>,
                T8: ::planus::WriteAsDefault<self::AssertionClaim, self::AssertionClaim>,
            > ::planus::WriteAsOffset<ProposedAssertion>
                for ProposedAssertionBuilder<(T0, T1, T2, T3, T4, T5, T6, T7, T8)>
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<ProposedAssertion> {
                    let (v0, v1, v2, v3, v4, v5, v6, v7, v8) = &self.0;
                    ProposedAssertion::create(builder, v0, v1, v2, v3, v4, v5, v6, v7, v8)
                }
            }

            /// Reference to a deserialized [ProposedAssertion].
            #[derive(Copy, Clone)]
            pub struct ProposedAssertionRef<'a>(
                #[allow(dead_code)] ::planus::table_reader::Table<'a>,
            );

            impl<'a> ProposedAssertionRef<'a> {
                /// Getter for the [`belief_id` field](ProposedAssertion#structfield.belief_id).
                #[inline]
                pub fn belief_id(&self) -> ::planus::Result<&'a [u8]> {
                    self.0.access_required(0, "ProposedAssertion", "belief_id")
                }

                /// Getter for the [`belief_type` field](ProposedAssertion#structfield.belief_type).
                #[inline]
                pub fn belief_type(&self) -> ::planus::Result<self::BeliefType> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(1, "ProposedAssertion", "belief_type")?
                            .unwrap_or(self::BeliefType::Fact),
                    )
                }

                /// Getter for the [`canonical_identity` field](ProposedAssertion#structfield.canonical_identity).
                #[inline]
                pub fn canonical_identity(&self) -> ::planus::Result<&'a ::core::primitive::str> {
                    self.0
                        .access_required(2, "ProposedAssertion", "canonical_identity")
                }

                /// Getter for the [`value` field](ProposedAssertion#structfield.value).
                #[inline]
                pub fn value(&self) -> ::planus::Result<&'a [u8]> {
                    self.0.access_required(3, "ProposedAssertion", "value")
                }

                /// Getter for the [`valid_from_ns` field](ProposedAssertion#structfield.valid_from_ns).
                #[inline]
                pub fn valid_from_ns(&self) -> ::planus::Result<i64> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(4, "ProposedAssertion", "valid_from_ns")?
                            .unwrap_or(0),
                    )
                }

                /// Getter for the [`valid_to_ns` field](ProposedAssertion#structfield.valid_to_ns).
                #[inline]
                pub fn valid_to_ns(&self) -> ::planus::Result<i64> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(5, "ProposedAssertion", "valid_to_ns")?
                            .unwrap_or(0),
                    )
                }

                /// Getter for the [`provenance` field](ProposedAssertion#structfield.provenance).
                #[inline]
                pub fn provenance(
                    &self,
                ) -> ::planus::Result<::planus::Vector<'a, self::ProvenanceRangeRef<'a>>>
                {
                    self.0.access_required(6, "ProposedAssertion", "provenance")
                }

                /// Getter for the [`conflict_domain` field](ProposedAssertion#structfield.conflict_domain).
                #[inline]
                pub fn conflict_domain(
                    &self,
                ) -> ::planus::Result<::core::option::Option<&'a ::core::primitive::str>>
                {
                    self.0.access(7, "ProposedAssertion", "conflict_domain")
                }

                /// Getter for the [`claim` field](ProposedAssertion#structfield.claim).
                #[inline]
                pub fn claim(&self) -> ::planus::Result<self::AssertionClaim> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(8, "ProposedAssertion", "claim")?
                            .unwrap_or(self::AssertionClaim::Affirmative),
                    )
                }
            }

            impl<'a> ::core::fmt::Debug for ProposedAssertionRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("ProposedAssertionRef");
                    f.field("belief_id", &self.belief_id());
                    f.field("belief_type", &self.belief_type());
                    f.field("canonical_identity", &self.canonical_identity());
                    f.field("value", &self.value());
                    f.field("valid_from_ns", &self.valid_from_ns());
                    f.field("valid_to_ns", &self.valid_to_ns());
                    f.field("provenance", &self.provenance());
                    if let ::core::option::Option::Some(field_conflict_domain) =
                        self.conflict_domain().transpose()
                    {
                        f.field("conflict_domain", &field_conflict_domain);
                    }
                    f.field("claim", &self.claim());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<ProposedAssertionRef<'a>> for ProposedAssertion {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: ProposedAssertionRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        belief_id: value.belief_id()?.to_vec(),
                        belief_type: ::core::convert::TryInto::try_into(value.belief_type()?)?,
                        canonical_identity: ::core::convert::Into::into(
                            value.canonical_identity()?,
                        ),
                        value: value.value()?.to_vec(),
                        valid_from_ns: ::core::convert::TryInto::try_into(value.valid_from_ns()?)?,
                        valid_to_ns: ::core::convert::TryInto::try_into(value.valid_to_ns()?)?,
                        provenance: value.provenance()?.to_vec()?,
                        conflict_domain: value.conflict_domain()?.map(::core::convert::Into::into),
                        claim: ::core::convert::TryInto::try_into(value.claim()?)?,
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for ProposedAssertionRef<'a> {
                #[inline]
                fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::ErrorKind> {
                    ::core::result::Result::Ok(Self(::planus::table_reader::Table::from_buffer(
                        buffer, offset,
                    )?))
                }
            }

            impl<'a> ::planus::VectorReadInner<'a> for ProposedAssertionRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[ProposedAssertionRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<ProposedAssertion>> for ProposedAssertion {
                type Value = ::planus::Offset<ProposedAssertion>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<ProposedAssertion>],
                    bytes: *mut ::core::mem::MaybeUninit<u8>,
                    buffer_position: u32,
                ) {
                    let bytes = bytes as *mut [::core::mem::MaybeUninit<u8>; 4];
                    for (i, v) in ::core::iter::Iterator::enumerate(values.iter()) {
                        ::planus::WriteAsPrimitive::write(
                            v,
                            ::planus::Cursor::new(unsafe { &mut *bytes.add(i) }),
                            buffer_position - (Self::STRIDE * i) as u32,
                        );
                    }
                }
            }

            impl<'a> ::planus::ReadAsRoot<'a> for ProposedAssertionRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[ProposedAssertionRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `Embedding` in the namespace `hypermind.schema`
            ///
            /// Generated from these locations:
            /// * Table `Embedding` in the file `schemas/events.fbs:157`
            #[derive(
                Clone,
                Debug,
                PartialEq,
                PartialOrd,
                Eq,
                Ord,
                Hash,
                ::serde::Serialize,
                ::serde::Deserialize,
            )]
            pub struct Embedding {
                /// The field `target_lsn` in the table `Embedding`
                pub target_lsn: u64,
                /// The field `dimension` in the table `Embedding`
                pub dimension: u32,
                /// The field `quantized` in the table `Embedding`
                pub quantized: ::planus::alloc::vec::Vec<i8>,
                /// The field `binary_prefilter` in the table `Embedding`
                pub binary_prefilter: ::planus::alloc::vec::Vec<u8>,
                /// The field `space_id` in the table `Embedding`
                pub space_id: ::planus::alloc::string::String,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for Embedding {
                fn default() -> Self {
                    Self {
                        target_lsn: 0,
                        dimension: 0,
                        quantized: ::core::default::Default::default(),
                        binary_prefilter: ::core::default::Default::default(),
                        space_id: ::core::default::Default::default(),
                    }
                }
            }

            impl Embedding {
                /// Creates a [EmbeddingBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> EmbeddingBuilder<()> {
                    EmbeddingBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_target_lsn: impl ::planus::WriteAsDefault<u64, u64>,
                    field_dimension: impl ::planus::WriteAsDefault<u32, u32>,
                    field_quantized: impl ::planus::WriteAs<::planus::Offset<[i8]>>,
                    field_binary_prefilter: impl ::planus::WriteAs<::planus::Offset<[u8]>>,
                    field_space_id: impl ::planus::WriteAs<::planus::Offset<str>>,
                ) -> ::planus::Offset<Self> {
                    let prepared_target_lsn = field_target_lsn.prepare(builder, &0);
                    let prepared_dimension = field_dimension.prepare(builder, &0);
                    let prepared_quantized = field_quantized.prepare(builder);
                    let prepared_binary_prefilter = field_binary_prefilter.prepare(builder);
                    let prepared_space_id = field_space_id.prepare(builder);

                    let mut table_writer: ::planus::table_writer::TableWriter<14> =
                        ::core::default::Default::default();
                    if prepared_target_lsn.is_some() {
                        table_writer.write_entry::<u64>(0);
                    }
                    if prepared_dimension.is_some() {
                        table_writer.write_entry::<u32>(1);
                    }
                    table_writer.write_entry::<::planus::Offset<[i8]>>(2);
                    table_writer.write_entry::<::planus::Offset<[u8]>>(3);
                    table_writer.write_entry::<::planus::Offset<str>>(4);

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            if let ::core::option::Option::Some(prepared_target_lsn) =
                                prepared_target_lsn
                            {
                                object_writer.write::<_, _, 8>(&prepared_target_lsn);
                            }
                            if let ::core::option::Option::Some(prepared_dimension) =
                                prepared_dimension
                            {
                                object_writer.write::<_, _, 4>(&prepared_dimension);
                            }
                            object_writer.write::<_, _, 4>(&prepared_quantized);
                            object_writer.write::<_, _, 4>(&prepared_binary_prefilter);
                            object_writer.write::<_, _, 4>(&prepared_space_id);
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<Embedding>> for Embedding {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Embedding> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<Embedding>> for Embedding {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Embedding>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<Embedding> for Embedding {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Embedding> {
                    Embedding::create(
                        builder,
                        self.target_lsn,
                        self.dimension,
                        &self.quantized,
                        &self.binary_prefilter,
                        &self.space_id,
                    )
                }
            }

            /// Builder for serializing an instance of the [Embedding] type.
            ///
            /// Can be created using the [Embedding::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct EmbeddingBuilder<State>(State);

            impl EmbeddingBuilder<()> {
                /// Setter for the [`target_lsn` field](Embedding#structfield.target_lsn).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn target_lsn<T0>(self, value: T0) -> EmbeddingBuilder<(T0,)>
                where
                    T0: ::planus::WriteAsDefault<u64, u64>,
                {
                    EmbeddingBuilder((value,))
                }

                /// Sets the [`target_lsn` field](Embedding#structfield.target_lsn) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn target_lsn_as_default(self) -> EmbeddingBuilder<(::planus::DefaultValue,)> {
                    self.target_lsn(::planus::DefaultValue)
                }
            }

            impl<T0> EmbeddingBuilder<(T0,)> {
                /// Setter for the [`dimension` field](Embedding#structfield.dimension).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn dimension<T1>(self, value: T1) -> EmbeddingBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAsDefault<u32, u32>,
                {
                    let (v0,) = self.0;
                    EmbeddingBuilder((v0, value))
                }

                /// Sets the [`dimension` field](Embedding#structfield.dimension) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn dimension_as_default(
                    self,
                ) -> EmbeddingBuilder<(T0, ::planus::DefaultValue)> {
                    self.dimension(::planus::DefaultValue)
                }
            }

            impl<T0, T1> EmbeddingBuilder<(T0, T1)> {
                /// Setter for the [`quantized` field](Embedding#structfield.quantized).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn quantized<T2>(self, value: T2) -> EmbeddingBuilder<(T0, T1, T2)>
                where
                    T2: ::planus::WriteAs<::planus::Offset<[i8]>>,
                {
                    let (v0, v1) = self.0;
                    EmbeddingBuilder((v0, v1, value))
                }
            }

            impl<T0, T1, T2> EmbeddingBuilder<(T0, T1, T2)> {
                /// Setter for the [`binary_prefilter` field](Embedding#structfield.binary_prefilter).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn binary_prefilter<T3>(self, value: T3) -> EmbeddingBuilder<(T0, T1, T2, T3)>
                where
                    T3: ::planus::WriteAs<::planus::Offset<[u8]>>,
                {
                    let (v0, v1, v2) = self.0;
                    EmbeddingBuilder((v0, v1, v2, value))
                }
            }

            impl<T0, T1, T2, T3> EmbeddingBuilder<(T0, T1, T2, T3)> {
                /// Setter for the [`space_id` field](Embedding#structfield.space_id).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn space_id<T4>(self, value: T4) -> EmbeddingBuilder<(T0, T1, T2, T3, T4)>
                where
                    T4: ::planus::WriteAs<::planus::Offset<str>>,
                {
                    let (v0, v1, v2, v3) = self.0;
                    EmbeddingBuilder((v0, v1, v2, v3, value))
                }
            }

            impl<T0, T1, T2, T3, T4> EmbeddingBuilder<(T0, T1, T2, T3, T4)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [Embedding].
                #[inline]
                pub fn finish(self, builder: &mut ::planus::Builder) -> ::planus::Offset<Embedding>
                where
                    Self: ::planus::WriteAsOffset<Embedding>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u64, u64>,
                T1: ::planus::WriteAsDefault<u32, u32>,
                T2: ::planus::WriteAs<::planus::Offset<[i8]>>,
                T3: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T4: ::planus::WriteAs<::planus::Offset<str>>,
            > ::planus::WriteAs<::planus::Offset<Embedding>>
                for EmbeddingBuilder<(T0, T1, T2, T3, T4)>
            {
                type Prepared = ::planus::Offset<Embedding>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Embedding> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u64, u64>,
                T1: ::planus::WriteAsDefault<u32, u32>,
                T2: ::planus::WriteAs<::planus::Offset<[i8]>>,
                T3: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T4: ::planus::WriteAs<::planus::Offset<str>>,
            > ::planus::WriteAsOptional<::planus::Offset<Embedding>>
                for EmbeddingBuilder<(T0, T1, T2, T3, T4)>
            {
                type Prepared = ::planus::Offset<Embedding>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Embedding>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u64, u64>,
                T1: ::planus::WriteAsDefault<u32, u32>,
                T2: ::planus::WriteAs<::planus::Offset<[i8]>>,
                T3: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T4: ::planus::WriteAs<::planus::Offset<str>>,
            > ::planus::WriteAsOffset<Embedding> for EmbeddingBuilder<(T0, T1, T2, T3, T4)>
            {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Embedding> {
                    let (v0, v1, v2, v3, v4) = &self.0;
                    Embedding::create(builder, v0, v1, v2, v3, v4)
                }
            }

            /// Reference to a deserialized [Embedding].
            #[derive(Copy, Clone)]
            pub struct EmbeddingRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> EmbeddingRef<'a> {
                /// Getter for the [`target_lsn` field](Embedding#structfield.target_lsn).
                #[inline]
                pub fn target_lsn(&self) -> ::planus::Result<u64> {
                    ::core::result::Result::Ok(
                        self.0.access(0, "Embedding", "target_lsn")?.unwrap_or(0),
                    )
                }

                /// Getter for the [`dimension` field](Embedding#structfield.dimension).
                #[inline]
                pub fn dimension(&self) -> ::planus::Result<u32> {
                    ::core::result::Result::Ok(
                        self.0.access(1, "Embedding", "dimension")?.unwrap_or(0),
                    )
                }

                /// Getter for the [`quantized` field](Embedding#structfield.quantized).
                #[inline]
                pub fn quantized(&self) -> ::planus::Result<&'a [i8]> {
                    self.0.access_required(2, "Embedding", "quantized")
                }

                /// Getter for the [`binary_prefilter` field](Embedding#structfield.binary_prefilter).
                #[inline]
                pub fn binary_prefilter(&self) -> ::planus::Result<&'a [u8]> {
                    self.0.access_required(3, "Embedding", "binary_prefilter")
                }

                /// Getter for the [`space_id` field](Embedding#structfield.space_id).
                #[inline]
                pub fn space_id(&self) -> ::planus::Result<&'a ::core::primitive::str> {
                    self.0.access_required(4, "Embedding", "space_id")
                }
            }

            impl<'a> ::core::fmt::Debug for EmbeddingRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("EmbeddingRef");
                    f.field("target_lsn", &self.target_lsn());
                    f.field("dimension", &self.dimension());
                    f.field("quantized", &self.quantized());
                    f.field("binary_prefilter", &self.binary_prefilter());
                    f.field("space_id", &self.space_id());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<EmbeddingRef<'a>> for Embedding {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: EmbeddingRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        target_lsn: ::core::convert::TryInto::try_into(value.target_lsn()?)?,
                        dimension: ::core::convert::TryInto::try_into(value.dimension()?)?,
                        quantized: value.quantized()?.to_vec(),
                        binary_prefilter: value.binary_prefilter()?.to_vec(),
                        space_id: ::core::convert::Into::into(value.space_id()?),
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for EmbeddingRef<'a> {
                #[inline]
                fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::ErrorKind> {
                    ::core::result::Result::Ok(Self(::planus::table_reader::Table::from_buffer(
                        buffer, offset,
                    )?))
                }
            }

            impl<'a> ::planus::VectorReadInner<'a> for EmbeddingRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[EmbeddingRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<Embedding>> for Embedding {
                type Value = ::planus::Offset<Embedding>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<Embedding>],
                    bytes: *mut ::core::mem::MaybeUninit<u8>,
                    buffer_position: u32,
                ) {
                    let bytes = bytes as *mut [::core::mem::MaybeUninit<u8>; 4];
                    for (i, v) in ::core::iter::Iterator::enumerate(values.iter()) {
                        ::planus::WriteAsPrimitive::write(
                            v,
                            ::planus::Cursor::new(unsafe { &mut *bytes.add(i) }),
                            buffer_position - (Self::STRIDE * i) as u32,
                        );
                    }
                }
            }

            impl<'a> ::planus::ReadAsRoot<'a> for EmbeddingRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[EmbeddingRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `Retract` in the namespace `hypermind.schema`
            ///
            /// Generated from these locations:
            /// * Table `Retract` in the file `schemas/events.fbs:165`
            #[derive(
                Clone,
                Debug,
                PartialEq,
                PartialOrd,
                Eq,
                Ord,
                Hash,
                ::serde::Serialize,
                ::serde::Deserialize,
            )]
            pub struct Retract {
                /// The field `belief_id` in the table `Retract`
                pub belief_id: ::planus::alloc::vec::Vec<u8>,
                /// The field `provenance` in the table `Retract`
                pub provenance: ::planus::alloc::vec::Vec<self::ProvenanceRange>,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for Retract {
                fn default() -> Self {
                    Self {
                        belief_id: ::core::default::Default::default(),
                        provenance: ::core::default::Default::default(),
                    }
                }
            }

            impl Retract {
                /// Creates a [RetractBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> RetractBuilder<()> {
                    RetractBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_belief_id: impl ::planus::WriteAs<::planus::Offset<[u8]>>,
                    field_provenance: impl ::planus::WriteAs<::planus::Offset<[self::ProvenanceRange]>>,
                ) -> ::planus::Offset<Self> {
                    let prepared_belief_id = field_belief_id.prepare(builder);
                    let prepared_provenance = field_provenance.prepare(builder);

                    let mut table_writer: ::planus::table_writer::TableWriter<8> =
                        ::core::default::Default::default();
                    table_writer.write_entry::<::planus::Offset<[u8]>>(0);
                    table_writer.write_entry::<::planus::Offset<[self::ProvenanceRange]>>(1);

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            object_writer.write::<_, _, 4>(&prepared_belief_id);
                            object_writer.write::<_, _, 4>(&prepared_provenance);
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<Retract>> for Retract {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Retract> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<Retract>> for Retract {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Retract>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<Retract> for Retract {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Retract> {
                    Retract::create(builder, &self.belief_id, &self.provenance)
                }
            }

            /// Builder for serializing an instance of the [Retract] type.
            ///
            /// Can be created using the [Retract::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct RetractBuilder<State>(State);

            impl RetractBuilder<()> {
                /// Setter for the [`belief_id` field](Retract#structfield.belief_id).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn belief_id<T0>(self, value: T0) -> RetractBuilder<(T0,)>
                where
                    T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                {
                    RetractBuilder((value,))
                }
            }

            impl<T0> RetractBuilder<(T0,)> {
                /// Setter for the [`provenance` field](Retract#structfield.provenance).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn provenance<T1>(self, value: T1) -> RetractBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAs<::planus::Offset<[self::ProvenanceRange]>>,
                {
                    let (v0,) = self.0;
                    RetractBuilder((v0, value))
                }
            }

            impl<T0, T1> RetractBuilder<(T0, T1)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [Retract].
                #[inline]
                pub fn finish(self, builder: &mut ::planus::Builder) -> ::planus::Offset<Retract>
                where
                    Self: ::planus::WriteAsOffset<Retract>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T1: ::planus::WriteAs<::planus::Offset<[self::ProvenanceRange]>>,
            > ::planus::WriteAs<::planus::Offset<Retract>> for RetractBuilder<(T0, T1)>
            {
                type Prepared = ::planus::Offset<Retract>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Retract> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T1: ::planus::WriteAs<::planus::Offset<[self::ProvenanceRange]>>,
            > ::planus::WriteAsOptional<::planus::Offset<Retract>> for RetractBuilder<(T0, T1)>
            {
                type Prepared = ::planus::Offset<Retract>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Retract>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T1: ::planus::WriteAs<::planus::Offset<[self::ProvenanceRange]>>,
            > ::planus::WriteAsOffset<Retract> for RetractBuilder<(T0, T1)>
            {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Retract> {
                    let (v0, v1) = &self.0;
                    Retract::create(builder, v0, v1)
                }
            }

            /// Reference to a deserialized [Retract].
            #[derive(Copy, Clone)]
            pub struct RetractRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> RetractRef<'a> {
                /// Getter for the [`belief_id` field](Retract#structfield.belief_id).
                #[inline]
                pub fn belief_id(&self) -> ::planus::Result<&'a [u8]> {
                    self.0.access_required(0, "Retract", "belief_id")
                }

                /// Getter for the [`provenance` field](Retract#structfield.provenance).
                #[inline]
                pub fn provenance(
                    &self,
                ) -> ::planus::Result<::planus::Vector<'a, self::ProvenanceRangeRef<'a>>>
                {
                    self.0.access_required(1, "Retract", "provenance")
                }
            }

            impl<'a> ::core::fmt::Debug for RetractRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("RetractRef");
                    f.field("belief_id", &self.belief_id());
                    f.field("provenance", &self.provenance());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<RetractRef<'a>> for Retract {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: RetractRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        belief_id: value.belief_id()?.to_vec(),
                        provenance: value.provenance()?.to_vec()?,
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for RetractRef<'a> {
                #[inline]
                fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::ErrorKind> {
                    ::core::result::Result::Ok(Self(::planus::table_reader::Table::from_buffer(
                        buffer, offset,
                    )?))
                }
            }

            impl<'a> ::planus::VectorReadInner<'a> for RetractRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[RetractRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<Retract>> for Retract {
                type Value = ::planus::Offset<Retract>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<Retract>],
                    bytes: *mut ::core::mem::MaybeUninit<u8>,
                    buffer_position: u32,
                ) {
                    let bytes = bytes as *mut [::core::mem::MaybeUninit<u8>; 4];
                    for (i, v) in ::core::iter::Iterator::enumerate(values.iter()) {
                        ::planus::WriteAsPrimitive::write(
                            v,
                            ::planus::Cursor::new(unsafe { &mut *bytes.add(i) }),
                            buffer_position - (Self::STRIDE * i) as u32,
                        );
                    }
                }
            }

            impl<'a> ::planus::ReadAsRoot<'a> for RetractRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[RetractRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `Attestation` in the namespace `hypermind.schema`
            ///
            /// Generated from these locations:
            /// * Table `Attestation` in the file `schemas/events.fbs:170`
            #[derive(
                Clone,
                Debug,
                PartialEq,
                PartialOrd,
                Eq,
                Ord,
                Hash,
                ::serde::Serialize,
                ::serde::Deserialize,
            )]
            pub struct Attestation {
                /// The field `target_lsn` in the table `Attestation`
                pub target_lsn: u64,
                /// The field `disposition` in the table `Attestation`
                pub disposition: self::AttestationDisposition,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for Attestation {
                fn default() -> Self {
                    Self {
                        target_lsn: 0,
                        disposition: self::AttestationDisposition::Used,
                    }
                }
            }

            impl Attestation {
                /// Creates a [AttestationBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> AttestationBuilder<()> {
                    AttestationBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_target_lsn: impl ::planus::WriteAsDefault<u64, u64>,
                    field_disposition: impl ::planus::WriteAsDefault<
                        self::AttestationDisposition,
                        self::AttestationDisposition,
                    >,
                ) -> ::planus::Offset<Self> {
                    let prepared_target_lsn = field_target_lsn.prepare(builder, &0);
                    let prepared_disposition =
                        field_disposition.prepare(builder, &self::AttestationDisposition::Used);

                    let mut table_writer: ::planus::table_writer::TableWriter<8> =
                        ::core::default::Default::default();
                    if prepared_target_lsn.is_some() {
                        table_writer.write_entry::<u64>(0);
                    }
                    if prepared_disposition.is_some() {
                        table_writer.write_entry::<self::AttestationDisposition>(1);
                    }

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            if let ::core::option::Option::Some(prepared_target_lsn) =
                                prepared_target_lsn
                            {
                                object_writer.write::<_, _, 8>(&prepared_target_lsn);
                            }
                            if let ::core::option::Option::Some(prepared_disposition) =
                                prepared_disposition
                            {
                                object_writer.write::<_, _, 1>(&prepared_disposition);
                            }
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<Attestation>> for Attestation {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<Attestation> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<Attestation>> for Attestation {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Attestation>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<Attestation> for Attestation {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<Attestation> {
                    Attestation::create(builder, self.target_lsn, self.disposition)
                }
            }

            /// Builder for serializing an instance of the [Attestation] type.
            ///
            /// Can be created using the [Attestation::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct AttestationBuilder<State>(State);

            impl AttestationBuilder<()> {
                /// Setter for the [`target_lsn` field](Attestation#structfield.target_lsn).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn target_lsn<T0>(self, value: T0) -> AttestationBuilder<(T0,)>
                where
                    T0: ::planus::WriteAsDefault<u64, u64>,
                {
                    AttestationBuilder((value,))
                }

                /// Sets the [`target_lsn` field](Attestation#structfield.target_lsn) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn target_lsn_as_default(
                    self,
                ) -> AttestationBuilder<(::planus::DefaultValue,)> {
                    self.target_lsn(::planus::DefaultValue)
                }
            }

            impl<T0> AttestationBuilder<(T0,)> {
                /// Setter for the [`disposition` field](Attestation#structfield.disposition).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn disposition<T1>(self, value: T1) -> AttestationBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAsDefault<
                            self::AttestationDisposition,
                            self::AttestationDisposition,
                        >,
                {
                    let (v0,) = self.0;
                    AttestationBuilder((v0, value))
                }

                /// Sets the [`disposition` field](Attestation#structfield.disposition) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn disposition_as_default(
                    self,
                ) -> AttestationBuilder<(T0, ::planus::DefaultValue)> {
                    self.disposition(::planus::DefaultValue)
                }
            }

            impl<T0, T1> AttestationBuilder<(T0, T1)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [Attestation].
                #[inline]
                pub fn finish(
                    self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<Attestation>
                where
                    Self: ::planus::WriteAsOffset<Attestation>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u64, u64>,
                T1: ::planus::WriteAsDefault<
                        self::AttestationDisposition,
                        self::AttestationDisposition,
                    >,
            > ::planus::WriteAs<::planus::Offset<Attestation>> for AttestationBuilder<(T0, T1)>
            {
                type Prepared = ::planus::Offset<Attestation>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<Attestation> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u64, u64>,
                T1: ::planus::WriteAsDefault<
                        self::AttestationDisposition,
                        self::AttestationDisposition,
                    >,
            > ::planus::WriteAsOptional<::planus::Offset<Attestation>>
                for AttestationBuilder<(T0, T1)>
            {
                type Prepared = ::planus::Offset<Attestation>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Attestation>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u64, u64>,
                T1: ::planus::WriteAsDefault<
                        self::AttestationDisposition,
                        self::AttestationDisposition,
                    >,
            > ::planus::WriteAsOffset<Attestation> for AttestationBuilder<(T0, T1)>
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<Attestation> {
                    let (v0, v1) = &self.0;
                    Attestation::create(builder, v0, v1)
                }
            }

            /// Reference to a deserialized [Attestation].
            #[derive(Copy, Clone)]
            pub struct AttestationRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> AttestationRef<'a> {
                /// Getter for the [`target_lsn` field](Attestation#structfield.target_lsn).
                #[inline]
                pub fn target_lsn(&self) -> ::planus::Result<u64> {
                    ::core::result::Result::Ok(
                        self.0.access(0, "Attestation", "target_lsn")?.unwrap_or(0),
                    )
                }

                /// Getter for the [`disposition` field](Attestation#structfield.disposition).
                #[inline]
                pub fn disposition(&self) -> ::planus::Result<self::AttestationDisposition> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(1, "Attestation", "disposition")?
                            .unwrap_or(self::AttestationDisposition::Used),
                    )
                }
            }

            impl<'a> ::core::fmt::Debug for AttestationRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("AttestationRef");
                    f.field("target_lsn", &self.target_lsn());
                    f.field("disposition", &self.disposition());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<AttestationRef<'a>> for Attestation {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: AttestationRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        target_lsn: ::core::convert::TryInto::try_into(value.target_lsn()?)?,
                        disposition: ::core::convert::TryInto::try_into(value.disposition()?)?,
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for AttestationRef<'a> {
                #[inline]
                fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::ErrorKind> {
                    ::core::result::Result::Ok(Self(::planus::table_reader::Table::from_buffer(
                        buffer, offset,
                    )?))
                }
            }

            impl<'a> ::planus::VectorReadInner<'a> for AttestationRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[AttestationRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<Attestation>> for Attestation {
                type Value = ::planus::Offset<Attestation>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<Attestation>],
                    bytes: *mut ::core::mem::MaybeUninit<u8>,
                    buffer_position: u32,
                ) {
                    let bytes = bytes as *mut [::core::mem::MaybeUninit<u8>; 4];
                    for (i, v) in ::core::iter::Iterator::enumerate(values.iter()) {
                        ::planus::WriteAsPrimitive::write(
                            v,
                            ::planus::Cursor::new(unsafe { &mut *bytes.add(i) }),
                            buffer_position - (Self::STRIDE * i) as u32,
                        );
                    }
                }
            }

            impl<'a> ::planus::ReadAsRoot<'a> for AttestationRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[AttestationRef]", "read_as_root", 0)
                    })
                }
            }

            /// The union `EventPayload` in the namespace `hypermind.schema`
            ///
            /// Generated from these locations:
            /// * Union `EventPayload` in the file `schemas/events.fbs:175`
            #[derive(
                Clone,
                Debug,
                PartialEq,
                PartialOrd,
                Eq,
                Ord,
                Hash,
                ::serde::Serialize,
                ::serde::Deserialize,
            )]
            pub enum EventPayload {
                /// The variant of type `UserMsg` in the union `EventPayload`
                UserMsg(::planus::alloc::boxed::Box<self::UserMsg>),

                /// The variant of type `DeliveredMsg` in the union `EventPayload`
                DeliveredMsg(::planus::alloc::boxed::Box<self::DeliveredMsg>),

                /// The variant of type `ToolCall` in the union `EventPayload`
                ToolCall(::planus::alloc::boxed::Box<self::ToolCall>),

                /// The variant of type `ToolResult` in the union `EventPayload`
                ToolResult(::planus::alloc::boxed::Box<self::ToolResult>),

                /// The variant of type `Reasoning` in the union `EventPayload`
                Reasoning(::planus::alloc::boxed::Box<self::Reasoning>),

                /// The variant of type `ProviderFrame` in the union `EventPayload`
                ProviderFrame(::planus::alloc::boxed::Box<self::ProviderFrame>),

                /// The variant of type `MediaRef` in the union `EventPayload`
                MediaRef(::planus::alloc::boxed::Box<self::MediaRef>),

                /// The variant of type `Effect` in the union `EventPayload`
                Effect(::planus::alloc::boxed::Box<self::Effect>),

                /// The variant of type `Approval` in the union `EventPayload`
                Approval(::planus::alloc::boxed::Box<self::Approval>),

                /// The variant of type `Outcome` in the union `EventPayload`
                Outcome(::planus::alloc::boxed::Box<self::Outcome>),

                /// The variant of type `Checkpoint` in the union `EventPayload`
                Checkpoint(::planus::alloc::boxed::Box<self::Checkpoint>),

                /// The variant of type `Supervisor` in the union `EventPayload`
                Supervisor(::planus::alloc::boxed::Box<self::Supervisor>),

                /// The variant of type `Recovery` in the union `EventPayload`
                Recovery(::planus::alloc::boxed::Box<self::Recovery>),

                /// The variant of type `IntentSet` in the union `EventPayload`
                IntentSet(::planus::alloc::boxed::Box<self::IntentSet>),

                /// The variant of type `LoopOpened` in the union `EventPayload`
                LoopOpened(::planus::alloc::boxed::Box<self::LoopOpened>),

                /// The variant of type `LoopClosed` in the union `EventPayload`
                LoopClosed(::planus::alloc::boxed::Box<self::LoopClosed>),

                /// The variant of type `Assertion` in the union `EventPayload`
                Assertion(::planus::alloc::boxed::Box<self::Assertion>),

                /// The variant of type `Consolidation` in the union `EventPayload`
                Consolidation(::planus::alloc::boxed::Box<self::Consolidation>),

                /// The variant of type `Embedding` in the union `EventPayload`
                Embedding(::planus::alloc::boxed::Box<self::Embedding>),

                /// The variant of type `Retract` in the union `EventPayload`
                Retract(::planus::alloc::boxed::Box<self::Retract>),

                /// The variant of type `Attestation` in the union `EventPayload`
                Attestation(::planus::alloc::boxed::Box<self::Attestation>),

                /// The variant of type `Binding` in the union `EventPayload`
                Binding(::planus::alloc::boxed::Box<self::Binding>),

                /// The variant of type `ProposedAssertion` in the union `EventPayload`
                ProposedAssertion(::planus::alloc::boxed::Box<self::ProposedAssertion>),
            }

            impl EventPayload {
                /// Creates a [EventPayloadBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> EventPayloadBuilder<::planus::Uninitialized> {
                    EventPayloadBuilder(::planus::Uninitialized)
                }

                #[inline]
                pub fn create_user_msg(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::UserMsg>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(1, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_delivered_msg(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::DeliveredMsg>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(2, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_tool_call(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::ToolCall>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(3, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_tool_result(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::ToolResult>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(4, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_reasoning(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::Reasoning>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(5, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_provider_frame(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::ProviderFrame>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(6, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_media_ref(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::MediaRef>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(7, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_effect(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::Effect>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(8, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_approval(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::Approval>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(9, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_outcome(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::Outcome>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(10, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_checkpoint(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::Checkpoint>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(11, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_supervisor(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::Supervisor>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(12, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_recovery(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::Recovery>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(13, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_intent_set(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::IntentSet>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(14, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_loop_opened(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::LoopOpened>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(15, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_loop_closed(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::LoopClosed>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(16, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_assertion(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::Assertion>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(17, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_consolidation(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::Consolidation>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(18, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_embedding(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::Embedding>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(19, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_retract(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::Retract>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(20, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_attestation(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::Attestation>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(21, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_binding(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::Binding>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(22, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_proposed_assertion(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::ProposedAssertion>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(23, value.prepare(builder).downcast())
                }
            }

            impl ::planus::WriteAsUnion<EventPayload> for EventPayload {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::UnionOffset<Self> {
                    match self {
                        Self::UserMsg(value) => Self::create_user_msg(builder, value),
                        Self::DeliveredMsg(value) => Self::create_delivered_msg(builder, value),
                        Self::ToolCall(value) => Self::create_tool_call(builder, value),
                        Self::ToolResult(value) => Self::create_tool_result(builder, value),
                        Self::Reasoning(value) => Self::create_reasoning(builder, value),
                        Self::ProviderFrame(value) => Self::create_provider_frame(builder, value),
                        Self::MediaRef(value) => Self::create_media_ref(builder, value),
                        Self::Effect(value) => Self::create_effect(builder, value),
                        Self::Approval(value) => Self::create_approval(builder, value),
                        Self::Outcome(value) => Self::create_outcome(builder, value),
                        Self::Checkpoint(value) => Self::create_checkpoint(builder, value),
                        Self::Supervisor(value) => Self::create_supervisor(builder, value),
                        Self::Recovery(value) => Self::create_recovery(builder, value),
                        Self::IntentSet(value) => Self::create_intent_set(builder, value),
                        Self::LoopOpened(value) => Self::create_loop_opened(builder, value),
                        Self::LoopClosed(value) => Self::create_loop_closed(builder, value),
                        Self::Assertion(value) => Self::create_assertion(builder, value),
                        Self::Consolidation(value) => Self::create_consolidation(builder, value),
                        Self::Embedding(value) => Self::create_embedding(builder, value),
                        Self::Retract(value) => Self::create_retract(builder, value),
                        Self::Attestation(value) => Self::create_attestation(builder, value),
                        Self::Binding(value) => Self::create_binding(builder, value),
                        Self::ProposedAssertion(value) => {
                            Self::create_proposed_assertion(builder, value)
                        }
                    }
                }
            }

            impl ::planus::WriteAsOptionalUnion<EventPayload> for EventPayload {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<Self>> {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }

            /// Builder for serializing an instance of the [EventPayload] type.
            ///
            /// Can be created using the [EventPayload::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct EventPayloadBuilder<T>(T);

            impl EventPayloadBuilder<::planus::Uninitialized> {
                /// Creates an instance of the [`UserMsg` variant](EventPayload#variant.UserMsg).
                #[inline]
                pub fn user_msg<T>(
                    self,
                    value: T,
                ) -> EventPayloadBuilder<::planus::Initialized<1, T>>
                where
                    T: ::planus::WriteAsOffset<self::UserMsg>,
                {
                    EventPayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`DeliveredMsg` variant](EventPayload#variant.DeliveredMsg).
                #[inline]
                pub fn delivered_msg<T>(
                    self,
                    value: T,
                ) -> EventPayloadBuilder<::planus::Initialized<2, T>>
                where
                    T: ::planus::WriteAsOffset<self::DeliveredMsg>,
                {
                    EventPayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`ToolCall` variant](EventPayload#variant.ToolCall).
                #[inline]
                pub fn tool_call<T>(
                    self,
                    value: T,
                ) -> EventPayloadBuilder<::planus::Initialized<3, T>>
                where
                    T: ::planus::WriteAsOffset<self::ToolCall>,
                {
                    EventPayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`ToolResult` variant](EventPayload#variant.ToolResult).
                #[inline]
                pub fn tool_result<T>(
                    self,
                    value: T,
                ) -> EventPayloadBuilder<::planus::Initialized<4, T>>
                where
                    T: ::planus::WriteAsOffset<self::ToolResult>,
                {
                    EventPayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`Reasoning` variant](EventPayload#variant.Reasoning).
                #[inline]
                pub fn reasoning<T>(
                    self,
                    value: T,
                ) -> EventPayloadBuilder<::planus::Initialized<5, T>>
                where
                    T: ::planus::WriteAsOffset<self::Reasoning>,
                {
                    EventPayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`ProviderFrame` variant](EventPayload#variant.ProviderFrame).
                #[inline]
                pub fn provider_frame<T>(
                    self,
                    value: T,
                ) -> EventPayloadBuilder<::planus::Initialized<6, T>>
                where
                    T: ::planus::WriteAsOffset<self::ProviderFrame>,
                {
                    EventPayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`MediaRef` variant](EventPayload#variant.MediaRef).
                #[inline]
                pub fn media_ref<T>(
                    self,
                    value: T,
                ) -> EventPayloadBuilder<::planus::Initialized<7, T>>
                where
                    T: ::planus::WriteAsOffset<self::MediaRef>,
                {
                    EventPayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`Effect` variant](EventPayload#variant.Effect).
                #[inline]
                pub fn effect<T>(self, value: T) -> EventPayloadBuilder<::planus::Initialized<8, T>>
                where
                    T: ::planus::WriteAsOffset<self::Effect>,
                {
                    EventPayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`Approval` variant](EventPayload#variant.Approval).
                #[inline]
                pub fn approval<T>(
                    self,
                    value: T,
                ) -> EventPayloadBuilder<::planus::Initialized<9, T>>
                where
                    T: ::planus::WriteAsOffset<self::Approval>,
                {
                    EventPayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`Outcome` variant](EventPayload#variant.Outcome).
                #[inline]
                pub fn outcome<T>(
                    self,
                    value: T,
                ) -> EventPayloadBuilder<::planus::Initialized<10, T>>
                where
                    T: ::planus::WriteAsOffset<self::Outcome>,
                {
                    EventPayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`Checkpoint` variant](EventPayload#variant.Checkpoint).
                #[inline]
                pub fn checkpoint<T>(
                    self,
                    value: T,
                ) -> EventPayloadBuilder<::planus::Initialized<11, T>>
                where
                    T: ::planus::WriteAsOffset<self::Checkpoint>,
                {
                    EventPayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`Supervisor` variant](EventPayload#variant.Supervisor).
                #[inline]
                pub fn supervisor<T>(
                    self,
                    value: T,
                ) -> EventPayloadBuilder<::planus::Initialized<12, T>>
                where
                    T: ::planus::WriteAsOffset<self::Supervisor>,
                {
                    EventPayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`Recovery` variant](EventPayload#variant.Recovery).
                #[inline]
                pub fn recovery<T>(
                    self,
                    value: T,
                ) -> EventPayloadBuilder<::planus::Initialized<13, T>>
                where
                    T: ::planus::WriteAsOffset<self::Recovery>,
                {
                    EventPayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`IntentSet` variant](EventPayload#variant.IntentSet).
                #[inline]
                pub fn intent_set<T>(
                    self,
                    value: T,
                ) -> EventPayloadBuilder<::planus::Initialized<14, T>>
                where
                    T: ::planus::WriteAsOffset<self::IntentSet>,
                {
                    EventPayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`LoopOpened` variant](EventPayload#variant.LoopOpened).
                #[inline]
                pub fn loop_opened<T>(
                    self,
                    value: T,
                ) -> EventPayloadBuilder<::planus::Initialized<15, T>>
                where
                    T: ::planus::WriteAsOffset<self::LoopOpened>,
                {
                    EventPayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`LoopClosed` variant](EventPayload#variant.LoopClosed).
                #[inline]
                pub fn loop_closed<T>(
                    self,
                    value: T,
                ) -> EventPayloadBuilder<::planus::Initialized<16, T>>
                where
                    T: ::planus::WriteAsOffset<self::LoopClosed>,
                {
                    EventPayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`Assertion` variant](EventPayload#variant.Assertion).
                #[inline]
                pub fn assertion<T>(
                    self,
                    value: T,
                ) -> EventPayloadBuilder<::planus::Initialized<17, T>>
                where
                    T: ::planus::WriteAsOffset<self::Assertion>,
                {
                    EventPayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`Consolidation` variant](EventPayload#variant.Consolidation).
                #[inline]
                pub fn consolidation<T>(
                    self,
                    value: T,
                ) -> EventPayloadBuilder<::planus::Initialized<18, T>>
                where
                    T: ::planus::WriteAsOffset<self::Consolidation>,
                {
                    EventPayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`Embedding` variant](EventPayload#variant.Embedding).
                #[inline]
                pub fn embedding<T>(
                    self,
                    value: T,
                ) -> EventPayloadBuilder<::planus::Initialized<19, T>>
                where
                    T: ::planus::WriteAsOffset<self::Embedding>,
                {
                    EventPayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`Retract` variant](EventPayload#variant.Retract).
                #[inline]
                pub fn retract<T>(
                    self,
                    value: T,
                ) -> EventPayloadBuilder<::planus::Initialized<20, T>>
                where
                    T: ::planus::WriteAsOffset<self::Retract>,
                {
                    EventPayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`Attestation` variant](EventPayload#variant.Attestation).
                #[inline]
                pub fn attestation<T>(
                    self,
                    value: T,
                ) -> EventPayloadBuilder<::planus::Initialized<21, T>>
                where
                    T: ::planus::WriteAsOffset<self::Attestation>,
                {
                    EventPayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`Binding` variant](EventPayload#variant.Binding).
                #[inline]
                pub fn binding<T>(
                    self,
                    value: T,
                ) -> EventPayloadBuilder<::planus::Initialized<22, T>>
                where
                    T: ::planus::WriteAsOffset<self::Binding>,
                {
                    EventPayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`ProposedAssertion` variant](EventPayload#variant.ProposedAssertion).
                #[inline]
                pub fn proposed_assertion<T>(
                    self,
                    value: T,
                ) -> EventPayloadBuilder<::planus::Initialized<23, T>>
                where
                    T: ::planus::WriteAsOffset<self::ProposedAssertion>,
                {
                    EventPayloadBuilder(::planus::Initialized(value))
                }
            }

            impl<const N: u8, T> EventPayloadBuilder<::planus::Initialized<N, T>> {
                /// Finish writing the builder to get an [UnionOffset](::planus::UnionOffset) to a serialized [EventPayload].
                #[inline]
                pub fn finish(
                    self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<EventPayload>
                where
                    Self: ::planus::WriteAsUnion<EventPayload>,
                {
                    ::planus::WriteAsUnion::prepare(&self, builder)
                }
            }

            impl<T> ::planus::WriteAsUnion<EventPayload> for EventPayloadBuilder<::planus::Initialized<1, T>>
            where
                T: ::planus::WriteAsOffset<self::UserMsg>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<EventPayload> {
                    ::planus::UnionOffset::new(1, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<EventPayload>
                for EventPayloadBuilder<::planus::Initialized<1, T>>
            where
                T: ::planus::WriteAsOffset<self::UserMsg>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<EventPayload>> {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<EventPayload> for EventPayloadBuilder<::planus::Initialized<2, T>>
            where
                T: ::planus::WriteAsOffset<self::DeliveredMsg>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<EventPayload> {
                    ::planus::UnionOffset::new(2, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<EventPayload>
                for EventPayloadBuilder<::planus::Initialized<2, T>>
            where
                T: ::planus::WriteAsOffset<self::DeliveredMsg>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<EventPayload>> {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<EventPayload> for EventPayloadBuilder<::planus::Initialized<3, T>>
            where
                T: ::planus::WriteAsOffset<self::ToolCall>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<EventPayload> {
                    ::planus::UnionOffset::new(3, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<EventPayload>
                for EventPayloadBuilder<::planus::Initialized<3, T>>
            where
                T: ::planus::WriteAsOffset<self::ToolCall>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<EventPayload>> {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<EventPayload> for EventPayloadBuilder<::planus::Initialized<4, T>>
            where
                T: ::planus::WriteAsOffset<self::ToolResult>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<EventPayload> {
                    ::planus::UnionOffset::new(4, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<EventPayload>
                for EventPayloadBuilder<::planus::Initialized<4, T>>
            where
                T: ::planus::WriteAsOffset<self::ToolResult>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<EventPayload>> {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<EventPayload> for EventPayloadBuilder<::planus::Initialized<5, T>>
            where
                T: ::planus::WriteAsOffset<self::Reasoning>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<EventPayload> {
                    ::planus::UnionOffset::new(5, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<EventPayload>
                for EventPayloadBuilder<::planus::Initialized<5, T>>
            where
                T: ::planus::WriteAsOffset<self::Reasoning>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<EventPayload>> {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<EventPayload> for EventPayloadBuilder<::planus::Initialized<6, T>>
            where
                T: ::planus::WriteAsOffset<self::ProviderFrame>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<EventPayload> {
                    ::planus::UnionOffset::new(6, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<EventPayload>
                for EventPayloadBuilder<::planus::Initialized<6, T>>
            where
                T: ::planus::WriteAsOffset<self::ProviderFrame>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<EventPayload>> {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<EventPayload> for EventPayloadBuilder<::planus::Initialized<7, T>>
            where
                T: ::planus::WriteAsOffset<self::MediaRef>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<EventPayload> {
                    ::planus::UnionOffset::new(7, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<EventPayload>
                for EventPayloadBuilder<::planus::Initialized<7, T>>
            where
                T: ::planus::WriteAsOffset<self::MediaRef>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<EventPayload>> {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<EventPayload> for EventPayloadBuilder<::planus::Initialized<8, T>>
            where
                T: ::planus::WriteAsOffset<self::Effect>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<EventPayload> {
                    ::planus::UnionOffset::new(8, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<EventPayload>
                for EventPayloadBuilder<::planus::Initialized<8, T>>
            where
                T: ::planus::WriteAsOffset<self::Effect>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<EventPayload>> {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<EventPayload> for EventPayloadBuilder<::planus::Initialized<9, T>>
            where
                T: ::planus::WriteAsOffset<self::Approval>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<EventPayload> {
                    ::planus::UnionOffset::new(9, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<EventPayload>
                for EventPayloadBuilder<::planus::Initialized<9, T>>
            where
                T: ::planus::WriteAsOffset<self::Approval>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<EventPayload>> {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<EventPayload> for EventPayloadBuilder<::planus::Initialized<10, T>>
            where
                T: ::planus::WriteAsOffset<self::Outcome>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<EventPayload> {
                    ::planus::UnionOffset::new(10, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<EventPayload>
                for EventPayloadBuilder<::planus::Initialized<10, T>>
            where
                T: ::planus::WriteAsOffset<self::Outcome>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<EventPayload>> {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<EventPayload> for EventPayloadBuilder<::planus::Initialized<11, T>>
            where
                T: ::planus::WriteAsOffset<self::Checkpoint>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<EventPayload> {
                    ::planus::UnionOffset::new(11, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<EventPayload>
                for EventPayloadBuilder<::planus::Initialized<11, T>>
            where
                T: ::planus::WriteAsOffset<self::Checkpoint>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<EventPayload>> {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<EventPayload> for EventPayloadBuilder<::planus::Initialized<12, T>>
            where
                T: ::planus::WriteAsOffset<self::Supervisor>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<EventPayload> {
                    ::planus::UnionOffset::new(12, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<EventPayload>
                for EventPayloadBuilder<::planus::Initialized<12, T>>
            where
                T: ::planus::WriteAsOffset<self::Supervisor>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<EventPayload>> {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<EventPayload> for EventPayloadBuilder<::planus::Initialized<13, T>>
            where
                T: ::planus::WriteAsOffset<self::Recovery>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<EventPayload> {
                    ::planus::UnionOffset::new(13, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<EventPayload>
                for EventPayloadBuilder<::planus::Initialized<13, T>>
            where
                T: ::planus::WriteAsOffset<self::Recovery>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<EventPayload>> {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<EventPayload> for EventPayloadBuilder<::planus::Initialized<14, T>>
            where
                T: ::planus::WriteAsOffset<self::IntentSet>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<EventPayload> {
                    ::planus::UnionOffset::new(14, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<EventPayload>
                for EventPayloadBuilder<::planus::Initialized<14, T>>
            where
                T: ::planus::WriteAsOffset<self::IntentSet>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<EventPayload>> {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<EventPayload> for EventPayloadBuilder<::planus::Initialized<15, T>>
            where
                T: ::planus::WriteAsOffset<self::LoopOpened>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<EventPayload> {
                    ::planus::UnionOffset::new(15, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<EventPayload>
                for EventPayloadBuilder<::planus::Initialized<15, T>>
            where
                T: ::planus::WriteAsOffset<self::LoopOpened>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<EventPayload>> {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<EventPayload> for EventPayloadBuilder<::planus::Initialized<16, T>>
            where
                T: ::planus::WriteAsOffset<self::LoopClosed>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<EventPayload> {
                    ::planus::UnionOffset::new(16, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<EventPayload>
                for EventPayloadBuilder<::planus::Initialized<16, T>>
            where
                T: ::planus::WriteAsOffset<self::LoopClosed>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<EventPayload>> {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<EventPayload> for EventPayloadBuilder<::planus::Initialized<17, T>>
            where
                T: ::planus::WriteAsOffset<self::Assertion>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<EventPayload> {
                    ::planus::UnionOffset::new(17, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<EventPayload>
                for EventPayloadBuilder<::planus::Initialized<17, T>>
            where
                T: ::planus::WriteAsOffset<self::Assertion>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<EventPayload>> {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<EventPayload> for EventPayloadBuilder<::planus::Initialized<18, T>>
            where
                T: ::planus::WriteAsOffset<self::Consolidation>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<EventPayload> {
                    ::planus::UnionOffset::new(18, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<EventPayload>
                for EventPayloadBuilder<::planus::Initialized<18, T>>
            where
                T: ::planus::WriteAsOffset<self::Consolidation>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<EventPayload>> {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<EventPayload> for EventPayloadBuilder<::planus::Initialized<19, T>>
            where
                T: ::planus::WriteAsOffset<self::Embedding>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<EventPayload> {
                    ::planus::UnionOffset::new(19, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<EventPayload>
                for EventPayloadBuilder<::planus::Initialized<19, T>>
            where
                T: ::planus::WriteAsOffset<self::Embedding>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<EventPayload>> {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<EventPayload> for EventPayloadBuilder<::planus::Initialized<20, T>>
            where
                T: ::planus::WriteAsOffset<self::Retract>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<EventPayload> {
                    ::planus::UnionOffset::new(20, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<EventPayload>
                for EventPayloadBuilder<::planus::Initialized<20, T>>
            where
                T: ::planus::WriteAsOffset<self::Retract>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<EventPayload>> {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<EventPayload> for EventPayloadBuilder<::planus::Initialized<21, T>>
            where
                T: ::planus::WriteAsOffset<self::Attestation>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<EventPayload> {
                    ::planus::UnionOffset::new(21, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<EventPayload>
                for EventPayloadBuilder<::planus::Initialized<21, T>>
            where
                T: ::planus::WriteAsOffset<self::Attestation>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<EventPayload>> {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<EventPayload> for EventPayloadBuilder<::planus::Initialized<22, T>>
            where
                T: ::planus::WriteAsOffset<self::Binding>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<EventPayload> {
                    ::planus::UnionOffset::new(22, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<EventPayload>
                for EventPayloadBuilder<::planus::Initialized<22, T>>
            where
                T: ::planus::WriteAsOffset<self::Binding>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<EventPayload>> {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<EventPayload> for EventPayloadBuilder<::planus::Initialized<23, T>>
            where
                T: ::planus::WriteAsOffset<self::ProposedAssertion>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<EventPayload> {
                    ::planus::UnionOffset::new(23, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<EventPayload>
                for EventPayloadBuilder<::planus::Initialized<23, T>>
            where
                T: ::planus::WriteAsOffset<self::ProposedAssertion>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<EventPayload>> {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }

            /// Reference to a deserialized [EventPayload].
            #[derive(Copy, Clone, Debug)]
            pub enum EventPayloadRef<'a> {
                UserMsg(self::UserMsgRef<'a>),
                DeliveredMsg(self::DeliveredMsgRef<'a>),
                ToolCall(self::ToolCallRef<'a>),
                ToolResult(self::ToolResultRef<'a>),
                Reasoning(self::ReasoningRef<'a>),
                ProviderFrame(self::ProviderFrameRef<'a>),
                MediaRef(self::MediaRefRef<'a>),
                Effect(self::EffectRef<'a>),
                Approval(self::ApprovalRef<'a>),
                Outcome(self::OutcomeRef<'a>),
                Checkpoint(self::CheckpointRef<'a>),
                Supervisor(self::SupervisorRef<'a>),
                Recovery(self::RecoveryRef<'a>),
                IntentSet(self::IntentSetRef<'a>),
                LoopOpened(self::LoopOpenedRef<'a>),
                LoopClosed(self::LoopClosedRef<'a>),
                Assertion(self::AssertionRef<'a>),
                Consolidation(self::ConsolidationRef<'a>),
                Embedding(self::EmbeddingRef<'a>),
                Retract(self::RetractRef<'a>),
                Attestation(self::AttestationRef<'a>),
                Binding(self::BindingRef<'a>),
                ProposedAssertion(self::ProposedAssertionRef<'a>),
            }

            impl<'a> ::core::convert::TryFrom<EventPayloadRef<'a>> for EventPayload {
                type Error = ::planus::Error;

                fn try_from(value: EventPayloadRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(match value {
                        EventPayloadRef::UserMsg(value) => {
                            Self::UserMsg(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        EventPayloadRef::DeliveredMsg(value) => {
                            Self::DeliveredMsg(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        EventPayloadRef::ToolCall(value) => {
                            Self::ToolCall(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        EventPayloadRef::ToolResult(value) => {
                            Self::ToolResult(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        EventPayloadRef::Reasoning(value) => {
                            Self::Reasoning(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        EventPayloadRef::ProviderFrame(value) => {
                            Self::ProviderFrame(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        EventPayloadRef::MediaRef(value) => {
                            Self::MediaRef(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        EventPayloadRef::Effect(value) => {
                            Self::Effect(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        EventPayloadRef::Approval(value) => {
                            Self::Approval(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        EventPayloadRef::Outcome(value) => {
                            Self::Outcome(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        EventPayloadRef::Checkpoint(value) => {
                            Self::Checkpoint(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        EventPayloadRef::Supervisor(value) => {
                            Self::Supervisor(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        EventPayloadRef::Recovery(value) => {
                            Self::Recovery(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        EventPayloadRef::IntentSet(value) => {
                            Self::IntentSet(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        EventPayloadRef::LoopOpened(value) => {
                            Self::LoopOpened(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        EventPayloadRef::LoopClosed(value) => {
                            Self::LoopClosed(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        EventPayloadRef::Assertion(value) => {
                            Self::Assertion(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        EventPayloadRef::Consolidation(value) => {
                            Self::Consolidation(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        EventPayloadRef::Embedding(value) => {
                            Self::Embedding(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        EventPayloadRef::Retract(value) => {
                            Self::Retract(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        EventPayloadRef::Attestation(value) => {
                            Self::Attestation(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        EventPayloadRef::Binding(value) => {
                            Self::Binding(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        EventPayloadRef::ProposedAssertion(value) => {
                            Self::ProposedAssertion(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }
                    })
                }
            }

            impl<'a> ::planus::TableReadUnion<'a> for EventPayloadRef<'a> {
                fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    tag: u8,
                    field_offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::ErrorKind> {
                    match tag {
                        1 => ::core::result::Result::Ok(Self::UserMsg(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        2 => ::core::result::Result::Ok(Self::DeliveredMsg(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        3 => ::core::result::Result::Ok(Self::ToolCall(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        4 => ::core::result::Result::Ok(Self::ToolResult(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        5 => ::core::result::Result::Ok(Self::Reasoning(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        6 => ::core::result::Result::Ok(Self::ProviderFrame(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        7 => ::core::result::Result::Ok(Self::MediaRef(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        8 => ::core::result::Result::Ok(Self::Effect(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        9 => ::core::result::Result::Ok(Self::Approval(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        10 => ::core::result::Result::Ok(Self::Outcome(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        11 => ::core::result::Result::Ok(Self::Checkpoint(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        12 => ::core::result::Result::Ok(Self::Supervisor(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        13 => ::core::result::Result::Ok(Self::Recovery(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        14 => ::core::result::Result::Ok(Self::IntentSet(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        15 => ::core::result::Result::Ok(Self::LoopOpened(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        16 => ::core::result::Result::Ok(Self::LoopClosed(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        17 => ::core::result::Result::Ok(Self::Assertion(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        18 => ::core::result::Result::Ok(Self::Consolidation(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        19 => ::core::result::Result::Ok(Self::Embedding(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        20 => ::core::result::Result::Ok(Self::Retract(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        21 => ::core::result::Result::Ok(Self::Attestation(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        22 => ::core::result::Result::Ok(Self::Binding(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        23 => ::core::result::Result::Ok(Self::ProposedAssertion(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        _ => ::core::result::Result::Err(
                            ::planus::errors::ErrorKind::UnknownUnionTag { tag },
                        ),
                    }
                }
            }

            impl<'a> ::planus::VectorReadUnion<'a> for EventPayloadRef<'a> {
                const VECTOR_NAME: &'static str = "[EventPayloadRef]";
            }

            /// The table `EventEnvelope` in the namespace `hypermind.schema`
            ///
            /// Generated from these locations:
            /// * Table `EventEnvelope` in the file `schemas/events.fbs:201`
            #[derive(
                Clone, Debug, PartialEq, PartialOrd, ::serde::Serialize, ::serde::Deserialize,
            )]
            pub struct EventEnvelope {
                /// The field `schema_version` in the table `EventEnvelope`
                pub schema_version: u16,
                /// The field `payload` in the table `EventEnvelope`
                pub payload: self::EventPayload,
                /// The field `connection_id` in the table `EventEnvelope`
                pub connection_id: ::core::option::Option<::planus::alloc::vec::Vec<u8>>,
                /// The field `client_seq` in the table `EventEnvelope`
                pub client_seq: u64,
                /// The field `client_event_index` in the table `EventEnvelope`
                pub client_event_index: u32,
                /// The field `client_event_count` in the table `EventEnvelope`
                pub client_event_count: u32,
                /// The field `origin_actor` in the table `EventEnvelope`
                pub origin_actor: u16,
                /// The field `run_id` in the table `EventEnvelope`
                pub run_id: ::core::option::Option<::planus::alloc::vec::Vec<u8>>,
                /// The field `model_provenance` in the table `EventEnvelope`
                pub model_provenance:
                    ::core::option::Option<::planus::alloc::boxed::Box<self::ModelProvenance>>,
                /// The field `authority` in the table `EventEnvelope`
                pub authority: self::Authority,
                /// The field `retention` in the table `EventEnvelope`
                pub retention: self::Retention,
                /// The field `sensitivity` in the table `EventEnvelope`
                pub sensitivity: self::Sensitivity,
                /// The field `event_time_ns` in the table `EventEnvelope`
                pub event_time_ns: i64,
            }

            impl EventEnvelope {
                /// Creates a [EventEnvelopeBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> EventEnvelopeBuilder<()> {
                    EventEnvelopeBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_schema_version: impl ::planus::WriteAsDefault<u16, u16>,
                    field_payload: impl ::planus::WriteAsUnion<self::EventPayload>,
                    field_connection_id: impl ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                    field_client_seq: impl ::planus::WriteAsDefault<u64, u64>,
                    field_client_event_index: impl ::planus::WriteAsDefault<u32, u32>,
                    field_client_event_count: impl ::planus::WriteAsDefault<u32, u32>,
                    field_origin_actor: impl ::planus::WriteAsDefault<u16, u16>,
                    field_run_id: impl ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                    field_model_provenance: impl ::planus::WriteAsOptional<
                        ::planus::Offset<self::ModelProvenance>,
                    >,
                    field_authority: impl ::planus::WriteAsDefault<self::Authority, self::Authority>,
                    field_retention: impl ::planus::WriteAsDefault<self::Retention, self::Retention>,
                    field_sensitivity: impl ::planus::WriteAsDefault<
                        self::Sensitivity,
                        self::Sensitivity,
                    >,
                    field_event_time_ns: impl ::planus::WriteAsDefault<i64, i64>,
                ) -> ::planus::Offset<Self> {
                    let prepared_schema_version = field_schema_version.prepare(builder, &1);
                    let prepared_payload = field_payload.prepare(builder);
                    let prepared_connection_id = field_connection_id.prepare(builder);
                    let prepared_client_seq = field_client_seq.prepare(builder, &0);
                    let prepared_client_event_index = field_client_event_index.prepare(builder, &0);
                    let prepared_client_event_count = field_client_event_count.prepare(builder, &0);
                    let prepared_origin_actor = field_origin_actor.prepare(builder, &0);
                    let prepared_run_id = field_run_id.prepare(builder);
                    let prepared_model_provenance = field_model_provenance.prepare(builder);
                    let prepared_authority =
                        field_authority.prepare(builder, &self::Authority::UserAsserted);
                    let prepared_retention =
                        field_retention.prepare(builder, &self::Retention::CurrentState);
                    let prepared_sensitivity =
                        field_sensitivity.prepare(builder, &self::Sensitivity::Public);
                    let prepared_event_time_ns = field_event_time_ns.prepare(builder, &0);

                    let mut table_writer: ::planus::table_writer::TableWriter<32> =
                        ::core::default::Default::default();
                    if prepared_client_seq.is_some() {
                        table_writer.write_entry::<u64>(4);
                    }
                    if prepared_event_time_ns.is_some() {
                        table_writer.write_entry::<i64>(13);
                    }
                    table_writer.write_entry::<::planus::Offset<self::EventPayload>>(2);
                    if prepared_connection_id.is_some() {
                        table_writer.write_entry::<::planus::Offset<[u8]>>(3);
                    }
                    if prepared_client_event_index.is_some() {
                        table_writer.write_entry::<u32>(5);
                    }
                    if prepared_client_event_count.is_some() {
                        table_writer.write_entry::<u32>(6);
                    }
                    if prepared_run_id.is_some() {
                        table_writer.write_entry::<::planus::Offset<[u8]>>(8);
                    }
                    if prepared_model_provenance.is_some() {
                        table_writer.write_entry::<::planus::Offset<self::ModelProvenance>>(9);
                    }
                    if prepared_schema_version.is_some() {
                        table_writer.write_entry::<u16>(0);
                    }
                    if prepared_origin_actor.is_some() {
                        table_writer.write_entry::<u16>(7);
                    }
                    table_writer.write_entry::<u8>(1);
                    if prepared_authority.is_some() {
                        table_writer.write_entry::<self::Authority>(10);
                    }
                    if prepared_retention.is_some() {
                        table_writer.write_entry::<self::Retention>(11);
                    }
                    if prepared_sensitivity.is_some() {
                        table_writer.write_entry::<self::Sensitivity>(12);
                    }

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            if let ::core::option::Option::Some(prepared_client_seq) =
                                prepared_client_seq
                            {
                                object_writer.write::<_, _, 8>(&prepared_client_seq);
                            }
                            if let ::core::option::Option::Some(prepared_event_time_ns) =
                                prepared_event_time_ns
                            {
                                object_writer.write::<_, _, 8>(&prepared_event_time_ns);
                            }
                            object_writer.write::<_, _, 4>(&prepared_payload.offset());
                            if let ::core::option::Option::Some(prepared_connection_id) =
                                prepared_connection_id
                            {
                                object_writer.write::<_, _, 4>(&prepared_connection_id);
                            }
                            if let ::core::option::Option::Some(prepared_client_event_index) =
                                prepared_client_event_index
                            {
                                object_writer.write::<_, _, 4>(&prepared_client_event_index);
                            }
                            if let ::core::option::Option::Some(prepared_client_event_count) =
                                prepared_client_event_count
                            {
                                object_writer.write::<_, _, 4>(&prepared_client_event_count);
                            }
                            if let ::core::option::Option::Some(prepared_run_id) = prepared_run_id {
                                object_writer.write::<_, _, 4>(&prepared_run_id);
                            }
                            if let ::core::option::Option::Some(prepared_model_provenance) =
                                prepared_model_provenance
                            {
                                object_writer.write::<_, _, 4>(&prepared_model_provenance);
                            }
                            if let ::core::option::Option::Some(prepared_schema_version) =
                                prepared_schema_version
                            {
                                object_writer.write::<_, _, 2>(&prepared_schema_version);
                            }
                            if let ::core::option::Option::Some(prepared_origin_actor) =
                                prepared_origin_actor
                            {
                                object_writer.write::<_, _, 2>(&prepared_origin_actor);
                            }
                            object_writer.write::<_, _, 1>(&prepared_payload.tag());
                            if let ::core::option::Option::Some(prepared_authority) =
                                prepared_authority
                            {
                                object_writer.write::<_, _, 1>(&prepared_authority);
                            }
                            if let ::core::option::Option::Some(prepared_retention) =
                                prepared_retention
                            {
                                object_writer.write::<_, _, 1>(&prepared_retention);
                            }
                            if let ::core::option::Option::Some(prepared_sensitivity) =
                                prepared_sensitivity
                            {
                                object_writer.write::<_, _, 1>(&prepared_sensitivity);
                            }
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<EventEnvelope>> for EventEnvelope {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<EventEnvelope> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<EventEnvelope>> for EventEnvelope {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<EventEnvelope>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<EventEnvelope> for EventEnvelope {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<EventEnvelope> {
                    EventEnvelope::create(
                        builder,
                        self.schema_version,
                        &self.payload,
                        &self.connection_id,
                        self.client_seq,
                        self.client_event_index,
                        self.client_event_count,
                        self.origin_actor,
                        &self.run_id,
                        &self.model_provenance,
                        self.authority,
                        self.retention,
                        self.sensitivity,
                        self.event_time_ns,
                    )
                }
            }

            /// Builder for serializing an instance of the [EventEnvelope] type.
            ///
            /// Can be created using the [EventEnvelope::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct EventEnvelopeBuilder<State>(State);

            impl EventEnvelopeBuilder<()> {
                /// Setter for the [`schema_version` field](EventEnvelope#structfield.schema_version).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn schema_version<T0>(self, value: T0) -> EventEnvelopeBuilder<(T0,)>
                where
                    T0: ::planus::WriteAsDefault<u16, u16>,
                {
                    EventEnvelopeBuilder((value,))
                }

                /// Sets the [`schema_version` field](EventEnvelope#structfield.schema_version) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn schema_version_as_default(
                    self,
                ) -> EventEnvelopeBuilder<(::planus::DefaultValue,)> {
                    self.schema_version(::planus::DefaultValue)
                }
            }

            impl<T0> EventEnvelopeBuilder<(T0,)> {
                /// Setter for the [`payload` field](EventEnvelope#structfield.payload).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn payload<T1>(self, value: T1) -> EventEnvelopeBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAsUnion<self::EventPayload>,
                {
                    let (v0,) = self.0;
                    EventEnvelopeBuilder((v0, value))
                }
            }

            impl<T0, T1> EventEnvelopeBuilder<(T0, T1)> {
                /// Setter for the [`connection_id` field](EventEnvelope#structfield.connection_id).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn connection_id<T2>(self, value: T2) -> EventEnvelopeBuilder<(T0, T1, T2)>
                where
                    T2: ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                {
                    let (v0, v1) = self.0;
                    EventEnvelopeBuilder((v0, v1, value))
                }

                /// Sets the [`connection_id` field](EventEnvelope#structfield.connection_id) to null.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn connection_id_as_null(self) -> EventEnvelopeBuilder<(T0, T1, ())> {
                    self.connection_id(())
                }
            }

            impl<T0, T1, T2> EventEnvelopeBuilder<(T0, T1, T2)> {
                /// Setter for the [`client_seq` field](EventEnvelope#structfield.client_seq).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn client_seq<T3>(self, value: T3) -> EventEnvelopeBuilder<(T0, T1, T2, T3)>
                where
                    T3: ::planus::WriteAsDefault<u64, u64>,
                {
                    let (v0, v1, v2) = self.0;
                    EventEnvelopeBuilder((v0, v1, v2, value))
                }

                /// Sets the [`client_seq` field](EventEnvelope#structfield.client_seq) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn client_seq_as_default(
                    self,
                ) -> EventEnvelopeBuilder<(T0, T1, T2, ::planus::DefaultValue)> {
                    self.client_seq(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2, T3> EventEnvelopeBuilder<(T0, T1, T2, T3)> {
                /// Setter for the [`client_event_index` field](EventEnvelope#structfield.client_event_index).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn client_event_index<T4>(
                    self,
                    value: T4,
                ) -> EventEnvelopeBuilder<(T0, T1, T2, T3, T4)>
                where
                    T4: ::planus::WriteAsDefault<u32, u32>,
                {
                    let (v0, v1, v2, v3) = self.0;
                    EventEnvelopeBuilder((v0, v1, v2, v3, value))
                }

                /// Sets the [`client_event_index` field](EventEnvelope#structfield.client_event_index) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn client_event_index_as_default(
                    self,
                ) -> EventEnvelopeBuilder<(T0, T1, T2, T3, ::planus::DefaultValue)>
                {
                    self.client_event_index(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2, T3, T4> EventEnvelopeBuilder<(T0, T1, T2, T3, T4)> {
                /// Setter for the [`client_event_count` field](EventEnvelope#structfield.client_event_count).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn client_event_count<T5>(
                    self,
                    value: T5,
                ) -> EventEnvelopeBuilder<(T0, T1, T2, T3, T4, T5)>
                where
                    T5: ::planus::WriteAsDefault<u32, u32>,
                {
                    let (v0, v1, v2, v3, v4) = self.0;
                    EventEnvelopeBuilder((v0, v1, v2, v3, v4, value))
                }

                /// Sets the [`client_event_count` field](EventEnvelope#structfield.client_event_count) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn client_event_count_as_default(
                    self,
                ) -> EventEnvelopeBuilder<(T0, T1, T2, T3, T4, ::planus::DefaultValue)>
                {
                    self.client_event_count(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2, T3, T4, T5> EventEnvelopeBuilder<(T0, T1, T2, T3, T4, T5)> {
                /// Setter for the [`origin_actor` field](EventEnvelope#structfield.origin_actor).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn origin_actor<T6>(
                    self,
                    value: T6,
                ) -> EventEnvelopeBuilder<(T0, T1, T2, T3, T4, T5, T6)>
                where
                    T6: ::planus::WriteAsDefault<u16, u16>,
                {
                    let (v0, v1, v2, v3, v4, v5) = self.0;
                    EventEnvelopeBuilder((v0, v1, v2, v3, v4, v5, value))
                }

                /// Sets the [`origin_actor` field](EventEnvelope#structfield.origin_actor) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn origin_actor_as_default(
                    self,
                ) -> EventEnvelopeBuilder<(T0, T1, T2, T3, T4, T5, ::planus::DefaultValue)>
                {
                    self.origin_actor(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2, T3, T4, T5, T6> EventEnvelopeBuilder<(T0, T1, T2, T3, T4, T5, T6)> {
                /// Setter for the [`run_id` field](EventEnvelope#structfield.run_id).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn run_id<T7>(
                    self,
                    value: T7,
                ) -> EventEnvelopeBuilder<(T0, T1, T2, T3, T4, T5, T6, T7)>
                where
                    T7: ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                {
                    let (v0, v1, v2, v3, v4, v5, v6) = self.0;
                    EventEnvelopeBuilder((v0, v1, v2, v3, v4, v5, v6, value))
                }

                /// Sets the [`run_id` field](EventEnvelope#structfield.run_id) to null.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn run_id_as_null(
                    self,
                ) -> EventEnvelopeBuilder<(T0, T1, T2, T3, T4, T5, T6, ())> {
                    self.run_id(())
                }
            }

            impl<T0, T1, T2, T3, T4, T5, T6, T7> EventEnvelopeBuilder<(T0, T1, T2, T3, T4, T5, T6, T7)> {
                /// Setter for the [`model_provenance` field](EventEnvelope#structfield.model_provenance).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn model_provenance<T8>(
                    self,
                    value: T8,
                ) -> EventEnvelopeBuilder<(T0, T1, T2, T3, T4, T5, T6, T7, T8)>
                where
                    T8: ::planus::WriteAsOptional<::planus::Offset<self::ModelProvenance>>,
                {
                    let (v0, v1, v2, v3, v4, v5, v6, v7) = self.0;
                    EventEnvelopeBuilder((v0, v1, v2, v3, v4, v5, v6, v7, value))
                }

                /// Sets the [`model_provenance` field](EventEnvelope#structfield.model_provenance) to null.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn model_provenance_as_null(
                    self,
                ) -> EventEnvelopeBuilder<(T0, T1, T2, T3, T4, T5, T6, T7, ())> {
                    self.model_provenance(())
                }
            }

            impl<T0, T1, T2, T3, T4, T5, T6, T7, T8>
                EventEnvelopeBuilder<(T0, T1, T2, T3, T4, T5, T6, T7, T8)>
            {
                /// Setter for the [`authority` field](EventEnvelope#structfield.authority).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn authority<T9>(
                    self,
                    value: T9,
                ) -> EventEnvelopeBuilder<(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9)>
                where
                    T9: ::planus::WriteAsDefault<self::Authority, self::Authority>,
                {
                    let (v0, v1, v2, v3, v4, v5, v6, v7, v8) = self.0;
                    EventEnvelopeBuilder((v0, v1, v2, v3, v4, v5, v6, v7, v8, value))
                }

                /// Sets the [`authority` field](EventEnvelope#structfield.authority) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn authority_as_default(
                    self,
                ) -> EventEnvelopeBuilder<(
                    T0,
                    T1,
                    T2,
                    T3,
                    T4,
                    T5,
                    T6,
                    T7,
                    T8,
                    ::planus::DefaultValue,
                )> {
                    self.authority(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9>
                EventEnvelopeBuilder<(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9)>
            {
                /// Setter for the [`retention` field](EventEnvelope#structfield.retention).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn retention<T10>(
                    self,
                    value: T10,
                ) -> EventEnvelopeBuilder<(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10)>
                where
                    T10: ::planus::WriteAsDefault<self::Retention, self::Retention>,
                {
                    let (v0, v1, v2, v3, v4, v5, v6, v7, v8, v9) = self.0;
                    EventEnvelopeBuilder((v0, v1, v2, v3, v4, v5, v6, v7, v8, v9, value))
                }

                /// Sets the [`retention` field](EventEnvelope#structfield.retention) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn retention_as_default(
                    self,
                ) -> EventEnvelopeBuilder<(
                    T0,
                    T1,
                    T2,
                    T3,
                    T4,
                    T5,
                    T6,
                    T7,
                    T8,
                    T9,
                    ::planus::DefaultValue,
                )> {
                    self.retention(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10>
                EventEnvelopeBuilder<(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10)>
            {
                /// Setter for the [`sensitivity` field](EventEnvelope#structfield.sensitivity).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn sensitivity<T11>(
                    self,
                    value: T11,
                ) -> EventEnvelopeBuilder<(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11)>
                where
                    T11: ::planus::WriteAsDefault<self::Sensitivity, self::Sensitivity>,
                {
                    let (v0, v1, v2, v3, v4, v5, v6, v7, v8, v9, v10) = self.0;
                    EventEnvelopeBuilder((v0, v1, v2, v3, v4, v5, v6, v7, v8, v9, v10, value))
                }

                /// Sets the [`sensitivity` field](EventEnvelope#structfield.sensitivity) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn sensitivity_as_default(
                    self,
                ) -> EventEnvelopeBuilder<(
                    T0,
                    T1,
                    T2,
                    T3,
                    T4,
                    T5,
                    T6,
                    T7,
                    T8,
                    T9,
                    T10,
                    ::planus::DefaultValue,
                )> {
                    self.sensitivity(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11>
                EventEnvelopeBuilder<(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11)>
            {
                /// Setter for the [`event_time_ns` field](EventEnvelope#structfield.event_time_ns).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn event_time_ns<T12>(
                    self,
                    value: T12,
                ) -> EventEnvelopeBuilder<(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12)>
                where
                    T12: ::planus::WriteAsDefault<i64, i64>,
                {
                    let (v0, v1, v2, v3, v4, v5, v6, v7, v8, v9, v10, v11) = self.0;
                    EventEnvelopeBuilder((v0, v1, v2, v3, v4, v5, v6, v7, v8, v9, v10, v11, value))
                }

                /// Sets the [`event_time_ns` field](EventEnvelope#structfield.event_time_ns) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn event_time_ns_as_default(
                    self,
                ) -> EventEnvelopeBuilder<(
                    T0,
                    T1,
                    T2,
                    T3,
                    T4,
                    T5,
                    T6,
                    T7,
                    T8,
                    T9,
                    T10,
                    T11,
                    ::planus::DefaultValue,
                )> {
                    self.event_time_ns(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12>
                EventEnvelopeBuilder<(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12)>
            {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [EventEnvelope].
                #[inline]
                pub fn finish(
                    self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<EventEnvelope>
                where
                    Self: ::planus::WriteAsOffset<EventEnvelope>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u16, u16>,
                T1: ::planus::WriteAsUnion<self::EventPayload>,
                T2: ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                T3: ::planus::WriteAsDefault<u64, u64>,
                T4: ::planus::WriteAsDefault<u32, u32>,
                T5: ::planus::WriteAsDefault<u32, u32>,
                T6: ::planus::WriteAsDefault<u16, u16>,
                T7: ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                T8: ::planus::WriteAsOptional<::planus::Offset<self::ModelProvenance>>,
                T9: ::planus::WriteAsDefault<self::Authority, self::Authority>,
                T10: ::planus::WriteAsDefault<self::Retention, self::Retention>,
                T11: ::planus::WriteAsDefault<self::Sensitivity, self::Sensitivity>,
                T12: ::planus::WriteAsDefault<i64, i64>,
            > ::planus::WriteAs<::planus::Offset<EventEnvelope>>
                for EventEnvelopeBuilder<(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12)>
            {
                type Prepared = ::planus::Offset<EventEnvelope>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<EventEnvelope> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u16, u16>,
                T1: ::planus::WriteAsUnion<self::EventPayload>,
                T2: ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                T3: ::planus::WriteAsDefault<u64, u64>,
                T4: ::planus::WriteAsDefault<u32, u32>,
                T5: ::planus::WriteAsDefault<u32, u32>,
                T6: ::planus::WriteAsDefault<u16, u16>,
                T7: ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                T8: ::planus::WriteAsOptional<::planus::Offset<self::ModelProvenance>>,
                T9: ::planus::WriteAsDefault<self::Authority, self::Authority>,
                T10: ::planus::WriteAsDefault<self::Retention, self::Retention>,
                T11: ::planus::WriteAsDefault<self::Sensitivity, self::Sensitivity>,
                T12: ::planus::WriteAsDefault<i64, i64>,
            > ::planus::WriteAsOptional<::planus::Offset<EventEnvelope>>
                for EventEnvelopeBuilder<(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12)>
            {
                type Prepared = ::planus::Offset<EventEnvelope>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<EventEnvelope>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u16, u16>,
                T1: ::planus::WriteAsUnion<self::EventPayload>,
                T2: ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                T3: ::planus::WriteAsDefault<u64, u64>,
                T4: ::planus::WriteAsDefault<u32, u32>,
                T5: ::planus::WriteAsDefault<u32, u32>,
                T6: ::planus::WriteAsDefault<u16, u16>,
                T7: ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                T8: ::planus::WriteAsOptional<::planus::Offset<self::ModelProvenance>>,
                T9: ::planus::WriteAsDefault<self::Authority, self::Authority>,
                T10: ::planus::WriteAsDefault<self::Retention, self::Retention>,
                T11: ::planus::WriteAsDefault<self::Sensitivity, self::Sensitivity>,
                T12: ::planus::WriteAsDefault<i64, i64>,
            > ::planus::WriteAsOffset<EventEnvelope>
                for EventEnvelopeBuilder<(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12)>
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<EventEnvelope> {
                    let (v0, v1, v2, v3, v4, v5, v6, v7, v8, v9, v10, v11, v12) = &self.0;
                    EventEnvelope::create(
                        builder, v0, v1, v2, v3, v4, v5, v6, v7, v8, v9, v10, v11, v12,
                    )
                }
            }

            /// Reference to a deserialized [EventEnvelope].
            #[derive(Copy, Clone)]
            pub struct EventEnvelopeRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> EventEnvelopeRef<'a> {
                /// Getter for the [`schema_version` field](EventEnvelope#structfield.schema_version).
                #[inline]
                pub fn schema_version(&self) -> ::planus::Result<u16> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(0, "EventEnvelope", "schema_version")?
                            .unwrap_or(1),
                    )
                }

                /// Getter for the [`payload` field](EventEnvelope#structfield.payload).
                #[inline]
                pub fn payload(&self) -> ::planus::Result<self::EventPayloadRef<'a>> {
                    self.0.access_union_required(1, "EventEnvelope", "payload")
                }

                /// Getter for the [`connection_id` field](EventEnvelope#structfield.connection_id).
                #[inline]
                pub fn connection_id(&self) -> ::planus::Result<::core::option::Option<&'a [u8]>> {
                    self.0.access(3, "EventEnvelope", "connection_id")
                }

                /// Getter for the [`client_seq` field](EventEnvelope#structfield.client_seq).
                #[inline]
                pub fn client_seq(&self) -> ::planus::Result<u64> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(4, "EventEnvelope", "client_seq")?
                            .unwrap_or(0),
                    )
                }

                /// Getter for the [`client_event_index` field](EventEnvelope#structfield.client_event_index).
                #[inline]
                pub fn client_event_index(&self) -> ::planus::Result<u32> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(5, "EventEnvelope", "client_event_index")?
                            .unwrap_or(0),
                    )
                }

                /// Getter for the [`client_event_count` field](EventEnvelope#structfield.client_event_count).
                #[inline]
                pub fn client_event_count(&self) -> ::planus::Result<u32> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(6, "EventEnvelope", "client_event_count")?
                            .unwrap_or(0),
                    )
                }

                /// Getter for the [`origin_actor` field](EventEnvelope#structfield.origin_actor).
                #[inline]
                pub fn origin_actor(&self) -> ::planus::Result<u16> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(7, "EventEnvelope", "origin_actor")?
                            .unwrap_or(0),
                    )
                }

                /// Getter for the [`run_id` field](EventEnvelope#structfield.run_id).
                #[inline]
                pub fn run_id(&self) -> ::planus::Result<::core::option::Option<&'a [u8]>> {
                    self.0.access(8, "EventEnvelope", "run_id")
                }

                /// Getter for the [`model_provenance` field](EventEnvelope#structfield.model_provenance).
                #[inline]
                pub fn model_provenance(
                    &self,
                ) -> ::planus::Result<::core::option::Option<self::ModelProvenanceRef<'a>>>
                {
                    self.0.access(9, "EventEnvelope", "model_provenance")
                }

                /// Getter for the [`authority` field](EventEnvelope#structfield.authority).
                #[inline]
                pub fn authority(&self) -> ::planus::Result<self::Authority> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(10, "EventEnvelope", "authority")?
                            .unwrap_or(self::Authority::UserAsserted),
                    )
                }

                /// Getter for the [`retention` field](EventEnvelope#structfield.retention).
                #[inline]
                pub fn retention(&self) -> ::planus::Result<self::Retention> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(11, "EventEnvelope", "retention")?
                            .unwrap_or(self::Retention::CurrentState),
                    )
                }

                /// Getter for the [`sensitivity` field](EventEnvelope#structfield.sensitivity).
                #[inline]
                pub fn sensitivity(&self) -> ::planus::Result<self::Sensitivity> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(12, "EventEnvelope", "sensitivity")?
                            .unwrap_or(self::Sensitivity::Public),
                    )
                }

                /// Getter for the [`event_time_ns` field](EventEnvelope#structfield.event_time_ns).
                #[inline]
                pub fn event_time_ns(&self) -> ::planus::Result<i64> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(13, "EventEnvelope", "event_time_ns")?
                            .unwrap_or(0),
                    )
                }
            }

            impl<'a> ::core::fmt::Debug for EventEnvelopeRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("EventEnvelopeRef");
                    f.field("schema_version", &self.schema_version());
                    f.field("payload", &self.payload());
                    if let ::core::option::Option::Some(field_connection_id) =
                        self.connection_id().transpose()
                    {
                        f.field("connection_id", &field_connection_id);
                    }
                    f.field("client_seq", &self.client_seq());
                    f.field("client_event_index", &self.client_event_index());
                    f.field("client_event_count", &self.client_event_count());
                    f.field("origin_actor", &self.origin_actor());
                    if let ::core::option::Option::Some(field_run_id) = self.run_id().transpose() {
                        f.field("run_id", &field_run_id);
                    }
                    if let ::core::option::Option::Some(field_model_provenance) =
                        self.model_provenance().transpose()
                    {
                        f.field("model_provenance", &field_model_provenance);
                    }
                    f.field("authority", &self.authority());
                    f.field("retention", &self.retention());
                    f.field("sensitivity", &self.sensitivity());
                    f.field("event_time_ns", &self.event_time_ns());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<EventEnvelopeRef<'a>> for EventEnvelope {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: EventEnvelopeRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        schema_version: ::core::convert::TryInto::try_into(
                            value.schema_version()?,
                        )?,
                        payload: ::core::convert::TryInto::try_into(value.payload()?)?,
                        connection_id: value.connection_id()?.map(|v| v.to_vec()),
                        client_seq: ::core::convert::TryInto::try_into(value.client_seq()?)?,
                        client_event_index: ::core::convert::TryInto::try_into(
                            value.client_event_index()?,
                        )?,
                        client_event_count: ::core::convert::TryInto::try_into(
                            value.client_event_count()?,
                        )?,
                        origin_actor: ::core::convert::TryInto::try_into(value.origin_actor()?)?,
                        run_id: value.run_id()?.map(|v| v.to_vec()),
                        model_provenance: if let ::core::option::Option::Some(model_provenance) =
                            value.model_provenance()?
                        {
                            ::core::option::Option::Some(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryInto::try_into(model_provenance)?,
                            ))
                        } else {
                            ::core::option::Option::None
                        },
                        authority: ::core::convert::TryInto::try_into(value.authority()?)?,
                        retention: ::core::convert::TryInto::try_into(value.retention()?)?,
                        sensitivity: ::core::convert::TryInto::try_into(value.sensitivity()?)?,
                        event_time_ns: ::core::convert::TryInto::try_into(value.event_time_ns()?)?,
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for EventEnvelopeRef<'a> {
                #[inline]
                fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::ErrorKind> {
                    ::core::result::Result::Ok(Self(::planus::table_reader::Table::from_buffer(
                        buffer, offset,
                    )?))
                }
            }

            impl<'a> ::planus::VectorReadInner<'a> for EventEnvelopeRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[EventEnvelopeRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<EventEnvelope>> for EventEnvelope {
                type Value = ::planus::Offset<EventEnvelope>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<EventEnvelope>],
                    bytes: *mut ::core::mem::MaybeUninit<u8>,
                    buffer_position: u32,
                ) {
                    let bytes = bytes as *mut [::core::mem::MaybeUninit<u8>; 4];
                    for (i, v) in ::core::iter::Iterator::enumerate(values.iter()) {
                        ::planus::WriteAsPrimitive::write(
                            v,
                            ::planus::Cursor::new(unsafe { &mut *bytes.add(i) }),
                            buffer_position - (Self::STRIDE * i) as u32,
                        );
                    }
                }
            }

            impl<'a> ::planus::ReadAsRoot<'a> for EventEnvelopeRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[EventEnvelopeRef]", "read_as_root", 0)
                    })
                }
            }
        }
    }
}
