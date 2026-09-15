pub use root::*;

const _: () = ::planus::check_version_compatibility("planus-1.3.0");

/// The root namespace
///
/// Generated from these locations:
/// * File `schemas/protocol.fbs`
#[no_implicit_prelude]
#[allow(clippy::needless_lifetimes)]
mod root {
    /// The namespace `hypermind`
    ///
    /// Generated from these locations:
    /// * File `schemas/protocol.fbs`
    pub mod hypermind {
        /// The namespace `hypermind.protocol`
        ///
        /// Generated from these locations:
        /// * File `schemas/protocol.fbs`
        pub mod protocol {
            /// The enum `ResponseStatus` in the namespace `hypermind.protocol`
            ///
            /// Generated from these locations:
            /// * Enum `ResponseStatus` in the file `schemas/protocol.fbs:3`
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
            pub enum ResponseStatus {
                /// The variant `ok` in the enum `ResponseStatus`
                Ok = 0,

                /// The variant `error` in the enum `ResponseStatus`
                Error = 1,
            }

            impl ResponseStatus {
                /// Array containing all valid variants of ResponseStatus
                pub const ENUM_VALUES: [Self; 2] = [Self::Ok, Self::Error];
            }

            impl ::core::convert::TryFrom<u8> for ResponseStatus {
                type Error = ::planus::errors::UnknownEnumTagKind;
                #[inline]
                fn try_from(
                    value: u8,
                ) -> ::core::result::Result<Self, ::planus::errors::UnknownEnumTagKind>
                {
                    #[allow(clippy::match_single_binding)]
                    match value {
                        0 => ::core::result::Result::Ok(ResponseStatus::Ok),
                        1 => ::core::result::Result::Ok(ResponseStatus::Error),

                        _ => ::core::result::Result::Err(::planus::errors::UnknownEnumTagKind {
                            tag: value as i128,
                        }),
                    }
                }
            }

            impl ::core::convert::From<ResponseStatus> for u8 {
                #[inline]
                fn from(value: ResponseStatus) -> Self {
                    value as u8
                }
            }

            /// # Safety
            /// The Planus compiler correctly calculates `ALIGNMENT` and `SIZE`.
            unsafe impl ::planus::Primitive for ResponseStatus {
                const ALIGNMENT: usize = 1;
                const SIZE: usize = 1;
            }

            impl ::planus::WriteAsPrimitive<ResponseStatus> for ResponseStatus {
                #[inline]
                fn write<const N: usize>(
                    &self,
                    cursor: ::planus::Cursor<'_, N>,
                    buffer_position: u32,
                ) {
                    (*self as u8).write(cursor, buffer_position);
                }
            }

            impl ::planus::WriteAs<ResponseStatus> for ResponseStatus {
                type Prepared = Self;

                #[inline]
                fn prepare(&self, _builder: &mut ::planus::Builder) -> ResponseStatus {
                    *self
                }
            }

            impl ::planus::WriteAsDefault<ResponseStatus, ResponseStatus> for ResponseStatus {
                type Prepared = Self;

                #[inline]
                fn prepare(
                    &self,
                    _builder: &mut ::planus::Builder,
                    default: &ResponseStatus,
                ) -> ::core::option::Option<ResponseStatus> {
                    if self == default {
                        ::core::option::Option::None
                    } else {
                        ::core::option::Option::Some(*self)
                    }
                }
            }

            impl ::planus::WriteAsOptional<ResponseStatus> for ResponseStatus {
                type Prepared = Self;

                #[inline]
                fn prepare(
                    &self,
                    _builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<ResponseStatus> {
                    ::core::option::Option::Some(*self)
                }
            }

            impl<'buf> ::planus::TableRead<'buf> for ResponseStatus {
                #[inline]
                fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'buf>,
                    offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::ErrorKind> {
                    let n: u8 = ::planus::TableRead::from_buffer(buffer, offset)?;
                    ::core::result::Result::Ok(::core::convert::TryInto::try_into(n)?)
                }
            }

            impl<'buf> ::planus::VectorReadInner<'buf> for ResponseStatus {
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
                            "ResponseStatus",
                            "VectorRead::from_buffer",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<ResponseStatus> for ResponseStatus {
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

            /// The table `Hello` in the namespace `hypermind.protocol`
            ///
            /// Generated from these locations:
            /// * Table `Hello` in the file `schemas/protocol.fbs:5`
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
            pub struct Hello {
                /// The field `proto_version` in the table `Hello`
                pub proto_version: u16,
                /// The field `connection_id` in the table `Hello`
                pub connection_id: ::planus::alloc::vec::Vec<u8>,
                /// The field `capability_token` in the table `Hello`
                pub capability_token: ::planus::alloc::vec::Vec<u8>,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for Hello {
                fn default() -> Self {
                    Self {
                        proto_version: 0,
                        connection_id: ::core::default::Default::default(),
                        capability_token: ::core::default::Default::default(),
                    }
                }
            }

            impl Hello {
                /// Creates a [HelloBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> HelloBuilder<()> {
                    HelloBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_proto_version: impl ::planus::WriteAsDefault<u16, u16>,
                    field_connection_id: impl ::planus::WriteAs<::planus::Offset<[u8]>>,
                    field_capability_token: impl ::planus::WriteAs<::planus::Offset<[u8]>>,
                ) -> ::planus::Offset<Self> {
                    let prepared_proto_version = field_proto_version.prepare(builder, &0);
                    let prepared_connection_id = field_connection_id.prepare(builder);
                    let prepared_capability_token = field_capability_token.prepare(builder);

                    let mut table_writer: ::planus::table_writer::TableWriter<10> =
                        ::core::default::Default::default();
                    table_writer.write_entry::<::planus::Offset<[u8]>>(1);
                    table_writer.write_entry::<::planus::Offset<[u8]>>(2);
                    if prepared_proto_version.is_some() {
                        table_writer.write_entry::<u16>(0);
                    }

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            object_writer.write::<_, _, 4>(&prepared_connection_id);
                            object_writer.write::<_, _, 4>(&prepared_capability_token);
                            if let ::core::option::Option::Some(prepared_proto_version) =
                                prepared_proto_version
                            {
                                object_writer.write::<_, _, 2>(&prepared_proto_version);
                            }
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<Hello>> for Hello {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Hello> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<Hello>> for Hello {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Hello>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<Hello> for Hello {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Hello> {
                    Hello::create(
                        builder,
                        self.proto_version,
                        &self.connection_id,
                        &self.capability_token,
                    )
                }
            }

            /// Builder for serializing an instance of the [Hello] type.
            ///
            /// Can be created using the [Hello::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct HelloBuilder<State>(State);

            impl HelloBuilder<()> {
                /// Setter for the [`proto_version` field](Hello#structfield.proto_version).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn proto_version<T0>(self, value: T0) -> HelloBuilder<(T0,)>
                where
                    T0: ::planus::WriteAsDefault<u16, u16>,
                {
                    HelloBuilder((value,))
                }

                /// Sets the [`proto_version` field](Hello#structfield.proto_version) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn proto_version_as_default(self) -> HelloBuilder<(::planus::DefaultValue,)> {
                    self.proto_version(::planus::DefaultValue)
                }
            }

            impl<T0> HelloBuilder<(T0,)> {
                /// Setter for the [`connection_id` field](Hello#structfield.connection_id).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn connection_id<T1>(self, value: T1) -> HelloBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAs<::planus::Offset<[u8]>>,
                {
                    let (v0,) = self.0;
                    HelloBuilder((v0, value))
                }
            }

            impl<T0, T1> HelloBuilder<(T0, T1)> {
                /// Setter for the [`capability_token` field](Hello#structfield.capability_token).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn capability_token<T2>(self, value: T2) -> HelloBuilder<(T0, T1, T2)>
                where
                    T2: ::planus::WriteAs<::planus::Offset<[u8]>>,
                {
                    let (v0, v1) = self.0;
                    HelloBuilder((v0, v1, value))
                }
            }

            impl<T0, T1, T2> HelloBuilder<(T0, T1, T2)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [Hello].
                #[inline]
                pub fn finish(self, builder: &mut ::planus::Builder) -> ::planus::Offset<Hello>
                where
                    Self: ::planus::WriteAsOffset<Hello>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u16, u16>,
                T1: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T2: ::planus::WriteAs<::planus::Offset<[u8]>>,
            > ::planus::WriteAs<::planus::Offset<Hello>> for HelloBuilder<(T0, T1, T2)>
            {
                type Prepared = ::planus::Offset<Hello>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Hello> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u16, u16>,
                T1: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T2: ::planus::WriteAs<::planus::Offset<[u8]>>,
            > ::planus::WriteAsOptional<::planus::Offset<Hello>> for HelloBuilder<(T0, T1, T2)>
            {
                type Prepared = ::planus::Offset<Hello>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Hello>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u16, u16>,
                T1: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T2: ::planus::WriteAs<::planus::Offset<[u8]>>,
            > ::planus::WriteAsOffset<Hello> for HelloBuilder<(T0, T1, T2)>
            {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Hello> {
                    let (v0, v1, v2) = &self.0;
                    Hello::create(builder, v0, v1, v2)
                }
            }

            /// Reference to a deserialized [Hello].
            #[derive(Copy, Clone)]
            pub struct HelloRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> HelloRef<'a> {
                /// Getter for the [`proto_version` field](Hello#structfield.proto_version).
                #[inline]
                pub fn proto_version(&self) -> ::planus::Result<u16> {
                    ::core::result::Result::Ok(
                        self.0.access(0, "Hello", "proto_version")?.unwrap_or(0),
                    )
                }

                /// Getter for the [`connection_id` field](Hello#structfield.connection_id).
                #[inline]
                pub fn connection_id(&self) -> ::planus::Result<&'a [u8]> {
                    self.0.access_required(1, "Hello", "connection_id")
                }

                /// Getter for the [`capability_token` field](Hello#structfield.capability_token).
                #[inline]
                pub fn capability_token(&self) -> ::planus::Result<&'a [u8]> {
                    self.0.access_required(2, "Hello", "capability_token")
                }
            }

            impl<'a> ::core::fmt::Debug for HelloRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("HelloRef");
                    f.field("proto_version", &self.proto_version());
                    f.field("connection_id", &self.connection_id());
                    f.field("capability_token", &self.capability_token());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<HelloRef<'a>> for Hello {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: HelloRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        proto_version: ::core::convert::TryInto::try_into(value.proto_version()?)?,
                        connection_id: value.connection_id()?.to_vec(),
                        capability_token: value.capability_token()?.to_vec(),
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for HelloRef<'a> {
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

            impl<'a> ::planus::VectorReadInner<'a> for HelloRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[HelloRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<Hello>> for Hello {
                type Value = ::planus::Offset<Hello>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<Hello>],
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

            impl<'a> ::planus::ReadAsRoot<'a> for HelloRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[HelloRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `Welcome` in the namespace `hypermind.protocol`
            ///
            /// Generated from these locations:
            /// * Table `Welcome` in the file `schemas/protocol.fbs:11`
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
            pub struct Welcome {
                /// The field `proto_version` in the table `Welcome`
                pub proto_version: u16,
                /// The field `actor_ns` in the table `Welcome`
                pub actor_ns: u16,
                /// The field `admin` in the table `Welcome`
                pub admin: bool,
                /// The field `maximum_frame_bytes` in the table `Welcome`
                pub maximum_frame_bytes: u32,
                /// The field `maximum_batch_events` in the table `Welcome`
                pub maximum_batch_events: u32,
                /// The field `maximum_subscriptions` in the table `Welcome`
                pub maximum_subscriptions: u32,
                /// The field `next_client_seq` in the table `Welcome`
                pub next_client_seq: u64,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for Welcome {
                fn default() -> Self {
                    Self {
                        proto_version: 0,
                        actor_ns: 0,
                        admin: false,
                        maximum_frame_bytes: 0,
                        maximum_batch_events: 0,
                        maximum_subscriptions: 0,
                        next_client_seq: 0,
                    }
                }
            }

            impl Welcome {
                /// Creates a [WelcomeBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> WelcomeBuilder<()> {
                    WelcomeBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_proto_version: impl ::planus::WriteAsDefault<u16, u16>,
                    field_actor_ns: impl ::planus::WriteAsDefault<u16, u16>,
                    field_admin: impl ::planus::WriteAsDefault<bool, bool>,
                    field_maximum_frame_bytes: impl ::planus::WriteAsDefault<u32, u32>,
                    field_maximum_batch_events: impl ::planus::WriteAsDefault<u32, u32>,
                    field_maximum_subscriptions: impl ::planus::WriteAsDefault<u32, u32>,
                    field_next_client_seq: impl ::planus::WriteAsDefault<u64, u64>,
                ) -> ::planus::Offset<Self> {
                    let prepared_proto_version = field_proto_version.prepare(builder, &0);
                    let prepared_actor_ns = field_actor_ns.prepare(builder, &0);
                    let prepared_admin = field_admin.prepare(builder, &false);
                    let prepared_maximum_frame_bytes =
                        field_maximum_frame_bytes.prepare(builder, &0);
                    let prepared_maximum_batch_events =
                        field_maximum_batch_events.prepare(builder, &0);
                    let prepared_maximum_subscriptions =
                        field_maximum_subscriptions.prepare(builder, &0);
                    let prepared_next_client_seq = field_next_client_seq.prepare(builder, &0);

                    let mut table_writer: ::planus::table_writer::TableWriter<18> =
                        ::core::default::Default::default();
                    if prepared_next_client_seq.is_some() {
                        table_writer.write_entry::<u64>(6);
                    }
                    if prepared_maximum_frame_bytes.is_some() {
                        table_writer.write_entry::<u32>(3);
                    }
                    if prepared_maximum_batch_events.is_some() {
                        table_writer.write_entry::<u32>(4);
                    }
                    if prepared_maximum_subscriptions.is_some() {
                        table_writer.write_entry::<u32>(5);
                    }
                    if prepared_proto_version.is_some() {
                        table_writer.write_entry::<u16>(0);
                    }
                    if prepared_actor_ns.is_some() {
                        table_writer.write_entry::<u16>(1);
                    }
                    if prepared_admin.is_some() {
                        table_writer.write_entry::<bool>(2);
                    }

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            if let ::core::option::Option::Some(prepared_next_client_seq) =
                                prepared_next_client_seq
                            {
                                object_writer.write::<_, _, 8>(&prepared_next_client_seq);
                            }
                            if let ::core::option::Option::Some(prepared_maximum_frame_bytes) =
                                prepared_maximum_frame_bytes
                            {
                                object_writer.write::<_, _, 4>(&prepared_maximum_frame_bytes);
                            }
                            if let ::core::option::Option::Some(prepared_maximum_batch_events) =
                                prepared_maximum_batch_events
                            {
                                object_writer.write::<_, _, 4>(&prepared_maximum_batch_events);
                            }
                            if let ::core::option::Option::Some(prepared_maximum_subscriptions) =
                                prepared_maximum_subscriptions
                            {
                                object_writer.write::<_, _, 4>(&prepared_maximum_subscriptions);
                            }
                            if let ::core::option::Option::Some(prepared_proto_version) =
                                prepared_proto_version
                            {
                                object_writer.write::<_, _, 2>(&prepared_proto_version);
                            }
                            if let ::core::option::Option::Some(prepared_actor_ns) =
                                prepared_actor_ns
                            {
                                object_writer.write::<_, _, 2>(&prepared_actor_ns);
                            }
                            if let ::core::option::Option::Some(prepared_admin) = prepared_admin {
                                object_writer.write::<_, _, 1>(&prepared_admin);
                            }
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<Welcome>> for Welcome {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Welcome> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<Welcome>> for Welcome {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Welcome>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<Welcome> for Welcome {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Welcome> {
                    Welcome::create(
                        builder,
                        self.proto_version,
                        self.actor_ns,
                        self.admin,
                        self.maximum_frame_bytes,
                        self.maximum_batch_events,
                        self.maximum_subscriptions,
                        self.next_client_seq,
                    )
                }
            }

            /// Builder for serializing an instance of the [Welcome] type.
            ///
            /// Can be created using the [Welcome::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct WelcomeBuilder<State>(State);

            impl WelcomeBuilder<()> {
                /// Setter for the [`proto_version` field](Welcome#structfield.proto_version).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn proto_version<T0>(self, value: T0) -> WelcomeBuilder<(T0,)>
                where
                    T0: ::planus::WriteAsDefault<u16, u16>,
                {
                    WelcomeBuilder((value,))
                }

                /// Sets the [`proto_version` field](Welcome#structfield.proto_version) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn proto_version_as_default(self) -> WelcomeBuilder<(::planus::DefaultValue,)> {
                    self.proto_version(::planus::DefaultValue)
                }
            }

            impl<T0> WelcomeBuilder<(T0,)> {
                /// Setter for the [`actor_ns` field](Welcome#structfield.actor_ns).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn actor_ns<T1>(self, value: T1) -> WelcomeBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAsDefault<u16, u16>,
                {
                    let (v0,) = self.0;
                    WelcomeBuilder((v0, value))
                }

                /// Sets the [`actor_ns` field](Welcome#structfield.actor_ns) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn actor_ns_as_default(self) -> WelcomeBuilder<(T0, ::planus::DefaultValue)> {
                    self.actor_ns(::planus::DefaultValue)
                }
            }

            impl<T0, T1> WelcomeBuilder<(T0, T1)> {
                /// Setter for the [`admin` field](Welcome#structfield.admin).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn admin<T2>(self, value: T2) -> WelcomeBuilder<(T0, T1, T2)>
                where
                    T2: ::planus::WriteAsDefault<bool, bool>,
                {
                    let (v0, v1) = self.0;
                    WelcomeBuilder((v0, v1, value))
                }

                /// Sets the [`admin` field](Welcome#structfield.admin) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn admin_as_default(self) -> WelcomeBuilder<(T0, T1, ::planus::DefaultValue)> {
                    self.admin(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2> WelcomeBuilder<(T0, T1, T2)> {
                /// Setter for the [`maximum_frame_bytes` field](Welcome#structfield.maximum_frame_bytes).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn maximum_frame_bytes<T3>(self, value: T3) -> WelcomeBuilder<(T0, T1, T2, T3)>
                where
                    T3: ::planus::WriteAsDefault<u32, u32>,
                {
                    let (v0, v1, v2) = self.0;
                    WelcomeBuilder((v0, v1, v2, value))
                }

                /// Sets the [`maximum_frame_bytes` field](Welcome#structfield.maximum_frame_bytes) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn maximum_frame_bytes_as_default(
                    self,
                ) -> WelcomeBuilder<(T0, T1, T2, ::planus::DefaultValue)> {
                    self.maximum_frame_bytes(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2, T3> WelcomeBuilder<(T0, T1, T2, T3)> {
                /// Setter for the [`maximum_batch_events` field](Welcome#structfield.maximum_batch_events).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn maximum_batch_events<T4>(
                    self,
                    value: T4,
                ) -> WelcomeBuilder<(T0, T1, T2, T3, T4)>
                where
                    T4: ::planus::WriteAsDefault<u32, u32>,
                {
                    let (v0, v1, v2, v3) = self.0;
                    WelcomeBuilder((v0, v1, v2, v3, value))
                }

                /// Sets the [`maximum_batch_events` field](Welcome#structfield.maximum_batch_events) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn maximum_batch_events_as_default(
                    self,
                ) -> WelcomeBuilder<(T0, T1, T2, T3, ::planus::DefaultValue)> {
                    self.maximum_batch_events(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2, T3, T4> WelcomeBuilder<(T0, T1, T2, T3, T4)> {
                /// Setter for the [`maximum_subscriptions` field](Welcome#structfield.maximum_subscriptions).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn maximum_subscriptions<T5>(
                    self,
                    value: T5,
                ) -> WelcomeBuilder<(T0, T1, T2, T3, T4, T5)>
                where
                    T5: ::planus::WriteAsDefault<u32, u32>,
                {
                    let (v0, v1, v2, v3, v4) = self.0;
                    WelcomeBuilder((v0, v1, v2, v3, v4, value))
                }

                /// Sets the [`maximum_subscriptions` field](Welcome#structfield.maximum_subscriptions) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn maximum_subscriptions_as_default(
                    self,
                ) -> WelcomeBuilder<(T0, T1, T2, T3, T4, ::planus::DefaultValue)> {
                    self.maximum_subscriptions(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2, T3, T4, T5> WelcomeBuilder<(T0, T1, T2, T3, T4, T5)> {
                /// Setter for the [`next_client_seq` field](Welcome#structfield.next_client_seq).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn next_client_seq<T6>(
                    self,
                    value: T6,
                ) -> WelcomeBuilder<(T0, T1, T2, T3, T4, T5, T6)>
                where
                    T6: ::planus::WriteAsDefault<u64, u64>,
                {
                    let (v0, v1, v2, v3, v4, v5) = self.0;
                    WelcomeBuilder((v0, v1, v2, v3, v4, v5, value))
                }

                /// Sets the [`next_client_seq` field](Welcome#structfield.next_client_seq) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn next_client_seq_as_default(
                    self,
                ) -> WelcomeBuilder<(T0, T1, T2, T3, T4, T5, ::planus::DefaultValue)>
                {
                    self.next_client_seq(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2, T3, T4, T5, T6> WelcomeBuilder<(T0, T1, T2, T3, T4, T5, T6)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [Welcome].
                #[inline]
                pub fn finish(self, builder: &mut ::planus::Builder) -> ::planus::Offset<Welcome>
                where
                    Self: ::planus::WriteAsOffset<Welcome>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u16, u16>,
                T1: ::planus::WriteAsDefault<u16, u16>,
                T2: ::planus::WriteAsDefault<bool, bool>,
                T3: ::planus::WriteAsDefault<u32, u32>,
                T4: ::planus::WriteAsDefault<u32, u32>,
                T5: ::planus::WriteAsDefault<u32, u32>,
                T6: ::planus::WriteAsDefault<u64, u64>,
            > ::planus::WriteAs<::planus::Offset<Welcome>>
                for WelcomeBuilder<(T0, T1, T2, T3, T4, T5, T6)>
            {
                type Prepared = ::planus::Offset<Welcome>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Welcome> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u16, u16>,
                T1: ::planus::WriteAsDefault<u16, u16>,
                T2: ::planus::WriteAsDefault<bool, bool>,
                T3: ::planus::WriteAsDefault<u32, u32>,
                T4: ::planus::WriteAsDefault<u32, u32>,
                T5: ::planus::WriteAsDefault<u32, u32>,
                T6: ::planus::WriteAsDefault<u64, u64>,
            > ::planus::WriteAsOptional<::planus::Offset<Welcome>>
                for WelcomeBuilder<(T0, T1, T2, T3, T4, T5, T6)>
            {
                type Prepared = ::planus::Offset<Welcome>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Welcome>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u16, u16>,
                T1: ::planus::WriteAsDefault<u16, u16>,
                T2: ::planus::WriteAsDefault<bool, bool>,
                T3: ::planus::WriteAsDefault<u32, u32>,
                T4: ::planus::WriteAsDefault<u32, u32>,
                T5: ::planus::WriteAsDefault<u32, u32>,
                T6: ::planus::WriteAsDefault<u64, u64>,
            > ::planus::WriteAsOffset<Welcome> for WelcomeBuilder<(T0, T1, T2, T3, T4, T5, T6)>
            {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Welcome> {
                    let (v0, v1, v2, v3, v4, v5, v6) = &self.0;
                    Welcome::create(builder, v0, v1, v2, v3, v4, v5, v6)
                }
            }

            /// Reference to a deserialized [Welcome].
            #[derive(Copy, Clone)]
            pub struct WelcomeRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> WelcomeRef<'a> {
                /// Getter for the [`proto_version` field](Welcome#structfield.proto_version).
                #[inline]
                pub fn proto_version(&self) -> ::planus::Result<u16> {
                    ::core::result::Result::Ok(
                        self.0.access(0, "Welcome", "proto_version")?.unwrap_or(0),
                    )
                }

                /// Getter for the [`actor_ns` field](Welcome#structfield.actor_ns).
                #[inline]
                pub fn actor_ns(&self) -> ::planus::Result<u16> {
                    ::core::result::Result::Ok(
                        self.0.access(1, "Welcome", "actor_ns")?.unwrap_or(0),
                    )
                }

                /// Getter for the [`admin` field](Welcome#structfield.admin).
                #[inline]
                pub fn admin(&self) -> ::planus::Result<bool> {
                    ::core::result::Result::Ok(
                        self.0.access(2, "Welcome", "admin")?.unwrap_or(false),
                    )
                }

                /// Getter for the [`maximum_frame_bytes` field](Welcome#structfield.maximum_frame_bytes).
                #[inline]
                pub fn maximum_frame_bytes(&self) -> ::planus::Result<u32> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(3, "Welcome", "maximum_frame_bytes")?
                            .unwrap_or(0),
                    )
                }

                /// Getter for the [`maximum_batch_events` field](Welcome#structfield.maximum_batch_events).
                #[inline]
                pub fn maximum_batch_events(&self) -> ::planus::Result<u32> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(4, "Welcome", "maximum_batch_events")?
                            .unwrap_or(0),
                    )
                }

                /// Getter for the [`maximum_subscriptions` field](Welcome#structfield.maximum_subscriptions).
                #[inline]
                pub fn maximum_subscriptions(&self) -> ::planus::Result<u32> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(5, "Welcome", "maximum_subscriptions")?
                            .unwrap_or(0),
                    )
                }

                /// Getter for the [`next_client_seq` field](Welcome#structfield.next_client_seq).
                #[inline]
                pub fn next_client_seq(&self) -> ::planus::Result<u64> {
                    ::core::result::Result::Ok(
                        self.0.access(6, "Welcome", "next_client_seq")?.unwrap_or(0),
                    )
                }
            }

            impl<'a> ::core::fmt::Debug for WelcomeRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("WelcomeRef");
                    f.field("proto_version", &self.proto_version());
                    f.field("actor_ns", &self.actor_ns());
                    f.field("admin", &self.admin());
                    f.field("maximum_frame_bytes", &self.maximum_frame_bytes());
                    f.field("maximum_batch_events", &self.maximum_batch_events());
                    f.field("maximum_subscriptions", &self.maximum_subscriptions());
                    f.field("next_client_seq", &self.next_client_seq());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<WelcomeRef<'a>> for Welcome {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: WelcomeRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        proto_version: ::core::convert::TryInto::try_into(value.proto_version()?)?,
                        actor_ns: ::core::convert::TryInto::try_into(value.actor_ns()?)?,
                        admin: ::core::convert::TryInto::try_into(value.admin()?)?,
                        maximum_frame_bytes: ::core::convert::TryInto::try_into(
                            value.maximum_frame_bytes()?,
                        )?,
                        maximum_batch_events: ::core::convert::TryInto::try_into(
                            value.maximum_batch_events()?,
                        )?,
                        maximum_subscriptions: ::core::convert::TryInto::try_into(
                            value.maximum_subscriptions()?,
                        )?,
                        next_client_seq: ::core::convert::TryInto::try_into(
                            value.next_client_seq()?,
                        )?,
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for WelcomeRef<'a> {
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

            impl<'a> ::planus::VectorReadInner<'a> for WelcomeRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[WelcomeRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<Welcome>> for Welcome {
                type Value = ::planus::Offset<Welcome>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<Welcome>],
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

            impl<'a> ::planus::ReadAsRoot<'a> for WelcomeRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[WelcomeRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `AppendEvent` in the namespace `hypermind.protocol`
            ///
            /// Generated from these locations:
            /// * Table `AppendEvent` in the file `schemas/protocol.fbs:21`
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
            pub struct AppendEvent {
                /// The field `kind` in the table `AppendEvent`
                pub kind: u8,
                /// The field `conversation` in the table `AppendEvent`
                pub conversation: ::planus::alloc::vec::Vec<u8>,
                /// The field `payload` in the table `AppendEvent`
                pub payload: ::planus::alloc::vec::Vec<u8>,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for AppendEvent {
                fn default() -> Self {
                    Self {
                        kind: 0,
                        conversation: ::core::default::Default::default(),
                        payload: ::core::default::Default::default(),
                    }
                }
            }

            impl AppendEvent {
                /// Creates a [AppendEventBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> AppendEventBuilder<()> {
                    AppendEventBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_kind: impl ::planus::WriteAsDefault<u8, u8>,
                    field_conversation: impl ::planus::WriteAs<::planus::Offset<[u8]>>,
                    field_payload: impl ::planus::WriteAs<::planus::Offset<[u8]>>,
                ) -> ::planus::Offset<Self> {
                    let prepared_kind = field_kind.prepare(builder, &0);
                    let prepared_conversation = field_conversation.prepare(builder);
                    let prepared_payload = field_payload.prepare(builder);

                    let mut table_writer: ::planus::table_writer::TableWriter<10> =
                        ::core::default::Default::default();
                    table_writer.write_entry::<::planus::Offset<[u8]>>(1);
                    table_writer.write_entry::<::planus::Offset<[u8]>>(2);
                    if prepared_kind.is_some() {
                        table_writer.write_entry::<u8>(0);
                    }

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            object_writer.write::<_, _, 4>(&prepared_conversation);
                            object_writer.write::<_, _, 4>(&prepared_payload);
                            if let ::core::option::Option::Some(prepared_kind) = prepared_kind {
                                object_writer.write::<_, _, 1>(&prepared_kind);
                            }
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<AppendEvent>> for AppendEvent {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<AppendEvent> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<AppendEvent>> for AppendEvent {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<AppendEvent>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<AppendEvent> for AppendEvent {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<AppendEvent> {
                    AppendEvent::create(builder, self.kind, &self.conversation, &self.payload)
                }
            }

            /// Builder for serializing an instance of the [AppendEvent] type.
            ///
            /// Can be created using the [AppendEvent::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct AppendEventBuilder<State>(State);

            impl AppendEventBuilder<()> {
                /// Setter for the [`kind` field](AppendEvent#structfield.kind).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn kind<T0>(self, value: T0) -> AppendEventBuilder<(T0,)>
                where
                    T0: ::planus::WriteAsDefault<u8, u8>,
                {
                    AppendEventBuilder((value,))
                }

                /// Sets the [`kind` field](AppendEvent#structfield.kind) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn kind_as_default(self) -> AppendEventBuilder<(::planus::DefaultValue,)> {
                    self.kind(::planus::DefaultValue)
                }
            }

            impl<T0> AppendEventBuilder<(T0,)> {
                /// Setter for the [`conversation` field](AppendEvent#structfield.conversation).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn conversation<T1>(self, value: T1) -> AppendEventBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAs<::planus::Offset<[u8]>>,
                {
                    let (v0,) = self.0;
                    AppendEventBuilder((v0, value))
                }
            }

            impl<T0, T1> AppendEventBuilder<(T0, T1)> {
                /// Setter for the [`payload` field](AppendEvent#structfield.payload).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn payload<T2>(self, value: T2) -> AppendEventBuilder<(T0, T1, T2)>
                where
                    T2: ::planus::WriteAs<::planus::Offset<[u8]>>,
                {
                    let (v0, v1) = self.0;
                    AppendEventBuilder((v0, v1, value))
                }
            }

            impl<T0, T1, T2> AppendEventBuilder<(T0, T1, T2)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [AppendEvent].
                #[inline]
                pub fn finish(
                    self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<AppendEvent>
                where
                    Self: ::planus::WriteAsOffset<AppendEvent>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u8, u8>,
                T1: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T2: ::planus::WriteAs<::planus::Offset<[u8]>>,
            > ::planus::WriteAs<::planus::Offset<AppendEvent>>
                for AppendEventBuilder<(T0, T1, T2)>
            {
                type Prepared = ::planus::Offset<AppendEvent>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<AppendEvent> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u8, u8>,
                T1: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T2: ::planus::WriteAs<::planus::Offset<[u8]>>,
            > ::planus::WriteAsOptional<::planus::Offset<AppendEvent>>
                for AppendEventBuilder<(T0, T1, T2)>
            {
                type Prepared = ::planus::Offset<AppendEvent>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<AppendEvent>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u8, u8>,
                T1: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T2: ::planus::WriteAs<::planus::Offset<[u8]>>,
            > ::planus::WriteAsOffset<AppendEvent> for AppendEventBuilder<(T0, T1, T2)>
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<AppendEvent> {
                    let (v0, v1, v2) = &self.0;
                    AppendEvent::create(builder, v0, v1, v2)
                }
            }

            /// Reference to a deserialized [AppendEvent].
            #[derive(Copy, Clone)]
            pub struct AppendEventRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> AppendEventRef<'a> {
                /// Getter for the [`kind` field](AppendEvent#structfield.kind).
                #[inline]
                pub fn kind(&self) -> ::planus::Result<u8> {
                    ::core::result::Result::Ok(
                        self.0.access(0, "AppendEvent", "kind")?.unwrap_or(0),
                    )
                }

                /// Getter for the [`conversation` field](AppendEvent#structfield.conversation).
                #[inline]
                pub fn conversation(&self) -> ::planus::Result<&'a [u8]> {
                    self.0.access_required(1, "AppendEvent", "conversation")
                }

                /// Getter for the [`payload` field](AppendEvent#structfield.payload).
                #[inline]
                pub fn payload(&self) -> ::planus::Result<&'a [u8]> {
                    self.0.access_required(2, "AppendEvent", "payload")
                }
            }

            impl<'a> ::core::fmt::Debug for AppendEventRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("AppendEventRef");
                    f.field("kind", &self.kind());
                    f.field("conversation", &self.conversation());
                    f.field("payload", &self.payload());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<AppendEventRef<'a>> for AppendEvent {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: AppendEventRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        kind: ::core::convert::TryInto::try_into(value.kind()?)?,
                        conversation: value.conversation()?.to_vec(),
                        payload: value.payload()?.to_vec(),
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for AppendEventRef<'a> {
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

            impl<'a> ::planus::VectorReadInner<'a> for AppendEventRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[AppendEventRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<AppendEvent>> for AppendEvent {
                type Value = ::planus::Offset<AppendEvent>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<AppendEvent>],
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

            impl<'a> ::planus::ReadAsRoot<'a> for AppendEventRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[AppendEventRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `Append` in the namespace `hypermind.protocol`
            ///
            /// Generated from these locations:
            /// * Table `Append` in the file `schemas/protocol.fbs:27`
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
            pub struct Append {
                /// The field `client_seq` in the table `Append`
                pub client_seq: u64,
                /// The field `events` in the table `Append`
                pub events: ::planus::alloc::vec::Vec<self::AppendEvent>,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for Append {
                fn default() -> Self {
                    Self {
                        client_seq: 0,
                        events: ::core::default::Default::default(),
                    }
                }
            }

            impl Append {
                /// Creates a [AppendBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> AppendBuilder<()> {
                    AppendBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_client_seq: impl ::planus::WriteAsDefault<u64, u64>,
                    field_events: impl ::planus::WriteAs<
                        ::planus::Offset<[::planus::Offset<self::AppendEvent>]>,
                    >,
                ) -> ::planus::Offset<Self> {
                    let prepared_client_seq = field_client_seq.prepare(builder, &0);
                    let prepared_events = field_events.prepare(builder);

                    let mut table_writer: ::planus::table_writer::TableWriter<8> =
                        ::core::default::Default::default();
                    if prepared_client_seq.is_some() {
                        table_writer.write_entry::<u64>(0);
                    }
                    table_writer
                        .write_entry::<::planus::Offset<[::planus::Offset<self::AppendEvent>]>>(1);

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            if let ::core::option::Option::Some(prepared_client_seq) =
                                prepared_client_seq
                            {
                                object_writer.write::<_, _, 8>(&prepared_client_seq);
                            }
                            object_writer.write::<_, _, 4>(&prepared_events);
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<Append>> for Append {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Append> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<Append>> for Append {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Append>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<Append> for Append {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Append> {
                    Append::create(builder, self.client_seq, &self.events)
                }
            }

            /// Builder for serializing an instance of the [Append] type.
            ///
            /// Can be created using the [Append::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct AppendBuilder<State>(State);

            impl AppendBuilder<()> {
                /// Setter for the [`client_seq` field](Append#structfield.client_seq).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn client_seq<T0>(self, value: T0) -> AppendBuilder<(T0,)>
                where
                    T0: ::planus::WriteAsDefault<u64, u64>,
                {
                    AppendBuilder((value,))
                }

                /// Sets the [`client_seq` field](Append#structfield.client_seq) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn client_seq_as_default(self) -> AppendBuilder<(::planus::DefaultValue,)> {
                    self.client_seq(::planus::DefaultValue)
                }
            }

            impl<T0> AppendBuilder<(T0,)> {
                /// Setter for the [`events` field](Append#structfield.events).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn events<T1>(self, value: T1) -> AppendBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAs<::planus::Offset<[::planus::Offset<self::AppendEvent>]>>,
                {
                    let (v0,) = self.0;
                    AppendBuilder((v0, value))
                }
            }

            impl<T0, T1> AppendBuilder<(T0, T1)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [Append].
                #[inline]
                pub fn finish(self, builder: &mut ::planus::Builder) -> ::planus::Offset<Append>
                where
                    Self: ::planus::WriteAsOffset<Append>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u64, u64>,
                T1: ::planus::WriteAs<::planus::Offset<[::planus::Offset<self::AppendEvent>]>>,
            > ::planus::WriteAs<::planus::Offset<Append>> for AppendBuilder<(T0, T1)>
            {
                type Prepared = ::planus::Offset<Append>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Append> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u64, u64>,
                T1: ::planus::WriteAs<::planus::Offset<[::planus::Offset<self::AppendEvent>]>>,
            > ::planus::WriteAsOptional<::planus::Offset<Append>> for AppendBuilder<(T0, T1)>
            {
                type Prepared = ::planus::Offset<Append>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Append>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u64, u64>,
                T1: ::planus::WriteAs<::planus::Offset<[::planus::Offset<self::AppendEvent>]>>,
            > ::planus::WriteAsOffset<Append> for AppendBuilder<(T0, T1)>
            {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Append> {
                    let (v0, v1) = &self.0;
                    Append::create(builder, v0, v1)
                }
            }

            /// Reference to a deserialized [Append].
            #[derive(Copy, Clone)]
            pub struct AppendRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> AppendRef<'a> {
                /// Getter for the [`client_seq` field](Append#structfield.client_seq).
                #[inline]
                pub fn client_seq(&self) -> ::planus::Result<u64> {
                    ::core::result::Result::Ok(
                        self.0.access(0, "Append", "client_seq")?.unwrap_or(0),
                    )
                }

                /// Getter for the [`events` field](Append#structfield.events).
                #[inline]
                pub fn events(
                    &self,
                ) -> ::planus::Result<
                    ::planus::Vector<'a, ::planus::Result<self::AppendEventRef<'a>>>,
                > {
                    self.0.access_required(1, "Append", "events")
                }
            }

            impl<'a> ::core::fmt::Debug for AppendRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("AppendRef");
                    f.field("client_seq", &self.client_seq());
                    f.field("events", &self.events());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<AppendRef<'a>> for Append {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: AppendRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        client_seq: ::core::convert::TryInto::try_into(value.client_seq()?)?,
                        events: value.events()?.to_vec_result()?,
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for AppendRef<'a> {
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

            impl<'a> ::planus::VectorReadInner<'a> for AppendRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[AppendRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<Append>> for Append {
                type Value = ::planus::Offset<Append>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<Append>],
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

            impl<'a> ::planus::ReadAsRoot<'a> for AppendRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[AppendRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `Activate` in the namespace `hypermind.protocol`
            ///
            /// Generated from these locations:
            /// * Table `Activate` in the file `schemas/protocol.fbs:32`
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
            pub struct Activate {
                /// The field `conversation` in the table `Activate`
                pub conversation: ::planus::alloc::vec::Vec<u8>,
                /// The field `query` in the table `Activate`
                pub query: ::planus::alloc::vec::Vec<u8>,
                /// The field `turn_text` in the table `Activate`
                pub turn_text: ::core::option::Option<::planus::alloc::vec::Vec<u8>>,
                /// The field `budget_tokens` in the table `Activate`
                pub budget_tokens: u64,
                /// The field `query_embedding` in the table `Activate`
                pub query_embedding: ::core::option::Option<::planus::alloc::vec::Vec<i8>>,
                /// The field `query_binary_prefilter` in the table `Activate`
                pub query_binary_prefilter: ::core::option::Option<::planus::alloc::vec::Vec<u8>>,
                /// The field `temporal_from_ns` in the table `Activate`
                pub temporal_from_ns: i64,
                /// The field `temporal_to_ns` in the table `Activate`
                pub temporal_to_ns: i64,
                /// The field `token_weights` in the table `Activate`
                pub token_weights: ::core::option::Option<::planus::alloc::vec::Vec<u16>>,
                /// The field `token_item_overhead` in the table `Activate`
                pub token_item_overhead: u32,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for Activate {
                fn default() -> Self {
                    Self {
                        conversation: ::core::default::Default::default(),
                        query: ::core::default::Default::default(),
                        turn_text: ::core::default::Default::default(),
                        budget_tokens: 0,
                        query_embedding: ::core::default::Default::default(),
                        query_binary_prefilter: ::core::default::Default::default(),
                        temporal_from_ns: 0,
                        temporal_to_ns: 0,
                        token_weights: ::core::default::Default::default(),
                        token_item_overhead: 0,
                    }
                }
            }

            impl Activate {
                /// Creates a [ActivateBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> ActivateBuilder<()> {
                    ActivateBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_conversation: impl ::planus::WriteAs<::planus::Offset<[u8]>>,
                    field_query: impl ::planus::WriteAs<::planus::Offset<[u8]>>,
                    field_turn_text: impl ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                    field_budget_tokens: impl ::planus::WriteAsDefault<u64, u64>,
                    field_query_embedding: impl ::planus::WriteAsOptional<::planus::Offset<[i8]>>,
                    field_query_binary_prefilter: impl ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                    field_temporal_from_ns: impl ::planus::WriteAsDefault<i64, i64>,
                    field_temporal_to_ns: impl ::planus::WriteAsDefault<i64, i64>,
                    field_token_weights: impl ::planus::WriteAsOptional<::planus::Offset<[u16]>>,
                    field_token_item_overhead: impl ::planus::WriteAsDefault<u32, u32>,
                ) -> ::planus::Offset<Self> {
                    let prepared_conversation = field_conversation.prepare(builder);
                    let prepared_query = field_query.prepare(builder);
                    let prepared_turn_text = field_turn_text.prepare(builder);
                    let prepared_budget_tokens = field_budget_tokens.prepare(builder, &0);
                    let prepared_query_embedding = field_query_embedding.prepare(builder);
                    let prepared_query_binary_prefilter =
                        field_query_binary_prefilter.prepare(builder);
                    let prepared_temporal_from_ns = field_temporal_from_ns.prepare(builder, &0);
                    let prepared_temporal_to_ns = field_temporal_to_ns.prepare(builder, &0);
                    let prepared_token_weights = field_token_weights.prepare(builder);
                    let prepared_token_item_overhead =
                        field_token_item_overhead.prepare(builder, &0);

                    let mut table_writer: ::planus::table_writer::TableWriter<24> =
                        ::core::default::Default::default();
                    if prepared_budget_tokens.is_some() {
                        table_writer.write_entry::<u64>(3);
                    }
                    if prepared_temporal_from_ns.is_some() {
                        table_writer.write_entry::<i64>(6);
                    }
                    if prepared_temporal_to_ns.is_some() {
                        table_writer.write_entry::<i64>(7);
                    }
                    table_writer.write_entry::<::planus::Offset<[u8]>>(0);
                    table_writer.write_entry::<::planus::Offset<[u8]>>(1);
                    if prepared_turn_text.is_some() {
                        table_writer.write_entry::<::planus::Offset<[u8]>>(2);
                    }
                    if prepared_query_embedding.is_some() {
                        table_writer.write_entry::<::planus::Offset<[i8]>>(4);
                    }
                    if prepared_query_binary_prefilter.is_some() {
                        table_writer.write_entry::<::planus::Offset<[u8]>>(5);
                    }
                    if prepared_token_weights.is_some() {
                        table_writer.write_entry::<::planus::Offset<[u16]>>(8);
                    }
                    if prepared_token_item_overhead.is_some() {
                        table_writer.write_entry::<u32>(9);
                    }

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            if let ::core::option::Option::Some(prepared_budget_tokens) =
                                prepared_budget_tokens
                            {
                                object_writer.write::<_, _, 8>(&prepared_budget_tokens);
                            }
                            if let ::core::option::Option::Some(prepared_temporal_from_ns) =
                                prepared_temporal_from_ns
                            {
                                object_writer.write::<_, _, 8>(&prepared_temporal_from_ns);
                            }
                            if let ::core::option::Option::Some(prepared_temporal_to_ns) =
                                prepared_temporal_to_ns
                            {
                                object_writer.write::<_, _, 8>(&prepared_temporal_to_ns);
                            }
                            object_writer.write::<_, _, 4>(&prepared_conversation);
                            object_writer.write::<_, _, 4>(&prepared_query);
                            if let ::core::option::Option::Some(prepared_turn_text) =
                                prepared_turn_text
                            {
                                object_writer.write::<_, _, 4>(&prepared_turn_text);
                            }
                            if let ::core::option::Option::Some(prepared_query_embedding) =
                                prepared_query_embedding
                            {
                                object_writer.write::<_, _, 4>(&prepared_query_embedding);
                            }
                            if let ::core::option::Option::Some(prepared_query_binary_prefilter) =
                                prepared_query_binary_prefilter
                            {
                                object_writer.write::<_, _, 4>(&prepared_query_binary_prefilter);
                            }
                            if let ::core::option::Option::Some(prepared_token_weights) =
                                prepared_token_weights
                            {
                                object_writer.write::<_, _, 4>(&prepared_token_weights);
                            }
                            if let ::core::option::Option::Some(prepared_token_item_overhead) =
                                prepared_token_item_overhead
                            {
                                object_writer.write::<_, _, 4>(&prepared_token_item_overhead);
                            }
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<Activate>> for Activate {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Activate> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<Activate>> for Activate {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Activate>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<Activate> for Activate {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Activate> {
                    Activate::create(
                        builder,
                        &self.conversation,
                        &self.query,
                        &self.turn_text,
                        self.budget_tokens,
                        &self.query_embedding,
                        &self.query_binary_prefilter,
                        self.temporal_from_ns,
                        self.temporal_to_ns,
                        &self.token_weights,
                        self.token_item_overhead,
                    )
                }
            }

            /// Builder for serializing an instance of the [Activate] type.
            ///
            /// Can be created using the [Activate::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct ActivateBuilder<State>(State);

            impl ActivateBuilder<()> {
                /// Setter for the [`conversation` field](Activate#structfield.conversation).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn conversation<T0>(self, value: T0) -> ActivateBuilder<(T0,)>
                where
                    T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                {
                    ActivateBuilder((value,))
                }
            }

            impl<T0> ActivateBuilder<(T0,)> {
                /// Setter for the [`query` field](Activate#structfield.query).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn query<T1>(self, value: T1) -> ActivateBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAs<::planus::Offset<[u8]>>,
                {
                    let (v0,) = self.0;
                    ActivateBuilder((v0, value))
                }
            }

            impl<T0, T1> ActivateBuilder<(T0, T1)> {
                /// Setter for the [`turn_text` field](Activate#structfield.turn_text).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn turn_text<T2>(self, value: T2) -> ActivateBuilder<(T0, T1, T2)>
                where
                    T2: ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                {
                    let (v0, v1) = self.0;
                    ActivateBuilder((v0, v1, value))
                }

                /// Sets the [`turn_text` field](Activate#structfield.turn_text) to null.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn turn_text_as_null(self) -> ActivateBuilder<(T0, T1, ())> {
                    self.turn_text(())
                }
            }

            impl<T0, T1, T2> ActivateBuilder<(T0, T1, T2)> {
                /// Setter for the [`budget_tokens` field](Activate#structfield.budget_tokens).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn budget_tokens<T3>(self, value: T3) -> ActivateBuilder<(T0, T1, T2, T3)>
                where
                    T3: ::planus::WriteAsDefault<u64, u64>,
                {
                    let (v0, v1, v2) = self.0;
                    ActivateBuilder((v0, v1, v2, value))
                }

                /// Sets the [`budget_tokens` field](Activate#structfield.budget_tokens) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn budget_tokens_as_default(
                    self,
                ) -> ActivateBuilder<(T0, T1, T2, ::planus::DefaultValue)> {
                    self.budget_tokens(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2, T3> ActivateBuilder<(T0, T1, T2, T3)> {
                /// Setter for the [`query_embedding` field](Activate#structfield.query_embedding).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn query_embedding<T4>(self, value: T4) -> ActivateBuilder<(T0, T1, T2, T3, T4)>
                where
                    T4: ::planus::WriteAsOptional<::planus::Offset<[i8]>>,
                {
                    let (v0, v1, v2, v3) = self.0;
                    ActivateBuilder((v0, v1, v2, v3, value))
                }

                /// Sets the [`query_embedding` field](Activate#structfield.query_embedding) to null.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn query_embedding_as_null(self) -> ActivateBuilder<(T0, T1, T2, T3, ())> {
                    self.query_embedding(())
                }
            }

            impl<T0, T1, T2, T3, T4> ActivateBuilder<(T0, T1, T2, T3, T4)> {
                /// Setter for the [`query_binary_prefilter` field](Activate#structfield.query_binary_prefilter).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn query_binary_prefilter<T5>(
                    self,
                    value: T5,
                ) -> ActivateBuilder<(T0, T1, T2, T3, T4, T5)>
                where
                    T5: ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                {
                    let (v0, v1, v2, v3, v4) = self.0;
                    ActivateBuilder((v0, v1, v2, v3, v4, value))
                }

                /// Sets the [`query_binary_prefilter` field](Activate#structfield.query_binary_prefilter) to null.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn query_binary_prefilter_as_null(
                    self,
                ) -> ActivateBuilder<(T0, T1, T2, T3, T4, ())> {
                    self.query_binary_prefilter(())
                }
            }

            impl<T0, T1, T2, T3, T4, T5> ActivateBuilder<(T0, T1, T2, T3, T4, T5)> {
                /// Setter for the [`temporal_from_ns` field](Activate#structfield.temporal_from_ns).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn temporal_from_ns<T6>(
                    self,
                    value: T6,
                ) -> ActivateBuilder<(T0, T1, T2, T3, T4, T5, T6)>
                where
                    T6: ::planus::WriteAsDefault<i64, i64>,
                {
                    let (v0, v1, v2, v3, v4, v5) = self.0;
                    ActivateBuilder((v0, v1, v2, v3, v4, v5, value))
                }

                /// Sets the [`temporal_from_ns` field](Activate#structfield.temporal_from_ns) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn temporal_from_ns_as_default(
                    self,
                ) -> ActivateBuilder<(T0, T1, T2, T3, T4, T5, ::planus::DefaultValue)>
                {
                    self.temporal_from_ns(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2, T3, T4, T5, T6> ActivateBuilder<(T0, T1, T2, T3, T4, T5, T6)> {
                /// Setter for the [`temporal_to_ns` field](Activate#structfield.temporal_to_ns).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn temporal_to_ns<T7>(
                    self,
                    value: T7,
                ) -> ActivateBuilder<(T0, T1, T2, T3, T4, T5, T6, T7)>
                where
                    T7: ::planus::WriteAsDefault<i64, i64>,
                {
                    let (v0, v1, v2, v3, v4, v5, v6) = self.0;
                    ActivateBuilder((v0, v1, v2, v3, v4, v5, v6, value))
                }

                /// Sets the [`temporal_to_ns` field](Activate#structfield.temporal_to_ns) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn temporal_to_ns_as_default(
                    self,
                ) -> ActivateBuilder<(T0, T1, T2, T3, T4, T5, T6, ::planus::DefaultValue)>
                {
                    self.temporal_to_ns(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2, T3, T4, T5, T6, T7> ActivateBuilder<(T0, T1, T2, T3, T4, T5, T6, T7)> {
                /// Setter for the [`token_weights` field](Activate#structfield.token_weights).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn token_weights<T8>(
                    self,
                    value: T8,
                ) -> ActivateBuilder<(T0, T1, T2, T3, T4, T5, T6, T7, T8)>
                where
                    T8: ::planus::WriteAsOptional<::planus::Offset<[u16]>>,
                {
                    let (v0, v1, v2, v3, v4, v5, v6, v7) = self.0;
                    ActivateBuilder((v0, v1, v2, v3, v4, v5, v6, v7, value))
                }

                /// Sets the [`token_weights` field](Activate#structfield.token_weights) to null.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn token_weights_as_null(
                    self,
                ) -> ActivateBuilder<(T0, T1, T2, T3, T4, T5, T6, T7, ())> {
                    self.token_weights(())
                }
            }

            impl<T0, T1, T2, T3, T4, T5, T6, T7, T8> ActivateBuilder<(T0, T1, T2, T3, T4, T5, T6, T7, T8)> {
                /// Setter for the [`token_item_overhead` field](Activate#structfield.token_item_overhead).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn token_item_overhead<T9>(
                    self,
                    value: T9,
                ) -> ActivateBuilder<(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9)>
                where
                    T9: ::planus::WriteAsDefault<u32, u32>,
                {
                    let (v0, v1, v2, v3, v4, v5, v6, v7, v8) = self.0;
                    ActivateBuilder((v0, v1, v2, v3, v4, v5, v6, v7, v8, value))
                }

                /// Sets the [`token_item_overhead` field](Activate#structfield.token_item_overhead) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn token_item_overhead_as_default(
                    self,
                ) -> ActivateBuilder<(T0, T1, T2, T3, T4, T5, T6, T7, T8, ::planus::DefaultValue)>
                {
                    self.token_item_overhead(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9>
                ActivateBuilder<(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9)>
            {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [Activate].
                #[inline]
                pub fn finish(self, builder: &mut ::planus::Builder) -> ::planus::Offset<Activate>
                where
                    Self: ::planus::WriteAsOffset<Activate>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T1: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T2: ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                T3: ::planus::WriteAsDefault<u64, u64>,
                T4: ::planus::WriteAsOptional<::planus::Offset<[i8]>>,
                T5: ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                T6: ::planus::WriteAsDefault<i64, i64>,
                T7: ::planus::WriteAsDefault<i64, i64>,
                T8: ::planus::WriteAsOptional<::planus::Offset<[u16]>>,
                T9: ::planus::WriteAsDefault<u32, u32>,
            > ::planus::WriteAs<::planus::Offset<Activate>>
                for ActivateBuilder<(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9)>
            {
                type Prepared = ::planus::Offset<Activate>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Activate> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T1: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T2: ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                T3: ::planus::WriteAsDefault<u64, u64>,
                T4: ::planus::WriteAsOptional<::planus::Offset<[i8]>>,
                T5: ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                T6: ::planus::WriteAsDefault<i64, i64>,
                T7: ::planus::WriteAsDefault<i64, i64>,
                T8: ::planus::WriteAsOptional<::planus::Offset<[u16]>>,
                T9: ::planus::WriteAsDefault<u32, u32>,
            > ::planus::WriteAsOptional<::planus::Offset<Activate>>
                for ActivateBuilder<(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9)>
            {
                type Prepared = ::planus::Offset<Activate>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Activate>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T1: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T2: ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                T3: ::planus::WriteAsDefault<u64, u64>,
                T4: ::planus::WriteAsOptional<::planus::Offset<[i8]>>,
                T5: ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                T6: ::planus::WriteAsDefault<i64, i64>,
                T7: ::planus::WriteAsDefault<i64, i64>,
                T8: ::planus::WriteAsOptional<::planus::Offset<[u16]>>,
                T9: ::planus::WriteAsDefault<u32, u32>,
            > ::planus::WriteAsOffset<Activate>
                for ActivateBuilder<(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9)>
            {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Activate> {
                    let (v0, v1, v2, v3, v4, v5, v6, v7, v8, v9) = &self.0;
                    Activate::create(builder, v0, v1, v2, v3, v4, v5, v6, v7, v8, v9)
                }
            }

            /// Reference to a deserialized [Activate].
            #[derive(Copy, Clone)]
            pub struct ActivateRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> ActivateRef<'a> {
                /// Getter for the [`conversation` field](Activate#structfield.conversation).
                #[inline]
                pub fn conversation(&self) -> ::planus::Result<&'a [u8]> {
                    self.0.access_required(0, "Activate", "conversation")
                }

                /// Getter for the [`query` field](Activate#structfield.query).
                #[inline]
                pub fn query(&self) -> ::planus::Result<&'a [u8]> {
                    self.0.access_required(1, "Activate", "query")
                }

                /// Getter for the [`turn_text` field](Activate#structfield.turn_text).
                #[inline]
                pub fn turn_text(&self) -> ::planus::Result<::core::option::Option<&'a [u8]>> {
                    self.0.access(2, "Activate", "turn_text")
                }

                /// Getter for the [`budget_tokens` field](Activate#structfield.budget_tokens).
                #[inline]
                pub fn budget_tokens(&self) -> ::planus::Result<u64> {
                    ::core::result::Result::Ok(
                        self.0.access(3, "Activate", "budget_tokens")?.unwrap_or(0),
                    )
                }

                /// Getter for the [`query_embedding` field](Activate#structfield.query_embedding).
                #[inline]
                pub fn query_embedding(
                    &self,
                ) -> ::planus::Result<::core::option::Option<&'a [i8]>> {
                    self.0.access(4, "Activate", "query_embedding")
                }

                /// Getter for the [`query_binary_prefilter` field](Activate#structfield.query_binary_prefilter).
                #[inline]
                pub fn query_binary_prefilter(
                    &self,
                ) -> ::planus::Result<::core::option::Option<&'a [u8]>> {
                    self.0.access(5, "Activate", "query_binary_prefilter")
                }

                /// Getter for the [`temporal_from_ns` field](Activate#structfield.temporal_from_ns).
                #[inline]
                pub fn temporal_from_ns(&self) -> ::planus::Result<i64> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(6, "Activate", "temporal_from_ns")?
                            .unwrap_or(0),
                    )
                }

                /// Getter for the [`temporal_to_ns` field](Activate#structfield.temporal_to_ns).
                #[inline]
                pub fn temporal_to_ns(&self) -> ::planus::Result<i64> {
                    ::core::result::Result::Ok(
                        self.0.access(7, "Activate", "temporal_to_ns")?.unwrap_or(0),
                    )
                }

                /// Getter for the [`token_weights` field](Activate#structfield.token_weights).
                #[inline]
                pub fn token_weights(
                    &self,
                ) -> ::planus::Result<::core::option::Option<::planus::Vector<'a, u16>>>
                {
                    self.0.access(8, "Activate", "token_weights")
                }

                /// Getter for the [`token_item_overhead` field](Activate#structfield.token_item_overhead).
                #[inline]
                pub fn token_item_overhead(&self) -> ::planus::Result<u32> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(9, "Activate", "token_item_overhead")?
                            .unwrap_or(0),
                    )
                }
            }

            impl<'a> ::core::fmt::Debug for ActivateRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("ActivateRef");
                    f.field("conversation", &self.conversation());
                    f.field("query", &self.query());
                    if let ::core::option::Option::Some(field_turn_text) =
                        self.turn_text().transpose()
                    {
                        f.field("turn_text", &field_turn_text);
                    }
                    f.field("budget_tokens", &self.budget_tokens());
                    if let ::core::option::Option::Some(field_query_embedding) =
                        self.query_embedding().transpose()
                    {
                        f.field("query_embedding", &field_query_embedding);
                    }
                    if let ::core::option::Option::Some(field_query_binary_prefilter) =
                        self.query_binary_prefilter().transpose()
                    {
                        f.field("query_binary_prefilter", &field_query_binary_prefilter);
                    }
                    f.field("temporal_from_ns", &self.temporal_from_ns());
                    f.field("temporal_to_ns", &self.temporal_to_ns());
                    if let ::core::option::Option::Some(field_token_weights) =
                        self.token_weights().transpose()
                    {
                        f.field("token_weights", &field_token_weights);
                    }
                    f.field("token_item_overhead", &self.token_item_overhead());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<ActivateRef<'a>> for Activate {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: ActivateRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        conversation: value.conversation()?.to_vec(),
                        query: value.query()?.to_vec(),
                        turn_text: value.turn_text()?.map(|v| v.to_vec()),
                        budget_tokens: ::core::convert::TryInto::try_into(value.budget_tokens()?)?,
                        query_embedding: value.query_embedding()?.map(|v| v.to_vec()),
                        query_binary_prefilter: value.query_binary_prefilter()?.map(|v| v.to_vec()),
                        temporal_from_ns: ::core::convert::TryInto::try_into(
                            value.temporal_from_ns()?,
                        )?,
                        temporal_to_ns: ::core::convert::TryInto::try_into(
                            value.temporal_to_ns()?,
                        )?,
                        token_weights: if let ::core::option::Option::Some(token_weights) =
                            value.token_weights()?
                        {
                            ::core::option::Option::Some(token_weights.to_vec()?)
                        } else {
                            ::core::option::Option::None
                        },
                        token_item_overhead: ::core::convert::TryInto::try_into(
                            value.token_item_overhead()?,
                        )?,
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for ActivateRef<'a> {
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

            impl<'a> ::planus::VectorReadInner<'a> for ActivateRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[ActivateRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<Activate>> for Activate {
                type Value = ::planus::Offset<Activate>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<Activate>],
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

            impl<'a> ::planus::ReadAsRoot<'a> for ActivateRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[ActivateRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `Transcript` in the namespace `hypermind.protocol`
            ///
            /// Generated from these locations:
            /// * Table `Transcript` in the file `schemas/protocol.fbs:45`
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
            pub struct Transcript {
                /// The field `conversation` in the table `Transcript`
                pub conversation: ::planus::alloc::vec::Vec<u8>,
                /// The field `since_lsn` in the table `Transcript`
                pub since_lsn: u64,
                /// The field `limit` in the table `Transcript`
                pub limit: u32,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for Transcript {
                fn default() -> Self {
                    Self {
                        conversation: ::core::default::Default::default(),
                        since_lsn: 0,
                        limit: 0,
                    }
                }
            }

            impl Transcript {
                /// Creates a [TranscriptBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> TranscriptBuilder<()> {
                    TranscriptBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_conversation: impl ::planus::WriteAs<::planus::Offset<[u8]>>,
                    field_since_lsn: impl ::planus::WriteAsDefault<u64, u64>,
                    field_limit: impl ::planus::WriteAsDefault<u32, u32>,
                ) -> ::planus::Offset<Self> {
                    let prepared_conversation = field_conversation.prepare(builder);
                    let prepared_since_lsn = field_since_lsn.prepare(builder, &0);
                    let prepared_limit = field_limit.prepare(builder, &0);

                    let mut table_writer: ::planus::table_writer::TableWriter<10> =
                        ::core::default::Default::default();
                    if prepared_since_lsn.is_some() {
                        table_writer.write_entry::<u64>(1);
                    }
                    table_writer.write_entry::<::planus::Offset<[u8]>>(0);
                    if prepared_limit.is_some() {
                        table_writer.write_entry::<u32>(2);
                    }

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            if let ::core::option::Option::Some(prepared_since_lsn) =
                                prepared_since_lsn
                            {
                                object_writer.write::<_, _, 8>(&prepared_since_lsn);
                            }
                            object_writer.write::<_, _, 4>(&prepared_conversation);
                            if let ::core::option::Option::Some(prepared_limit) = prepared_limit {
                                object_writer.write::<_, _, 4>(&prepared_limit);
                            }
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<Transcript>> for Transcript {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Transcript> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<Transcript>> for Transcript {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Transcript>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<Transcript> for Transcript {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Transcript> {
                    Transcript::create(builder, &self.conversation, self.since_lsn, self.limit)
                }
            }

            /// Builder for serializing an instance of the [Transcript] type.
            ///
            /// Can be created using the [Transcript::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct TranscriptBuilder<State>(State);

            impl TranscriptBuilder<()> {
                /// Setter for the [`conversation` field](Transcript#structfield.conversation).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn conversation<T0>(self, value: T0) -> TranscriptBuilder<(T0,)>
                where
                    T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                {
                    TranscriptBuilder((value,))
                }
            }

            impl<T0> TranscriptBuilder<(T0,)> {
                /// Setter for the [`since_lsn` field](Transcript#structfield.since_lsn).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn since_lsn<T1>(self, value: T1) -> TranscriptBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAsDefault<u64, u64>,
                {
                    let (v0,) = self.0;
                    TranscriptBuilder((v0, value))
                }

                /// Sets the [`since_lsn` field](Transcript#structfield.since_lsn) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn since_lsn_as_default(
                    self,
                ) -> TranscriptBuilder<(T0, ::planus::DefaultValue)> {
                    self.since_lsn(::planus::DefaultValue)
                }
            }

            impl<T0, T1> TranscriptBuilder<(T0, T1)> {
                /// Setter for the [`limit` field](Transcript#structfield.limit).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn limit<T2>(self, value: T2) -> TranscriptBuilder<(T0, T1, T2)>
                where
                    T2: ::planus::WriteAsDefault<u32, u32>,
                {
                    let (v0, v1) = self.0;
                    TranscriptBuilder((v0, v1, value))
                }

                /// Sets the [`limit` field](Transcript#structfield.limit) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn limit_as_default(
                    self,
                ) -> TranscriptBuilder<(T0, T1, ::planus::DefaultValue)> {
                    self.limit(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2> TranscriptBuilder<(T0, T1, T2)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [Transcript].
                #[inline]
                pub fn finish(self, builder: &mut ::planus::Builder) -> ::planus::Offset<Transcript>
                where
                    Self: ::planus::WriteAsOffset<Transcript>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T1: ::planus::WriteAsDefault<u64, u64>,
                T2: ::planus::WriteAsDefault<u32, u32>,
            > ::planus::WriteAs<::planus::Offset<Transcript>> for TranscriptBuilder<(T0, T1, T2)>
            {
                type Prepared = ::planus::Offset<Transcript>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Transcript> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T1: ::planus::WriteAsDefault<u64, u64>,
                T2: ::planus::WriteAsDefault<u32, u32>,
            > ::planus::WriteAsOptional<::planus::Offset<Transcript>>
                for TranscriptBuilder<(T0, T1, T2)>
            {
                type Prepared = ::planus::Offset<Transcript>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Transcript>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T1: ::planus::WriteAsDefault<u64, u64>,
                T2: ::planus::WriteAsDefault<u32, u32>,
            > ::planus::WriteAsOffset<Transcript> for TranscriptBuilder<(T0, T1, T2)>
            {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Transcript> {
                    let (v0, v1, v2) = &self.0;
                    Transcript::create(builder, v0, v1, v2)
                }
            }

            /// Reference to a deserialized [Transcript].
            #[derive(Copy, Clone)]
            pub struct TranscriptRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> TranscriptRef<'a> {
                /// Getter for the [`conversation` field](Transcript#structfield.conversation).
                #[inline]
                pub fn conversation(&self) -> ::planus::Result<&'a [u8]> {
                    self.0.access_required(0, "Transcript", "conversation")
                }

                /// Getter for the [`since_lsn` field](Transcript#structfield.since_lsn).
                #[inline]
                pub fn since_lsn(&self) -> ::planus::Result<u64> {
                    ::core::result::Result::Ok(
                        self.0.access(1, "Transcript", "since_lsn")?.unwrap_or(0),
                    )
                }

                /// Getter for the [`limit` field](Transcript#structfield.limit).
                #[inline]
                pub fn limit(&self) -> ::planus::Result<u32> {
                    ::core::result::Result::Ok(
                        self.0.access(2, "Transcript", "limit")?.unwrap_or(0),
                    )
                }
            }

            impl<'a> ::core::fmt::Debug for TranscriptRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("TranscriptRef");
                    f.field("conversation", &self.conversation());
                    f.field("since_lsn", &self.since_lsn());
                    f.field("limit", &self.limit());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<TranscriptRef<'a>> for Transcript {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: TranscriptRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        conversation: value.conversation()?.to_vec(),
                        since_lsn: ::core::convert::TryInto::try_into(value.since_lsn()?)?,
                        limit: ::core::convert::TryInto::try_into(value.limit()?)?,
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for TranscriptRef<'a> {
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

            impl<'a> ::planus::VectorReadInner<'a> for TranscriptRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[TranscriptRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<Transcript>> for Transcript {
                type Value = ::planus::Offset<Transcript>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<Transcript>],
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

            impl<'a> ::planus::ReadAsRoot<'a> for TranscriptRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[TranscriptRef]", "read_as_root", 0)
                    })
                }
            }

            /// The enum `RecallMode` in the namespace `hypermind.protocol`
            ///
            /// Generated from these locations:
            /// * Enum `RecallMode` in the file `schemas/protocol.fbs:51`
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
            pub enum RecallMode {
                /// The variant `list_windows` in the enum `RecallMode`
                ListWindows = 0,

                /// The variant `open_window` in the enum `RecallMode`
                OpenWindow = 1,

                /// The variant `resolve_members` in the enum `RecallMode`
                ResolveMembers = 2,
            }

            impl RecallMode {
                /// Array containing all valid variants of RecallMode
                pub const ENUM_VALUES: [Self; 3] =
                    [Self::ListWindows, Self::OpenWindow, Self::ResolveMembers];
            }

            impl ::core::convert::TryFrom<u8> for RecallMode {
                type Error = ::planus::errors::UnknownEnumTagKind;
                #[inline]
                fn try_from(
                    value: u8,
                ) -> ::core::result::Result<Self, ::planus::errors::UnknownEnumTagKind>
                {
                    #[allow(clippy::match_single_binding)]
                    match value {
                        0 => ::core::result::Result::Ok(RecallMode::ListWindows),
                        1 => ::core::result::Result::Ok(RecallMode::OpenWindow),
                        2 => ::core::result::Result::Ok(RecallMode::ResolveMembers),

                        _ => ::core::result::Result::Err(::planus::errors::UnknownEnumTagKind {
                            tag: value as i128,
                        }),
                    }
                }
            }

            impl ::core::convert::From<RecallMode> for u8 {
                #[inline]
                fn from(value: RecallMode) -> Self {
                    value as u8
                }
            }

            /// # Safety
            /// The Planus compiler correctly calculates `ALIGNMENT` and `SIZE`.
            unsafe impl ::planus::Primitive for RecallMode {
                const ALIGNMENT: usize = 1;
                const SIZE: usize = 1;
            }

            impl ::planus::WriteAsPrimitive<RecallMode> for RecallMode {
                #[inline]
                fn write<const N: usize>(
                    &self,
                    cursor: ::planus::Cursor<'_, N>,
                    buffer_position: u32,
                ) {
                    (*self as u8).write(cursor, buffer_position);
                }
            }

            impl ::planus::WriteAs<RecallMode> for RecallMode {
                type Prepared = Self;

                #[inline]
                fn prepare(&self, _builder: &mut ::planus::Builder) -> RecallMode {
                    *self
                }
            }

            impl ::planus::WriteAsDefault<RecallMode, RecallMode> for RecallMode {
                type Prepared = Self;

                #[inline]
                fn prepare(
                    &self,
                    _builder: &mut ::planus::Builder,
                    default: &RecallMode,
                ) -> ::core::option::Option<RecallMode> {
                    if self == default {
                        ::core::option::Option::None
                    } else {
                        ::core::option::Option::Some(*self)
                    }
                }
            }

            impl ::planus::WriteAsOptional<RecallMode> for RecallMode {
                type Prepared = Self;

                #[inline]
                fn prepare(
                    &self,
                    _builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<RecallMode> {
                    ::core::option::Option::Some(*self)
                }
            }

            impl<'buf> ::planus::TableRead<'buf> for RecallMode {
                #[inline]
                fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'buf>,
                    offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::ErrorKind> {
                    let n: u8 = ::planus::TableRead::from_buffer(buffer, offset)?;
                    ::core::result::Result::Ok(::core::convert::TryInto::try_into(n)?)
                }
            }

            impl<'buf> ::planus::VectorReadInner<'buf> for RecallMode {
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
                            "RecallMode",
                            "VectorRead::from_buffer",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<RecallMode> for RecallMode {
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

            /// The table `Recall` in the namespace `hypermind.protocol`
            ///
            /// Generated from these locations:
            /// * Table `Recall` in the file `schemas/protocol.fbs:53`
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
            pub struct Recall {
                /// The field `query` in the table `Recall`
                pub query: ::planus::alloc::vec::Vec<u8>,
                /// The field `limit` in the table `Recall`
                pub limit: u32,
                /// The field `mode` in the table `Recall`
                pub mode: self::RecallMode,
                /// The field `level` in the table `Recall`
                pub level: u8,
                /// The field `start_ns` in the table `Recall`
                pub start_ns: i64,
                /// The field `end_ns` in the table `Recall`
                pub end_ns: i64,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for Recall {
                fn default() -> Self {
                    Self {
                        query: ::core::default::Default::default(),
                        limit: 0,
                        mode: self::RecallMode::ListWindows,
                        level: 0,
                        start_ns: 0,
                        end_ns: 0,
                    }
                }
            }

            impl Recall {
                /// Creates a [RecallBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> RecallBuilder<()> {
                    RecallBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_query: impl ::planus::WriteAs<::planus::Offset<[u8]>>,
                    field_limit: impl ::planus::WriteAsDefault<u32, u32>,
                    field_mode: impl ::planus::WriteAsDefault<self::RecallMode, self::RecallMode>,
                    field_level: impl ::planus::WriteAsDefault<u8, u8>,
                    field_start_ns: impl ::planus::WriteAsDefault<i64, i64>,
                    field_end_ns: impl ::planus::WriteAsDefault<i64, i64>,
                ) -> ::planus::Offset<Self> {
                    let prepared_query = field_query.prepare(builder);
                    let prepared_limit = field_limit.prepare(builder, &0);
                    let prepared_mode = field_mode.prepare(builder, &self::RecallMode::ListWindows);
                    let prepared_level = field_level.prepare(builder, &0);
                    let prepared_start_ns = field_start_ns.prepare(builder, &0);
                    let prepared_end_ns = field_end_ns.prepare(builder, &0);

                    let mut table_writer: ::planus::table_writer::TableWriter<16> =
                        ::core::default::Default::default();
                    if prepared_start_ns.is_some() {
                        table_writer.write_entry::<i64>(4);
                    }
                    if prepared_end_ns.is_some() {
                        table_writer.write_entry::<i64>(5);
                    }
                    table_writer.write_entry::<::planus::Offset<[u8]>>(0);
                    if prepared_limit.is_some() {
                        table_writer.write_entry::<u32>(1);
                    }
                    if prepared_mode.is_some() {
                        table_writer.write_entry::<self::RecallMode>(2);
                    }
                    if prepared_level.is_some() {
                        table_writer.write_entry::<u8>(3);
                    }

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            if let ::core::option::Option::Some(prepared_start_ns) =
                                prepared_start_ns
                            {
                                object_writer.write::<_, _, 8>(&prepared_start_ns);
                            }
                            if let ::core::option::Option::Some(prepared_end_ns) = prepared_end_ns {
                                object_writer.write::<_, _, 8>(&prepared_end_ns);
                            }
                            object_writer.write::<_, _, 4>(&prepared_query);
                            if let ::core::option::Option::Some(prepared_limit) = prepared_limit {
                                object_writer.write::<_, _, 4>(&prepared_limit);
                            }
                            if let ::core::option::Option::Some(prepared_mode) = prepared_mode {
                                object_writer.write::<_, _, 1>(&prepared_mode);
                            }
                            if let ::core::option::Option::Some(prepared_level) = prepared_level {
                                object_writer.write::<_, _, 1>(&prepared_level);
                            }
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<Recall>> for Recall {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Recall> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<Recall>> for Recall {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Recall>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<Recall> for Recall {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Recall> {
                    Recall::create(
                        builder,
                        &self.query,
                        self.limit,
                        self.mode,
                        self.level,
                        self.start_ns,
                        self.end_ns,
                    )
                }
            }

            /// Builder for serializing an instance of the [Recall] type.
            ///
            /// Can be created using the [Recall::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct RecallBuilder<State>(State);

            impl RecallBuilder<()> {
                /// Setter for the [`query` field](Recall#structfield.query).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn query<T0>(self, value: T0) -> RecallBuilder<(T0,)>
                where
                    T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                {
                    RecallBuilder((value,))
                }
            }

            impl<T0> RecallBuilder<(T0,)> {
                /// Setter for the [`limit` field](Recall#structfield.limit).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn limit<T1>(self, value: T1) -> RecallBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAsDefault<u32, u32>,
                {
                    let (v0,) = self.0;
                    RecallBuilder((v0, value))
                }

                /// Sets the [`limit` field](Recall#structfield.limit) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn limit_as_default(self) -> RecallBuilder<(T0, ::planus::DefaultValue)> {
                    self.limit(::planus::DefaultValue)
                }
            }

            impl<T0, T1> RecallBuilder<(T0, T1)> {
                /// Setter for the [`mode` field](Recall#structfield.mode).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn mode<T2>(self, value: T2) -> RecallBuilder<(T0, T1, T2)>
                where
                    T2: ::planus::WriteAsDefault<self::RecallMode, self::RecallMode>,
                {
                    let (v0, v1) = self.0;
                    RecallBuilder((v0, v1, value))
                }

                /// Sets the [`mode` field](Recall#structfield.mode) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn mode_as_default(self) -> RecallBuilder<(T0, T1, ::planus::DefaultValue)> {
                    self.mode(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2> RecallBuilder<(T0, T1, T2)> {
                /// Setter for the [`level` field](Recall#structfield.level).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn level<T3>(self, value: T3) -> RecallBuilder<(T0, T1, T2, T3)>
                where
                    T3: ::planus::WriteAsDefault<u8, u8>,
                {
                    let (v0, v1, v2) = self.0;
                    RecallBuilder((v0, v1, v2, value))
                }

                /// Sets the [`level` field](Recall#structfield.level) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn level_as_default(
                    self,
                ) -> RecallBuilder<(T0, T1, T2, ::planus::DefaultValue)> {
                    self.level(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2, T3> RecallBuilder<(T0, T1, T2, T3)> {
                /// Setter for the [`start_ns` field](Recall#structfield.start_ns).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn start_ns<T4>(self, value: T4) -> RecallBuilder<(T0, T1, T2, T3, T4)>
                where
                    T4: ::planus::WriteAsDefault<i64, i64>,
                {
                    let (v0, v1, v2, v3) = self.0;
                    RecallBuilder((v0, v1, v2, v3, value))
                }

                /// Sets the [`start_ns` field](Recall#structfield.start_ns) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn start_ns_as_default(
                    self,
                ) -> RecallBuilder<(T0, T1, T2, T3, ::planus::DefaultValue)> {
                    self.start_ns(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2, T3, T4> RecallBuilder<(T0, T1, T2, T3, T4)> {
                /// Setter for the [`end_ns` field](Recall#structfield.end_ns).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn end_ns<T5>(self, value: T5) -> RecallBuilder<(T0, T1, T2, T3, T4, T5)>
                where
                    T5: ::planus::WriteAsDefault<i64, i64>,
                {
                    let (v0, v1, v2, v3, v4) = self.0;
                    RecallBuilder((v0, v1, v2, v3, v4, value))
                }

                /// Sets the [`end_ns` field](Recall#structfield.end_ns) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn end_ns_as_default(
                    self,
                ) -> RecallBuilder<(T0, T1, T2, T3, T4, ::planus::DefaultValue)> {
                    self.end_ns(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2, T3, T4, T5> RecallBuilder<(T0, T1, T2, T3, T4, T5)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [Recall].
                #[inline]
                pub fn finish(self, builder: &mut ::planus::Builder) -> ::planus::Offset<Recall>
                where
                    Self: ::planus::WriteAsOffset<Recall>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T1: ::planus::WriteAsDefault<u32, u32>,
                T2: ::planus::WriteAsDefault<self::RecallMode, self::RecallMode>,
                T3: ::planus::WriteAsDefault<u8, u8>,
                T4: ::planus::WriteAsDefault<i64, i64>,
                T5: ::planus::WriteAsDefault<i64, i64>,
            > ::planus::WriteAs<::planus::Offset<Recall>>
                for RecallBuilder<(T0, T1, T2, T3, T4, T5)>
            {
                type Prepared = ::planus::Offset<Recall>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Recall> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T1: ::planus::WriteAsDefault<u32, u32>,
                T2: ::planus::WriteAsDefault<self::RecallMode, self::RecallMode>,
                T3: ::planus::WriteAsDefault<u8, u8>,
                T4: ::planus::WriteAsDefault<i64, i64>,
                T5: ::planus::WriteAsDefault<i64, i64>,
            > ::planus::WriteAsOptional<::planus::Offset<Recall>>
                for RecallBuilder<(T0, T1, T2, T3, T4, T5)>
            {
                type Prepared = ::planus::Offset<Recall>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Recall>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T1: ::planus::WriteAsDefault<u32, u32>,
                T2: ::planus::WriteAsDefault<self::RecallMode, self::RecallMode>,
                T3: ::planus::WriteAsDefault<u8, u8>,
                T4: ::planus::WriteAsDefault<i64, i64>,
                T5: ::planus::WriteAsDefault<i64, i64>,
            > ::planus::WriteAsOffset<Recall> for RecallBuilder<(T0, T1, T2, T3, T4, T5)>
            {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Recall> {
                    let (v0, v1, v2, v3, v4, v5) = &self.0;
                    Recall::create(builder, v0, v1, v2, v3, v4, v5)
                }
            }

            /// Reference to a deserialized [Recall].
            #[derive(Copy, Clone)]
            pub struct RecallRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> RecallRef<'a> {
                /// Getter for the [`query` field](Recall#structfield.query).
                #[inline]
                pub fn query(&self) -> ::planus::Result<&'a [u8]> {
                    self.0.access_required(0, "Recall", "query")
                }

                /// Getter for the [`limit` field](Recall#structfield.limit).
                #[inline]
                pub fn limit(&self) -> ::planus::Result<u32> {
                    ::core::result::Result::Ok(self.0.access(1, "Recall", "limit")?.unwrap_or(0))
                }

                /// Getter for the [`mode` field](Recall#structfield.mode).
                #[inline]
                pub fn mode(&self) -> ::planus::Result<self::RecallMode> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(2, "Recall", "mode")?
                            .unwrap_or(self::RecallMode::ListWindows),
                    )
                }

                /// Getter for the [`level` field](Recall#structfield.level).
                #[inline]
                pub fn level(&self) -> ::planus::Result<u8> {
                    ::core::result::Result::Ok(self.0.access(3, "Recall", "level")?.unwrap_or(0))
                }

                /// Getter for the [`start_ns` field](Recall#structfield.start_ns).
                #[inline]
                pub fn start_ns(&self) -> ::planus::Result<i64> {
                    ::core::result::Result::Ok(self.0.access(4, "Recall", "start_ns")?.unwrap_or(0))
                }

                /// Getter for the [`end_ns` field](Recall#structfield.end_ns).
                #[inline]
                pub fn end_ns(&self) -> ::planus::Result<i64> {
                    ::core::result::Result::Ok(self.0.access(5, "Recall", "end_ns")?.unwrap_or(0))
                }
            }

            impl<'a> ::core::fmt::Debug for RecallRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("RecallRef");
                    f.field("query", &self.query());
                    f.field("limit", &self.limit());
                    f.field("mode", &self.mode());
                    f.field("level", &self.level());
                    f.field("start_ns", &self.start_ns());
                    f.field("end_ns", &self.end_ns());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<RecallRef<'a>> for Recall {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: RecallRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        query: value.query()?.to_vec(),
                        limit: ::core::convert::TryInto::try_into(value.limit()?)?,
                        mode: ::core::convert::TryInto::try_into(value.mode()?)?,
                        level: ::core::convert::TryInto::try_into(value.level()?)?,
                        start_ns: ::core::convert::TryInto::try_into(value.start_ns()?)?,
                        end_ns: ::core::convert::TryInto::try_into(value.end_ns()?)?,
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for RecallRef<'a> {
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

            impl<'a> ::planus::VectorReadInner<'a> for RecallRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[RecallRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<Recall>> for Recall {
                type Value = ::planus::Offset<Recall>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<Recall>],
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

            impl<'a> ::planus::ReadAsRoot<'a> for RecallRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[RecallRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `AsOf` in the namespace `hypermind.protocol`
            ///
            /// Generated from these locations:
            /// * Table `AsOf` in the file `schemas/protocol.fbs:62`
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
            pub struct AsOf {
                /// The field `belief_type` in the table `AsOf`
                pub belief_type: u8,
                /// The field `canonical_identity` in the table `AsOf`
                pub canonical_identity: ::planus::alloc::string::String,
                /// The field `valid_time_ns` in the table `AsOf`
                pub valid_time_ns: i64,
                /// The field `transaction_lsn` in the table `AsOf`
                pub transaction_lsn: u64,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for AsOf {
                fn default() -> Self {
                    Self {
                        belief_type: 0,
                        canonical_identity: ::core::default::Default::default(),
                        valid_time_ns: 0,
                        transaction_lsn: 0,
                    }
                }
            }

            impl AsOf {
                /// Creates a [AsOfBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> AsOfBuilder<()> {
                    AsOfBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_belief_type: impl ::planus::WriteAsDefault<u8, u8>,
                    field_canonical_identity: impl ::planus::WriteAs<::planus::Offset<str>>,
                    field_valid_time_ns: impl ::planus::WriteAsDefault<i64, i64>,
                    field_transaction_lsn: impl ::planus::WriteAsDefault<u64, u64>,
                ) -> ::planus::Offset<Self> {
                    let prepared_belief_type = field_belief_type.prepare(builder, &0);
                    let prepared_canonical_identity = field_canonical_identity.prepare(builder);
                    let prepared_valid_time_ns = field_valid_time_ns.prepare(builder, &0);
                    let prepared_transaction_lsn = field_transaction_lsn.prepare(builder, &0);

                    let mut table_writer: ::planus::table_writer::TableWriter<12> =
                        ::core::default::Default::default();
                    if prepared_valid_time_ns.is_some() {
                        table_writer.write_entry::<i64>(2);
                    }
                    if prepared_transaction_lsn.is_some() {
                        table_writer.write_entry::<u64>(3);
                    }
                    table_writer.write_entry::<::planus::Offset<str>>(1);
                    if prepared_belief_type.is_some() {
                        table_writer.write_entry::<u8>(0);
                    }

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            if let ::core::option::Option::Some(prepared_valid_time_ns) =
                                prepared_valid_time_ns
                            {
                                object_writer.write::<_, _, 8>(&prepared_valid_time_ns);
                            }
                            if let ::core::option::Option::Some(prepared_transaction_lsn) =
                                prepared_transaction_lsn
                            {
                                object_writer.write::<_, _, 8>(&prepared_transaction_lsn);
                            }
                            object_writer.write::<_, _, 4>(&prepared_canonical_identity);
                            if let ::core::option::Option::Some(prepared_belief_type) =
                                prepared_belief_type
                            {
                                object_writer.write::<_, _, 1>(&prepared_belief_type);
                            }
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<AsOf>> for AsOf {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<AsOf> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<AsOf>> for AsOf {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<AsOf>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<AsOf> for AsOf {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<AsOf> {
                    AsOf::create(
                        builder,
                        self.belief_type,
                        &self.canonical_identity,
                        self.valid_time_ns,
                        self.transaction_lsn,
                    )
                }
            }

            /// Builder for serializing an instance of the [AsOf] type.
            ///
            /// Can be created using the [AsOf::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct AsOfBuilder<State>(State);

            impl AsOfBuilder<()> {
                /// Setter for the [`belief_type` field](AsOf#structfield.belief_type).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn belief_type<T0>(self, value: T0) -> AsOfBuilder<(T0,)>
                where
                    T0: ::planus::WriteAsDefault<u8, u8>,
                {
                    AsOfBuilder((value,))
                }

                /// Sets the [`belief_type` field](AsOf#structfield.belief_type) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn belief_type_as_default(self) -> AsOfBuilder<(::planus::DefaultValue,)> {
                    self.belief_type(::planus::DefaultValue)
                }
            }

            impl<T0> AsOfBuilder<(T0,)> {
                /// Setter for the [`canonical_identity` field](AsOf#structfield.canonical_identity).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn canonical_identity<T1>(self, value: T1) -> AsOfBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAs<::planus::Offset<str>>,
                {
                    let (v0,) = self.0;
                    AsOfBuilder((v0, value))
                }
            }

            impl<T0, T1> AsOfBuilder<(T0, T1)> {
                /// Setter for the [`valid_time_ns` field](AsOf#structfield.valid_time_ns).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn valid_time_ns<T2>(self, value: T2) -> AsOfBuilder<(T0, T1, T2)>
                where
                    T2: ::planus::WriteAsDefault<i64, i64>,
                {
                    let (v0, v1) = self.0;
                    AsOfBuilder((v0, v1, value))
                }

                /// Sets the [`valid_time_ns` field](AsOf#structfield.valid_time_ns) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn valid_time_ns_as_default(
                    self,
                ) -> AsOfBuilder<(T0, T1, ::planus::DefaultValue)> {
                    self.valid_time_ns(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2> AsOfBuilder<(T0, T1, T2)> {
                /// Setter for the [`transaction_lsn` field](AsOf#structfield.transaction_lsn).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn transaction_lsn<T3>(self, value: T3) -> AsOfBuilder<(T0, T1, T2, T3)>
                where
                    T3: ::planus::WriteAsDefault<u64, u64>,
                {
                    let (v0, v1, v2) = self.0;
                    AsOfBuilder((v0, v1, v2, value))
                }

                /// Sets the [`transaction_lsn` field](AsOf#structfield.transaction_lsn) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn transaction_lsn_as_default(
                    self,
                ) -> AsOfBuilder<(T0, T1, T2, ::planus::DefaultValue)> {
                    self.transaction_lsn(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2, T3> AsOfBuilder<(T0, T1, T2, T3)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [AsOf].
                #[inline]
                pub fn finish(self, builder: &mut ::planus::Builder) -> ::planus::Offset<AsOf>
                where
                    Self: ::planus::WriteAsOffset<AsOf>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u8, u8>,
                T1: ::planus::WriteAs<::planus::Offset<str>>,
                T2: ::planus::WriteAsDefault<i64, i64>,
                T3: ::planus::WriteAsDefault<u64, u64>,
            > ::planus::WriteAs<::planus::Offset<AsOf>> for AsOfBuilder<(T0, T1, T2, T3)>
            {
                type Prepared = ::planus::Offset<AsOf>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<AsOf> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u8, u8>,
                T1: ::planus::WriteAs<::planus::Offset<str>>,
                T2: ::planus::WriteAsDefault<i64, i64>,
                T3: ::planus::WriteAsDefault<u64, u64>,
            > ::planus::WriteAsOptional<::planus::Offset<AsOf>> for AsOfBuilder<(T0, T1, T2, T3)>
            {
                type Prepared = ::planus::Offset<AsOf>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<AsOf>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u8, u8>,
                T1: ::planus::WriteAs<::planus::Offset<str>>,
                T2: ::planus::WriteAsDefault<i64, i64>,
                T3: ::planus::WriteAsDefault<u64, u64>,
            > ::planus::WriteAsOffset<AsOf> for AsOfBuilder<(T0, T1, T2, T3)>
            {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<AsOf> {
                    let (v0, v1, v2, v3) = &self.0;
                    AsOf::create(builder, v0, v1, v2, v3)
                }
            }

            /// Reference to a deserialized [AsOf].
            #[derive(Copy, Clone)]
            pub struct AsOfRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> AsOfRef<'a> {
                /// Getter for the [`belief_type` field](AsOf#structfield.belief_type).
                #[inline]
                pub fn belief_type(&self) -> ::planus::Result<u8> {
                    ::core::result::Result::Ok(
                        self.0.access(0, "AsOf", "belief_type")?.unwrap_or(0),
                    )
                }

                /// Getter for the [`canonical_identity` field](AsOf#structfield.canonical_identity).
                #[inline]
                pub fn canonical_identity(&self) -> ::planus::Result<&'a ::core::primitive::str> {
                    self.0.access_required(1, "AsOf", "canonical_identity")
                }

                /// Getter for the [`valid_time_ns` field](AsOf#structfield.valid_time_ns).
                #[inline]
                pub fn valid_time_ns(&self) -> ::planus::Result<i64> {
                    ::core::result::Result::Ok(
                        self.0.access(2, "AsOf", "valid_time_ns")?.unwrap_or(0),
                    )
                }

                /// Getter for the [`transaction_lsn` field](AsOf#structfield.transaction_lsn).
                #[inline]
                pub fn transaction_lsn(&self) -> ::planus::Result<u64> {
                    ::core::result::Result::Ok(
                        self.0.access(3, "AsOf", "transaction_lsn")?.unwrap_or(0),
                    )
                }
            }

            impl<'a> ::core::fmt::Debug for AsOfRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("AsOfRef");
                    f.field("belief_type", &self.belief_type());
                    f.field("canonical_identity", &self.canonical_identity());
                    f.field("valid_time_ns", &self.valid_time_ns());
                    f.field("transaction_lsn", &self.transaction_lsn());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<AsOfRef<'a>> for AsOf {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: AsOfRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        belief_type: ::core::convert::TryInto::try_into(value.belief_type()?)?,
                        canonical_identity: ::core::convert::Into::into(
                            value.canonical_identity()?,
                        ),
                        valid_time_ns: ::core::convert::TryInto::try_into(value.valid_time_ns()?)?,
                        transaction_lsn: ::core::convert::TryInto::try_into(
                            value.transaction_lsn()?,
                        )?,
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for AsOfRef<'a> {
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

            impl<'a> ::planus::VectorReadInner<'a> for AsOfRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location("[AsOfRef]", "get", buffer.offset_from_start)
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<AsOf>> for AsOf {
                type Value = ::planus::Offset<AsOf>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<AsOf>],
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

            impl<'a> ::planus::ReadAsRoot<'a> for AsOfRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[AsOfRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `Checkpoint` in the namespace `hypermind.protocol`
            ///
            /// Generated from these locations:
            /// * Table `Checkpoint` in the file `schemas/protocol.fbs:69`
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
                /// The field `turn_id` in the table `Checkpoint`
                pub turn_id: ::planus::alloc::vec::Vec<u8>,
                /// The field `blob` in the table `Checkpoint`
                pub blob: ::planus::alloc::vec::Vec<u8>,
                /// The field `client_seq` in the table `Checkpoint`
                pub client_seq: u64,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for Checkpoint {
                fn default() -> Self {
                    Self {
                        turn_id: ::core::default::Default::default(),
                        blob: ::core::default::Default::default(),
                        client_seq: 0,
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
                    field_turn_id: impl ::planus::WriteAs<::planus::Offset<[u8]>>,
                    field_blob: impl ::planus::WriteAs<::planus::Offset<[u8]>>,
                    field_client_seq: impl ::planus::WriteAsDefault<u64, u64>,
                ) -> ::planus::Offset<Self> {
                    let prepared_turn_id = field_turn_id.prepare(builder);
                    let prepared_blob = field_blob.prepare(builder);
                    let prepared_client_seq = field_client_seq.prepare(builder, &0);

                    let mut table_writer: ::planus::table_writer::TableWriter<10> =
                        ::core::default::Default::default();
                    if prepared_client_seq.is_some() {
                        table_writer.write_entry::<u64>(2);
                    }
                    table_writer.write_entry::<::planus::Offset<[u8]>>(0);
                    table_writer.write_entry::<::planus::Offset<[u8]>>(1);

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            if let ::core::option::Option::Some(prepared_client_seq) =
                                prepared_client_seq
                            {
                                object_writer.write::<_, _, 8>(&prepared_client_seq);
                            }
                            object_writer.write::<_, _, 4>(&prepared_turn_id);
                            object_writer.write::<_, _, 4>(&prepared_blob);
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
                    Checkpoint::create(builder, &self.turn_id, &self.blob, self.client_seq)
                }
            }

            /// Builder for serializing an instance of the [Checkpoint] type.
            ///
            /// Can be created using the [Checkpoint::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct CheckpointBuilder<State>(State);

            impl CheckpointBuilder<()> {
                /// Setter for the [`turn_id` field](Checkpoint#structfield.turn_id).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn turn_id<T0>(self, value: T0) -> CheckpointBuilder<(T0,)>
                where
                    T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                {
                    CheckpointBuilder((value,))
                }
            }

            impl<T0> CheckpointBuilder<(T0,)> {
                /// Setter for the [`blob` field](Checkpoint#structfield.blob).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn blob<T1>(self, value: T1) -> CheckpointBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAs<::planus::Offset<[u8]>>,
                {
                    let (v0,) = self.0;
                    CheckpointBuilder((v0, value))
                }
            }

            impl<T0, T1> CheckpointBuilder<(T0, T1)> {
                /// Setter for the [`client_seq` field](Checkpoint#structfield.client_seq).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn client_seq<T2>(self, value: T2) -> CheckpointBuilder<(T0, T1, T2)>
                where
                    T2: ::planus::WriteAsDefault<u64, u64>,
                {
                    let (v0, v1) = self.0;
                    CheckpointBuilder((v0, v1, value))
                }

                /// Sets the [`client_seq` field](Checkpoint#structfield.client_seq) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn client_seq_as_default(
                    self,
                ) -> CheckpointBuilder<(T0, T1, ::planus::DefaultValue)> {
                    self.client_seq(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2> CheckpointBuilder<(T0, T1, T2)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [Checkpoint].
                #[inline]
                pub fn finish(self, builder: &mut ::planus::Builder) -> ::planus::Offset<Checkpoint>
                where
                    Self: ::planus::WriteAsOffset<Checkpoint>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T1: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T2: ::planus::WriteAsDefault<u64, u64>,
            > ::planus::WriteAs<::planus::Offset<Checkpoint>> for CheckpointBuilder<(T0, T1, T2)>
            {
                type Prepared = ::planus::Offset<Checkpoint>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Checkpoint> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T1: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T2: ::planus::WriteAsDefault<u64, u64>,
            > ::planus::WriteAsOptional<::planus::Offset<Checkpoint>>
                for CheckpointBuilder<(T0, T1, T2)>
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

            impl<
                T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T1: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T2: ::planus::WriteAsDefault<u64, u64>,
            > ::planus::WriteAsOffset<Checkpoint> for CheckpointBuilder<(T0, T1, T2)>
            {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Checkpoint> {
                    let (v0, v1, v2) = &self.0;
                    Checkpoint::create(builder, v0, v1, v2)
                }
            }

            /// Reference to a deserialized [Checkpoint].
            #[derive(Copy, Clone)]
            pub struct CheckpointRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> CheckpointRef<'a> {
                /// Getter for the [`turn_id` field](Checkpoint#structfield.turn_id).
                #[inline]
                pub fn turn_id(&self) -> ::planus::Result<&'a [u8]> {
                    self.0.access_required(0, "Checkpoint", "turn_id")
                }

                /// Getter for the [`blob` field](Checkpoint#structfield.blob).
                #[inline]
                pub fn blob(&self) -> ::planus::Result<&'a [u8]> {
                    self.0.access_required(1, "Checkpoint", "blob")
                }

                /// Getter for the [`client_seq` field](Checkpoint#structfield.client_seq).
                #[inline]
                pub fn client_seq(&self) -> ::planus::Result<u64> {
                    ::core::result::Result::Ok(
                        self.0.access(2, "Checkpoint", "client_seq")?.unwrap_or(0),
                    )
                }
            }

            impl<'a> ::core::fmt::Debug for CheckpointRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("CheckpointRef");
                    f.field("turn_id", &self.turn_id());
                    f.field("blob", &self.blob());
                    f.field("client_seq", &self.client_seq());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<CheckpointRef<'a>> for Checkpoint {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: CheckpointRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        turn_id: value.turn_id()?.to_vec(),
                        blob: value.blob()?.to_vec(),
                        client_seq: ::core::convert::TryInto::try_into(value.client_seq()?)?,
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

            /// The table `LatestCheckpoint` in the namespace `hypermind.protocol`
            ///
            /// Generated from these locations:
            /// * Table `LatestCheckpoint` in the file `schemas/protocol.fbs:75`
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
            pub struct LatestCheckpoint {
                /// The field `turn_id` in the table `LatestCheckpoint`
                pub turn_id: ::planus::alloc::vec::Vec<u8>,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for LatestCheckpoint {
                fn default() -> Self {
                    Self {
                        turn_id: ::core::default::Default::default(),
                    }
                }
            }

            impl LatestCheckpoint {
                /// Creates a [LatestCheckpointBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> LatestCheckpointBuilder<()> {
                    LatestCheckpointBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_turn_id: impl ::planus::WriteAs<::planus::Offset<[u8]>>,
                ) -> ::planus::Offset<Self> {
                    let prepared_turn_id = field_turn_id.prepare(builder);

                    let mut table_writer: ::planus::table_writer::TableWriter<6> =
                        ::core::default::Default::default();
                    table_writer.write_entry::<::planus::Offset<[u8]>>(0);

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            object_writer.write::<_, _, 4>(&prepared_turn_id);
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<LatestCheckpoint>> for LatestCheckpoint {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<LatestCheckpoint> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<LatestCheckpoint>> for LatestCheckpoint {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<LatestCheckpoint>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<LatestCheckpoint> for LatestCheckpoint {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<LatestCheckpoint> {
                    LatestCheckpoint::create(builder, &self.turn_id)
                }
            }

            /// Builder for serializing an instance of the [LatestCheckpoint] type.
            ///
            /// Can be created using the [LatestCheckpoint::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct LatestCheckpointBuilder<State>(State);

            impl LatestCheckpointBuilder<()> {
                /// Setter for the [`turn_id` field](LatestCheckpoint#structfield.turn_id).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn turn_id<T0>(self, value: T0) -> LatestCheckpointBuilder<(T0,)>
                where
                    T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                {
                    LatestCheckpointBuilder((value,))
                }
            }

            impl<T0> LatestCheckpointBuilder<(T0,)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [LatestCheckpoint].
                #[inline]
                pub fn finish(
                    self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<LatestCheckpoint>
                where
                    Self: ::planus::WriteAsOffset<LatestCheckpoint>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<T0: ::planus::WriteAs<::planus::Offset<[u8]>>>
                ::planus::WriteAs<::planus::Offset<LatestCheckpoint>>
                for LatestCheckpointBuilder<(T0,)>
            {
                type Prepared = ::planus::Offset<LatestCheckpoint>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<LatestCheckpoint> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<T0: ::planus::WriteAs<::planus::Offset<[u8]>>>
                ::planus::WriteAsOptional<::planus::Offset<LatestCheckpoint>>
                for LatestCheckpointBuilder<(T0,)>
            {
                type Prepared = ::planus::Offset<LatestCheckpoint>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<LatestCheckpoint>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<T0: ::planus::WriteAs<::planus::Offset<[u8]>>>
                ::planus::WriteAsOffset<LatestCheckpoint> for LatestCheckpointBuilder<(T0,)>
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<LatestCheckpoint> {
                    let (v0,) = &self.0;
                    LatestCheckpoint::create(builder, v0)
                }
            }

            /// Reference to a deserialized [LatestCheckpoint].
            #[derive(Copy, Clone)]
            pub struct LatestCheckpointRef<'a>(
                #[allow(dead_code)] ::planus::table_reader::Table<'a>,
            );

            impl<'a> LatestCheckpointRef<'a> {
                /// Getter for the [`turn_id` field](LatestCheckpoint#structfield.turn_id).
                #[inline]
                pub fn turn_id(&self) -> ::planus::Result<&'a [u8]> {
                    self.0.access_required(0, "LatestCheckpoint", "turn_id")
                }
            }

            impl<'a> ::core::fmt::Debug for LatestCheckpointRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("LatestCheckpointRef");
                    f.field("turn_id", &self.turn_id());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<LatestCheckpointRef<'a>> for LatestCheckpoint {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: LatestCheckpointRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        turn_id: value.turn_id()?.to_vec(),
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for LatestCheckpointRef<'a> {
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

            impl<'a> ::planus::VectorReadInner<'a> for LatestCheckpointRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[LatestCheckpointRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<LatestCheckpoint>> for LatestCheckpoint {
                type Value = ::planus::Offset<LatestCheckpoint>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<LatestCheckpoint>],
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

            impl<'a> ::planus::ReadAsRoot<'a> for LatestCheckpointRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[LatestCheckpointRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `Attest` in the namespace `hypermind.protocol`
            ///
            /// Generated from these locations:
            /// * Table `Attest` in the file `schemas/protocol.fbs:79`
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
            pub struct Attest {
                /// The field `used` in the table `Attest`
                pub used: ::core::option::Option<::planus::alloc::vec::Vec<u64>>,
                /// The field `ignored` in the table `Attest`
                pub ignored: ::core::option::Option<::planus::alloc::vec::Vec<u64>>,
                /// The field `client_seq` in the table `Attest`
                pub client_seq: u64,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for Attest {
                fn default() -> Self {
                    Self {
                        used: ::core::default::Default::default(),
                        ignored: ::core::default::Default::default(),
                        client_seq: 0,
                    }
                }
            }

            impl Attest {
                /// Creates a [AttestBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> AttestBuilder<()> {
                    AttestBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_used: impl ::planus::WriteAsOptional<::planus::Offset<[u64]>>,
                    field_ignored: impl ::planus::WriteAsOptional<::planus::Offset<[u64]>>,
                    field_client_seq: impl ::planus::WriteAsDefault<u64, u64>,
                ) -> ::planus::Offset<Self> {
                    let prepared_used = field_used.prepare(builder);
                    let prepared_ignored = field_ignored.prepare(builder);
                    let prepared_client_seq = field_client_seq.prepare(builder, &0);

                    let mut table_writer: ::planus::table_writer::TableWriter<10> =
                        ::core::default::Default::default();
                    if prepared_client_seq.is_some() {
                        table_writer.write_entry::<u64>(2);
                    }
                    if prepared_used.is_some() {
                        table_writer.write_entry::<::planus::Offset<[u64]>>(0);
                    }
                    if prepared_ignored.is_some() {
                        table_writer.write_entry::<::planus::Offset<[u64]>>(1);
                    }

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            if let ::core::option::Option::Some(prepared_client_seq) =
                                prepared_client_seq
                            {
                                object_writer.write::<_, _, 8>(&prepared_client_seq);
                            }
                            if let ::core::option::Option::Some(prepared_used) = prepared_used {
                                object_writer.write::<_, _, 4>(&prepared_used);
                            }
                            if let ::core::option::Option::Some(prepared_ignored) = prepared_ignored
                            {
                                object_writer.write::<_, _, 4>(&prepared_ignored);
                            }
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<Attest>> for Attest {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Attest> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<Attest>> for Attest {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Attest>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<Attest> for Attest {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Attest> {
                    Attest::create(builder, &self.used, &self.ignored, self.client_seq)
                }
            }

            /// Builder for serializing an instance of the [Attest] type.
            ///
            /// Can be created using the [Attest::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct AttestBuilder<State>(State);

            impl AttestBuilder<()> {
                /// Setter for the [`used` field](Attest#structfield.used).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn used<T0>(self, value: T0) -> AttestBuilder<(T0,)>
                where
                    T0: ::planus::WriteAsOptional<::planus::Offset<[u64]>>,
                {
                    AttestBuilder((value,))
                }

                /// Sets the [`used` field](Attest#structfield.used) to null.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn used_as_null(self) -> AttestBuilder<((),)> {
                    self.used(())
                }
            }

            impl<T0> AttestBuilder<(T0,)> {
                /// Setter for the [`ignored` field](Attest#structfield.ignored).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn ignored<T1>(self, value: T1) -> AttestBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAsOptional<::planus::Offset<[u64]>>,
                {
                    let (v0,) = self.0;
                    AttestBuilder((v0, value))
                }

                /// Sets the [`ignored` field](Attest#structfield.ignored) to null.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn ignored_as_null(self) -> AttestBuilder<(T0, ())> {
                    self.ignored(())
                }
            }

            impl<T0, T1> AttestBuilder<(T0, T1)> {
                /// Setter for the [`client_seq` field](Attest#structfield.client_seq).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn client_seq<T2>(self, value: T2) -> AttestBuilder<(T0, T1, T2)>
                where
                    T2: ::planus::WriteAsDefault<u64, u64>,
                {
                    let (v0, v1) = self.0;
                    AttestBuilder((v0, v1, value))
                }

                /// Sets the [`client_seq` field](Attest#structfield.client_seq) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn client_seq_as_default(
                    self,
                ) -> AttestBuilder<(T0, T1, ::planus::DefaultValue)> {
                    self.client_seq(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2> AttestBuilder<(T0, T1, T2)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [Attest].
                #[inline]
                pub fn finish(self, builder: &mut ::planus::Builder) -> ::planus::Offset<Attest>
                where
                    Self: ::planus::WriteAsOffset<Attest>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsOptional<::planus::Offset<[u64]>>,
                T1: ::planus::WriteAsOptional<::planus::Offset<[u64]>>,
                T2: ::planus::WriteAsDefault<u64, u64>,
            > ::planus::WriteAs<::planus::Offset<Attest>> for AttestBuilder<(T0, T1, T2)>
            {
                type Prepared = ::planus::Offset<Attest>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Attest> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsOptional<::planus::Offset<[u64]>>,
                T1: ::planus::WriteAsOptional<::planus::Offset<[u64]>>,
                T2: ::planus::WriteAsDefault<u64, u64>,
            > ::planus::WriteAsOptional<::planus::Offset<Attest>> for AttestBuilder<(T0, T1, T2)>
            {
                type Prepared = ::planus::Offset<Attest>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Attest>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAsOptional<::planus::Offset<[u64]>>,
                T1: ::planus::WriteAsOptional<::planus::Offset<[u64]>>,
                T2: ::planus::WriteAsDefault<u64, u64>,
            > ::planus::WriteAsOffset<Attest> for AttestBuilder<(T0, T1, T2)>
            {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Attest> {
                    let (v0, v1, v2) = &self.0;
                    Attest::create(builder, v0, v1, v2)
                }
            }

            /// Reference to a deserialized [Attest].
            #[derive(Copy, Clone)]
            pub struct AttestRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> AttestRef<'a> {
                /// Getter for the [`used` field](Attest#structfield.used).
                #[inline]
                pub fn used(
                    &self,
                ) -> ::planus::Result<::core::option::Option<::planus::Vector<'a, u64>>>
                {
                    self.0.access(0, "Attest", "used")
                }

                /// Getter for the [`ignored` field](Attest#structfield.ignored).
                #[inline]
                pub fn ignored(
                    &self,
                ) -> ::planus::Result<::core::option::Option<::planus::Vector<'a, u64>>>
                {
                    self.0.access(1, "Attest", "ignored")
                }

                /// Getter for the [`client_seq` field](Attest#structfield.client_seq).
                #[inline]
                pub fn client_seq(&self) -> ::planus::Result<u64> {
                    ::core::result::Result::Ok(
                        self.0.access(2, "Attest", "client_seq")?.unwrap_or(0),
                    )
                }
            }

            impl<'a> ::core::fmt::Debug for AttestRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("AttestRef");
                    if let ::core::option::Option::Some(field_used) = self.used().transpose() {
                        f.field("used", &field_used);
                    }
                    if let ::core::option::Option::Some(field_ignored) = self.ignored().transpose()
                    {
                        f.field("ignored", &field_ignored);
                    }
                    f.field("client_seq", &self.client_seq());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<AttestRef<'a>> for Attest {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: AttestRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        used: if let ::core::option::Option::Some(used) = value.used()? {
                            ::core::option::Option::Some(used.to_vec()?)
                        } else {
                            ::core::option::Option::None
                        },
                        ignored: if let ::core::option::Option::Some(ignored) = value.ignored()? {
                            ::core::option::Option::Some(ignored.to_vec()?)
                        } else {
                            ::core::option::Option::None
                        },
                        client_seq: ::core::convert::TryInto::try_into(value.client_seq()?)?,
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for AttestRef<'a> {
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

            impl<'a> ::planus::VectorReadInner<'a> for AttestRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[AttestRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<Attest>> for Attest {
                type Value = ::planus::Offset<Attest>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<Attest>],
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

            impl<'a> ::planus::ReadAsRoot<'a> for AttestRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[AttestRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `Subscribe` in the namespace `hypermind.protocol`
            ///
            /// Generated from these locations:
            /// * Table `Subscribe` in the file `schemas/protocol.fbs:85`
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
            pub struct Subscribe {
                /// The field `conversation` in the table `Subscribe`
                pub conversation: ::core::option::Option<::planus::alloc::vec::Vec<u8>>,
                /// The field `since_lsn` in the table `Subscribe`
                pub since_lsn: u64,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for Subscribe {
                fn default() -> Self {
                    Self {
                        conversation: ::core::default::Default::default(),
                        since_lsn: 0,
                    }
                }
            }

            impl Subscribe {
                /// Creates a [SubscribeBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> SubscribeBuilder<()> {
                    SubscribeBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_conversation: impl ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                    field_since_lsn: impl ::planus::WriteAsDefault<u64, u64>,
                ) -> ::planus::Offset<Self> {
                    let prepared_conversation = field_conversation.prepare(builder);
                    let prepared_since_lsn = field_since_lsn.prepare(builder, &0);

                    let mut table_writer: ::planus::table_writer::TableWriter<8> =
                        ::core::default::Default::default();
                    if prepared_since_lsn.is_some() {
                        table_writer.write_entry::<u64>(1);
                    }
                    if prepared_conversation.is_some() {
                        table_writer.write_entry::<::planus::Offset<[u8]>>(0);
                    }

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            if let ::core::option::Option::Some(prepared_since_lsn) =
                                prepared_since_lsn
                            {
                                object_writer.write::<_, _, 8>(&prepared_since_lsn);
                            }
                            if let ::core::option::Option::Some(prepared_conversation) =
                                prepared_conversation
                            {
                                object_writer.write::<_, _, 4>(&prepared_conversation);
                            }
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<Subscribe>> for Subscribe {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Subscribe> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<Subscribe>> for Subscribe {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Subscribe>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<Subscribe> for Subscribe {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Subscribe> {
                    Subscribe::create(builder, &self.conversation, self.since_lsn)
                }
            }

            /// Builder for serializing an instance of the [Subscribe] type.
            ///
            /// Can be created using the [Subscribe::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct SubscribeBuilder<State>(State);

            impl SubscribeBuilder<()> {
                /// Setter for the [`conversation` field](Subscribe#structfield.conversation).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn conversation<T0>(self, value: T0) -> SubscribeBuilder<(T0,)>
                where
                    T0: ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                {
                    SubscribeBuilder((value,))
                }

                /// Sets the [`conversation` field](Subscribe#structfield.conversation) to null.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn conversation_as_null(self) -> SubscribeBuilder<((),)> {
                    self.conversation(())
                }
            }

            impl<T0> SubscribeBuilder<(T0,)> {
                /// Setter for the [`since_lsn` field](Subscribe#structfield.since_lsn).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn since_lsn<T1>(self, value: T1) -> SubscribeBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAsDefault<u64, u64>,
                {
                    let (v0,) = self.0;
                    SubscribeBuilder((v0, value))
                }

                /// Sets the [`since_lsn` field](Subscribe#structfield.since_lsn) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn since_lsn_as_default(
                    self,
                ) -> SubscribeBuilder<(T0, ::planus::DefaultValue)> {
                    self.since_lsn(::planus::DefaultValue)
                }
            }

            impl<T0, T1> SubscribeBuilder<(T0, T1)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [Subscribe].
                #[inline]
                pub fn finish(self, builder: &mut ::planus::Builder) -> ::planus::Offset<Subscribe>
                where
                    Self: ::planus::WriteAsOffset<Subscribe>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                T1: ::planus::WriteAsDefault<u64, u64>,
            > ::planus::WriteAs<::planus::Offset<Subscribe>> for SubscribeBuilder<(T0, T1)>
            {
                type Prepared = ::planus::Offset<Subscribe>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Subscribe> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                T1: ::planus::WriteAsDefault<u64, u64>,
            > ::planus::WriteAsOptional<::planus::Offset<Subscribe>>
                for SubscribeBuilder<(T0, T1)>
            {
                type Prepared = ::planus::Offset<Subscribe>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Subscribe>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                T1: ::planus::WriteAsDefault<u64, u64>,
            > ::planus::WriteAsOffset<Subscribe> for SubscribeBuilder<(T0, T1)>
            {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Subscribe> {
                    let (v0, v1) = &self.0;
                    Subscribe::create(builder, v0, v1)
                }
            }

            /// Reference to a deserialized [Subscribe].
            #[derive(Copy, Clone)]
            pub struct SubscribeRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> SubscribeRef<'a> {
                /// Getter for the [`conversation` field](Subscribe#structfield.conversation).
                #[inline]
                pub fn conversation(&self) -> ::planus::Result<::core::option::Option<&'a [u8]>> {
                    self.0.access(0, "Subscribe", "conversation")
                }

                /// Getter for the [`since_lsn` field](Subscribe#structfield.since_lsn).
                #[inline]
                pub fn since_lsn(&self) -> ::planus::Result<u64> {
                    ::core::result::Result::Ok(
                        self.0.access(1, "Subscribe", "since_lsn")?.unwrap_or(0),
                    )
                }
            }

            impl<'a> ::core::fmt::Debug for SubscribeRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("SubscribeRef");
                    if let ::core::option::Option::Some(field_conversation) =
                        self.conversation().transpose()
                    {
                        f.field("conversation", &field_conversation);
                    }
                    f.field("since_lsn", &self.since_lsn());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<SubscribeRef<'a>> for Subscribe {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: SubscribeRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        conversation: value.conversation()?.map(|v| v.to_vec()),
                        since_lsn: ::core::convert::TryInto::try_into(value.since_lsn()?)?,
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for SubscribeRef<'a> {
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

            impl<'a> ::planus::VectorReadInner<'a> for SubscribeRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[SubscribeRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<Subscribe>> for Subscribe {
                type Value = ::planus::Offset<Subscribe>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<Subscribe>],
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

            impl<'a> ::planus::ReadAsRoot<'a> for SubscribeRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[SubscribeRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `Health` in the namespace `hypermind.protocol`
            ///
            /// Generated from these locations:
            /// * Table `Health` in the file `schemas/protocol.fbs:90`
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
            pub struct Health {}

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for Health {
                fn default() -> Self {
                    Self {}
                }
            }

            impl Health {
                /// Creates a [HealthBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> HealthBuilder<()> {
                    HealthBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(builder: &mut ::planus::Builder) -> ::planus::Offset<Self> {
                    let table_writer: ::planus::table_writer::TableWriter<4> =
                        ::core::default::Default::default();
                    unsafe {
                        table_writer.finish(builder, |_table_writer| {});
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<Health>> for Health {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Health> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<Health>> for Health {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Health>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<Health> for Health {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Health> {
                    Health::create(builder)
                }
            }

            /// Builder for serializing an instance of the [Health] type.
            ///
            /// Can be created using the [Health::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct HealthBuilder<State>(State);

            impl HealthBuilder<()> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [Health].
                #[inline]
                pub fn finish(self, builder: &mut ::planus::Builder) -> ::planus::Offset<Health>
                where
                    Self: ::planus::WriteAsOffset<Health>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl ::planus::WriteAs<::planus::Offset<Health>> for HealthBuilder<()> {
                type Prepared = ::planus::Offset<Health>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Health> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<Health>> for HealthBuilder<()> {
                type Prepared = ::planus::Offset<Health>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Health>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<Health> for HealthBuilder<()> {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Health> {
                    Health::create(builder)
                }
            }

            /// Reference to a deserialized [Health].
            #[derive(Copy, Clone)]
            pub struct HealthRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> HealthRef<'a> {}

            impl<'a> ::core::fmt::Debug for HealthRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("HealthRef");

                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<HealthRef<'a>> for Health {
                type Error = ::planus::Error;

                fn try_from(_value: HealthRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {})
                }
            }

            impl<'a> ::planus::TableRead<'a> for HealthRef<'a> {
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

            impl<'a> ::planus::VectorReadInner<'a> for HealthRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[HealthRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<Health>> for Health {
                type Value = ::planus::Offset<Health>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<Health>],
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

            impl<'a> ::planus::ReadAsRoot<'a> for HealthRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[HealthRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `Stats` in the namespace `hypermind.protocol`
            ///
            /// Generated from these locations:
            /// * Table `Stats` in the file `schemas/protocol.fbs:92`
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
            pub struct Stats {
                /// The field `actor` in the table `Stats`
                pub actor: u16,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for Stats {
                fn default() -> Self {
                    Self { actor: 0 }
                }
            }

            impl Stats {
                /// Creates a [StatsBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> StatsBuilder<()> {
                    StatsBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_actor: impl ::planus::WriteAsDefault<u16, u16>,
                ) -> ::planus::Offset<Self> {
                    let prepared_actor = field_actor.prepare(builder, &0);

                    let mut table_writer: ::planus::table_writer::TableWriter<6> =
                        ::core::default::Default::default();
                    if prepared_actor.is_some() {
                        table_writer.write_entry::<u16>(0);
                    }

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            if let ::core::option::Option::Some(prepared_actor) = prepared_actor {
                                object_writer.write::<_, _, 2>(&prepared_actor);
                            }
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<Stats>> for Stats {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Stats> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<Stats>> for Stats {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Stats>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<Stats> for Stats {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Stats> {
                    Stats::create(builder, self.actor)
                }
            }

            /// Builder for serializing an instance of the [Stats] type.
            ///
            /// Can be created using the [Stats::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct StatsBuilder<State>(State);

            impl StatsBuilder<()> {
                /// Setter for the [`actor` field](Stats#structfield.actor).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn actor<T0>(self, value: T0) -> StatsBuilder<(T0,)>
                where
                    T0: ::planus::WriteAsDefault<u16, u16>,
                {
                    StatsBuilder((value,))
                }

                /// Sets the [`actor` field](Stats#structfield.actor) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn actor_as_default(self) -> StatsBuilder<(::planus::DefaultValue,)> {
                    self.actor(::planus::DefaultValue)
                }
            }

            impl<T0> StatsBuilder<(T0,)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [Stats].
                #[inline]
                pub fn finish(self, builder: &mut ::planus::Builder) -> ::planus::Offset<Stats>
                where
                    Self: ::planus::WriteAsOffset<Stats>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<T0: ::planus::WriteAsDefault<u16, u16>> ::planus::WriteAs<::planus::Offset<Stats>>
                for StatsBuilder<(T0,)>
            {
                type Prepared = ::planus::Offset<Stats>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Stats> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<T0: ::planus::WriteAsDefault<u16, u16>>
                ::planus::WriteAsOptional<::planus::Offset<Stats>> for StatsBuilder<(T0,)>
            {
                type Prepared = ::planus::Offset<Stats>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Stats>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<T0: ::planus::WriteAsDefault<u16, u16>> ::planus::WriteAsOffset<Stats>
                for StatsBuilder<(T0,)>
            {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Stats> {
                    let (v0,) = &self.0;
                    Stats::create(builder, v0)
                }
            }

            /// Reference to a deserialized [Stats].
            #[derive(Copy, Clone)]
            pub struct StatsRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> StatsRef<'a> {
                /// Getter for the [`actor` field](Stats#structfield.actor).
                #[inline]
                pub fn actor(&self) -> ::planus::Result<u16> {
                    ::core::result::Result::Ok(self.0.access(0, "Stats", "actor")?.unwrap_or(0))
                }
            }

            impl<'a> ::core::fmt::Debug for StatsRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("StatsRef");
                    f.field("actor", &self.actor());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<StatsRef<'a>> for Stats {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: StatsRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        actor: ::core::convert::TryInto::try_into(value.actor()?)?,
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for StatsRef<'a> {
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

            impl<'a> ::planus::VectorReadInner<'a> for StatsRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[StatsRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<Stats>> for Stats {
                type Value = ::planus::Offset<Stats>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<Stats>],
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

            impl<'a> ::planus::ReadAsRoot<'a> for StatsRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[StatsRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `LatencyHistograms` in the namespace `hypermind.protocol`
            ///
            /// Generated from these locations:
            /// * Table `LatencyHistograms` in the file `schemas/protocol.fbs:96`
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
            pub struct LatencyHistograms {}

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for LatencyHistograms {
                fn default() -> Self {
                    Self {}
                }
            }

            impl LatencyHistograms {
                /// Creates a [LatencyHistogramsBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> LatencyHistogramsBuilder<()> {
                    LatencyHistogramsBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(builder: &mut ::planus::Builder) -> ::planus::Offset<Self> {
                    let table_writer: ::planus::table_writer::TableWriter<4> =
                        ::core::default::Default::default();
                    unsafe {
                        table_writer.finish(builder, |_table_writer| {});
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<LatencyHistograms>> for LatencyHistograms {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<LatencyHistograms> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<LatencyHistograms>> for LatencyHistograms {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<LatencyHistograms>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<LatencyHistograms> for LatencyHistograms {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<LatencyHistograms> {
                    LatencyHistograms::create(builder)
                }
            }

            /// Builder for serializing an instance of the [LatencyHistograms] type.
            ///
            /// Can be created using the [LatencyHistograms::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct LatencyHistogramsBuilder<State>(State);

            impl LatencyHistogramsBuilder<()> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [LatencyHistograms].
                #[inline]
                pub fn finish(
                    self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<LatencyHistograms>
                where
                    Self: ::planus::WriteAsOffset<LatencyHistograms>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl ::planus::WriteAs<::planus::Offset<LatencyHistograms>> for LatencyHistogramsBuilder<()> {
                type Prepared = ::planus::Offset<LatencyHistograms>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<LatencyHistograms> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<LatencyHistograms>>
                for LatencyHistogramsBuilder<()>
            {
                type Prepared = ::planus::Offset<LatencyHistograms>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<LatencyHistograms>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<LatencyHistograms> for LatencyHistogramsBuilder<()> {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<LatencyHistograms> {
                    LatencyHistograms::create(builder)
                }
            }

            /// Reference to a deserialized [LatencyHistograms].
            #[derive(Copy, Clone)]
            pub struct LatencyHistogramsRef<'a>(
                #[allow(dead_code)] ::planus::table_reader::Table<'a>,
            );

            impl<'a> LatencyHistogramsRef<'a> {}

            impl<'a> ::core::fmt::Debug for LatencyHistogramsRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("LatencyHistogramsRef");

                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<LatencyHistogramsRef<'a>> for LatencyHistograms {
                type Error = ::planus::Error;

                fn try_from(_value: LatencyHistogramsRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {})
                }
            }

            impl<'a> ::planus::TableRead<'a> for LatencyHistogramsRef<'a> {
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

            impl<'a> ::planus::VectorReadInner<'a> for LatencyHistogramsRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[LatencyHistogramsRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<LatencyHistograms>> for LatencyHistograms {
                type Value = ::planus::Offset<LatencyHistograms>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<LatencyHistograms>],
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

            impl<'a> ::planus::ReadAsRoot<'a> for LatencyHistogramsRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[LatencyHistogramsRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `VerifyStatus` in the namespace `hypermind.protocol`
            ///
            /// Generated from these locations:
            /// * Table `VerifyStatus` in the file `schemas/protocol.fbs:98`
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
            pub struct VerifyStatus {
                /// The field `actor` in the table `VerifyStatus`
                pub actor: u16,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for VerifyStatus {
                fn default() -> Self {
                    Self { actor: 0 }
                }
            }

            impl VerifyStatus {
                /// Creates a [VerifyStatusBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> VerifyStatusBuilder<()> {
                    VerifyStatusBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_actor: impl ::planus::WriteAsDefault<u16, u16>,
                ) -> ::planus::Offset<Self> {
                    let prepared_actor = field_actor.prepare(builder, &0);

                    let mut table_writer: ::planus::table_writer::TableWriter<6> =
                        ::core::default::Default::default();
                    if prepared_actor.is_some() {
                        table_writer.write_entry::<u16>(0);
                    }

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            if let ::core::option::Option::Some(prepared_actor) = prepared_actor {
                                object_writer.write::<_, _, 2>(&prepared_actor);
                            }
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<VerifyStatus>> for VerifyStatus {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<VerifyStatus> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<VerifyStatus>> for VerifyStatus {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<VerifyStatus>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<VerifyStatus> for VerifyStatus {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<VerifyStatus> {
                    VerifyStatus::create(builder, self.actor)
                }
            }

            /// Builder for serializing an instance of the [VerifyStatus] type.
            ///
            /// Can be created using the [VerifyStatus::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct VerifyStatusBuilder<State>(State);

            impl VerifyStatusBuilder<()> {
                /// Setter for the [`actor` field](VerifyStatus#structfield.actor).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn actor<T0>(self, value: T0) -> VerifyStatusBuilder<(T0,)>
                where
                    T0: ::planus::WriteAsDefault<u16, u16>,
                {
                    VerifyStatusBuilder((value,))
                }

                /// Sets the [`actor` field](VerifyStatus#structfield.actor) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn actor_as_default(self) -> VerifyStatusBuilder<(::planus::DefaultValue,)> {
                    self.actor(::planus::DefaultValue)
                }
            }

            impl<T0> VerifyStatusBuilder<(T0,)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [VerifyStatus].
                #[inline]
                pub fn finish(
                    self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<VerifyStatus>
                where
                    Self: ::planus::WriteAsOffset<VerifyStatus>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<T0: ::planus::WriteAsDefault<u16, u16>>
                ::planus::WriteAs<::planus::Offset<VerifyStatus>> for VerifyStatusBuilder<(T0,)>
            {
                type Prepared = ::planus::Offset<VerifyStatus>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<VerifyStatus> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<T0: ::planus::WriteAsDefault<u16, u16>>
                ::planus::WriteAsOptional<::planus::Offset<VerifyStatus>>
                for VerifyStatusBuilder<(T0,)>
            {
                type Prepared = ::planus::Offset<VerifyStatus>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<VerifyStatus>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<T0: ::planus::WriteAsDefault<u16, u16>> ::planus::WriteAsOffset<VerifyStatus>
                for VerifyStatusBuilder<(T0,)>
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<VerifyStatus> {
                    let (v0,) = &self.0;
                    VerifyStatus::create(builder, v0)
                }
            }

            /// Reference to a deserialized [VerifyStatus].
            #[derive(Copy, Clone)]
            pub struct VerifyStatusRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> VerifyStatusRef<'a> {
                /// Getter for the [`actor` field](VerifyStatus#structfield.actor).
                #[inline]
                pub fn actor(&self) -> ::planus::Result<u16> {
                    ::core::result::Result::Ok(
                        self.0.access(0, "VerifyStatus", "actor")?.unwrap_or(0),
                    )
                }
            }

            impl<'a> ::core::fmt::Debug for VerifyStatusRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("VerifyStatusRef");
                    f.field("actor", &self.actor());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<VerifyStatusRef<'a>> for VerifyStatus {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: VerifyStatusRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        actor: ::core::convert::TryInto::try_into(value.actor()?)?,
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for VerifyStatusRef<'a> {
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

            impl<'a> ::planus::VectorReadInner<'a> for VerifyStatusRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[VerifyStatusRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<VerifyStatus>> for VerifyStatus {
                type Value = ::planus::Offset<VerifyStatus>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<VerifyStatus>],
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

            impl<'a> ::planus::ReadAsRoot<'a> for VerifyStatusRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[VerifyStatusRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `RebuildProjection` in the namespace `hypermind.protocol`
            ///
            /// Generated from these locations:
            /// * Table `RebuildProjection` in the file `schemas/protocol.fbs:102`
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
            pub struct RebuildProjection {
                /// The field `actor` in the table `RebuildProjection`
                pub actor: u16,
                /// The field `name` in the table `RebuildProjection`
                pub name: ::planus::alloc::string::String,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for RebuildProjection {
                fn default() -> Self {
                    Self {
                        actor: 0,
                        name: ::core::default::Default::default(),
                    }
                }
            }

            impl RebuildProjection {
                /// Creates a [RebuildProjectionBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> RebuildProjectionBuilder<()> {
                    RebuildProjectionBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_actor: impl ::planus::WriteAsDefault<u16, u16>,
                    field_name: impl ::planus::WriteAs<::planus::Offset<str>>,
                ) -> ::planus::Offset<Self> {
                    let prepared_actor = field_actor.prepare(builder, &0);
                    let prepared_name = field_name.prepare(builder);

                    let mut table_writer: ::planus::table_writer::TableWriter<8> =
                        ::core::default::Default::default();
                    table_writer.write_entry::<::planus::Offset<str>>(1);
                    if prepared_actor.is_some() {
                        table_writer.write_entry::<u16>(0);
                    }

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            object_writer.write::<_, _, 4>(&prepared_name);
                            if let ::core::option::Option::Some(prepared_actor) = prepared_actor {
                                object_writer.write::<_, _, 2>(&prepared_actor);
                            }
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<RebuildProjection>> for RebuildProjection {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<RebuildProjection> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<RebuildProjection>> for RebuildProjection {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<RebuildProjection>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<RebuildProjection> for RebuildProjection {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<RebuildProjection> {
                    RebuildProjection::create(builder, self.actor, &self.name)
                }
            }

            /// Builder for serializing an instance of the [RebuildProjection] type.
            ///
            /// Can be created using the [RebuildProjection::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct RebuildProjectionBuilder<State>(State);

            impl RebuildProjectionBuilder<()> {
                /// Setter for the [`actor` field](RebuildProjection#structfield.actor).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn actor<T0>(self, value: T0) -> RebuildProjectionBuilder<(T0,)>
                where
                    T0: ::planus::WriteAsDefault<u16, u16>,
                {
                    RebuildProjectionBuilder((value,))
                }

                /// Sets the [`actor` field](RebuildProjection#structfield.actor) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn actor_as_default(
                    self,
                ) -> RebuildProjectionBuilder<(::planus::DefaultValue,)> {
                    self.actor(::planus::DefaultValue)
                }
            }

            impl<T0> RebuildProjectionBuilder<(T0,)> {
                /// Setter for the [`name` field](RebuildProjection#structfield.name).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn name<T1>(self, value: T1) -> RebuildProjectionBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAs<::planus::Offset<str>>,
                {
                    let (v0,) = self.0;
                    RebuildProjectionBuilder((v0, value))
                }
            }

            impl<T0, T1> RebuildProjectionBuilder<(T0, T1)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [RebuildProjection].
                #[inline]
                pub fn finish(
                    self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<RebuildProjection>
                where
                    Self: ::planus::WriteAsOffset<RebuildProjection>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u16, u16>,
                T1: ::planus::WriteAs<::planus::Offset<str>>,
            > ::planus::WriteAs<::planus::Offset<RebuildProjection>>
                for RebuildProjectionBuilder<(T0, T1)>
            {
                type Prepared = ::planus::Offset<RebuildProjection>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<RebuildProjection> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u16, u16>,
                T1: ::planus::WriteAs<::planus::Offset<str>>,
            > ::planus::WriteAsOptional<::planus::Offset<RebuildProjection>>
                for RebuildProjectionBuilder<(T0, T1)>
            {
                type Prepared = ::planus::Offset<RebuildProjection>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<RebuildProjection>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u16, u16>,
                T1: ::planus::WriteAs<::planus::Offset<str>>,
            > ::planus::WriteAsOffset<RebuildProjection> for RebuildProjectionBuilder<(T0, T1)>
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<RebuildProjection> {
                    let (v0, v1) = &self.0;
                    RebuildProjection::create(builder, v0, v1)
                }
            }

            /// Reference to a deserialized [RebuildProjection].
            #[derive(Copy, Clone)]
            pub struct RebuildProjectionRef<'a>(
                #[allow(dead_code)] ::planus::table_reader::Table<'a>,
            );

            impl<'a> RebuildProjectionRef<'a> {
                /// Getter for the [`actor` field](RebuildProjection#structfield.actor).
                #[inline]
                pub fn actor(&self) -> ::planus::Result<u16> {
                    ::core::result::Result::Ok(
                        self.0.access(0, "RebuildProjection", "actor")?.unwrap_or(0),
                    )
                }

                /// Getter for the [`name` field](RebuildProjection#structfield.name).
                #[inline]
                pub fn name(&self) -> ::planus::Result<&'a ::core::primitive::str> {
                    self.0.access_required(1, "RebuildProjection", "name")
                }
            }

            impl<'a> ::core::fmt::Debug for RebuildProjectionRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("RebuildProjectionRef");
                    f.field("actor", &self.actor());
                    f.field("name", &self.name());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<RebuildProjectionRef<'a>> for RebuildProjection {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: RebuildProjectionRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        actor: ::core::convert::TryInto::try_into(value.actor()?)?,
                        name: ::core::convert::Into::into(value.name()?),
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for RebuildProjectionRef<'a> {
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

            impl<'a> ::planus::VectorReadInner<'a> for RebuildProjectionRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[RebuildProjectionRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<RebuildProjection>> for RebuildProjection {
                type Value = ::planus::Offset<RebuildProjection>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<RebuildProjection>],
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

            impl<'a> ::planus::ReadAsRoot<'a> for RebuildProjectionRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[RebuildProjectionRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `CryptoDelete` in the namespace `hypermind.protocol`
            ///
            /// Generated from these locations:
            /// * Table `CryptoDelete` in the file `schemas/protocol.fbs:107`
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
            pub struct CryptoDelete {
                /// The field `actor` in the table `CryptoDelete`
                pub actor: u16,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for CryptoDelete {
                fn default() -> Self {
                    Self { actor: 0 }
                }
            }

            impl CryptoDelete {
                /// Creates a [CryptoDeleteBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> CryptoDeleteBuilder<()> {
                    CryptoDeleteBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_actor: impl ::planus::WriteAsDefault<u16, u16>,
                ) -> ::planus::Offset<Self> {
                    let prepared_actor = field_actor.prepare(builder, &0);

                    let mut table_writer: ::planus::table_writer::TableWriter<6> =
                        ::core::default::Default::default();
                    if prepared_actor.is_some() {
                        table_writer.write_entry::<u16>(0);
                    }

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            if let ::core::option::Option::Some(prepared_actor) = prepared_actor {
                                object_writer.write::<_, _, 2>(&prepared_actor);
                            }
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<CryptoDelete>> for CryptoDelete {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<CryptoDelete> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<CryptoDelete>> for CryptoDelete {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<CryptoDelete>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<CryptoDelete> for CryptoDelete {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<CryptoDelete> {
                    CryptoDelete::create(builder, self.actor)
                }
            }

            /// Builder for serializing an instance of the [CryptoDelete] type.
            ///
            /// Can be created using the [CryptoDelete::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct CryptoDeleteBuilder<State>(State);

            impl CryptoDeleteBuilder<()> {
                /// Setter for the [`actor` field](CryptoDelete#structfield.actor).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn actor<T0>(self, value: T0) -> CryptoDeleteBuilder<(T0,)>
                where
                    T0: ::planus::WriteAsDefault<u16, u16>,
                {
                    CryptoDeleteBuilder((value,))
                }

                /// Sets the [`actor` field](CryptoDelete#structfield.actor) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn actor_as_default(self) -> CryptoDeleteBuilder<(::planus::DefaultValue,)> {
                    self.actor(::planus::DefaultValue)
                }
            }

            impl<T0> CryptoDeleteBuilder<(T0,)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [CryptoDelete].
                #[inline]
                pub fn finish(
                    self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<CryptoDelete>
                where
                    Self: ::planus::WriteAsOffset<CryptoDelete>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<T0: ::planus::WriteAsDefault<u16, u16>>
                ::planus::WriteAs<::planus::Offset<CryptoDelete>> for CryptoDeleteBuilder<(T0,)>
            {
                type Prepared = ::planus::Offset<CryptoDelete>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<CryptoDelete> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<T0: ::planus::WriteAsDefault<u16, u16>>
                ::planus::WriteAsOptional<::planus::Offset<CryptoDelete>>
                for CryptoDeleteBuilder<(T0,)>
            {
                type Prepared = ::planus::Offset<CryptoDelete>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<CryptoDelete>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<T0: ::planus::WriteAsDefault<u16, u16>> ::planus::WriteAsOffset<CryptoDelete>
                for CryptoDeleteBuilder<(T0,)>
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<CryptoDelete> {
                    let (v0,) = &self.0;
                    CryptoDelete::create(builder, v0)
                }
            }

            /// Reference to a deserialized [CryptoDelete].
            #[derive(Copy, Clone)]
            pub struct CryptoDeleteRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> CryptoDeleteRef<'a> {
                /// Getter for the [`actor` field](CryptoDelete#structfield.actor).
                #[inline]
                pub fn actor(&self) -> ::planus::Result<u16> {
                    ::core::result::Result::Ok(
                        self.0.access(0, "CryptoDelete", "actor")?.unwrap_or(0),
                    )
                }
            }

            impl<'a> ::core::fmt::Debug for CryptoDeleteRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("CryptoDeleteRef");
                    f.field("actor", &self.actor());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<CryptoDeleteRef<'a>> for CryptoDelete {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: CryptoDeleteRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        actor: ::core::convert::TryInto::try_into(value.actor()?)?,
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for CryptoDeleteRef<'a> {
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

            impl<'a> ::planus::VectorReadInner<'a> for CryptoDeleteRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[CryptoDeleteRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<CryptoDelete>> for CryptoDelete {
                type Value = ::planus::Offset<CryptoDelete>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<CryptoDelete>],
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

            impl<'a> ::planus::ReadAsRoot<'a> for CryptoDeleteRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[CryptoDeleteRef]", "read_as_root", 0)
                    })
                }
            }

            /// The union `RequestPayload` in the namespace `hypermind.protocol`
            ///
            /// Generated from these locations:
            /// * Union `RequestPayload` in the file `schemas/protocol.fbs:111`
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
            pub enum RequestPayload {
                /// The variant of type `Append` in the union `RequestPayload`
                Append(::planus::alloc::boxed::Box<self::Append>),

                /// The variant of type `Activate` in the union `RequestPayload`
                Activate(::planus::alloc::boxed::Box<self::Activate>),

                /// The variant of type `Transcript` in the union `RequestPayload`
                Transcript(::planus::alloc::boxed::Box<self::Transcript>),

                /// The variant of type `Recall` in the union `RequestPayload`
                Recall(::planus::alloc::boxed::Box<self::Recall>),

                /// The variant of type `AsOf` in the union `RequestPayload`
                AsOf(::planus::alloc::boxed::Box<self::AsOf>),

                /// The variant of type `Checkpoint` in the union `RequestPayload`
                Checkpoint(::planus::alloc::boxed::Box<self::Checkpoint>),

                /// The variant of type `LatestCheckpoint` in the union `RequestPayload`
                LatestCheckpoint(::planus::alloc::boxed::Box<self::LatestCheckpoint>),

                /// The variant of type `Attest` in the union `RequestPayload`
                Attest(::planus::alloc::boxed::Box<self::Attest>),

                /// The variant of type `Subscribe` in the union `RequestPayload`
                Subscribe(::planus::alloc::boxed::Box<self::Subscribe>),

                /// The variant of type `Health` in the union `RequestPayload`
                Health(::planus::alloc::boxed::Box<self::Health>),

                /// The variant of type `Stats` in the union `RequestPayload`
                Stats(::planus::alloc::boxed::Box<self::Stats>),

                /// The variant of type `LatencyHistograms` in the union `RequestPayload`
                LatencyHistograms(::planus::alloc::boxed::Box<self::LatencyHistograms>),

                /// The variant of type `VerifyStatus` in the union `RequestPayload`
                VerifyStatus(::planus::alloc::boxed::Box<self::VerifyStatus>),

                /// The variant of type `RebuildProjection` in the union `RequestPayload`
                RebuildProjection(::planus::alloc::boxed::Box<self::RebuildProjection>),

                /// The variant of type `CryptoDelete` in the union `RequestPayload`
                CryptoDelete(::planus::alloc::boxed::Box<self::CryptoDelete>),
            }

            impl RequestPayload {
                /// Creates a [RequestPayloadBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> RequestPayloadBuilder<::planus::Uninitialized> {
                    RequestPayloadBuilder(::planus::Uninitialized)
                }

                #[inline]
                pub fn create_append(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::Append>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(1, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_activate(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::Activate>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(2, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_transcript(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::Transcript>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(3, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_recall(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::Recall>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(4, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_as_of(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::AsOf>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(5, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_checkpoint(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::Checkpoint>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(6, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_latest_checkpoint(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::LatestCheckpoint>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(7, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_attest(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::Attest>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(8, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_subscribe(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::Subscribe>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(9, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_health(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::Health>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(10, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_stats(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::Stats>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(11, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_latency_histograms(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::LatencyHistograms>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(12, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_verify_status(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::VerifyStatus>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(13, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_rebuild_projection(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::RebuildProjection>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(14, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_crypto_delete(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::CryptoDelete>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(15, value.prepare(builder).downcast())
                }
            }

            impl ::planus::WriteAsUnion<RequestPayload> for RequestPayload {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::UnionOffset<Self> {
                    match self {
                        Self::Append(value) => Self::create_append(builder, value),
                        Self::Activate(value) => Self::create_activate(builder, value),
                        Self::Transcript(value) => Self::create_transcript(builder, value),
                        Self::Recall(value) => Self::create_recall(builder, value),
                        Self::AsOf(value) => Self::create_as_of(builder, value),
                        Self::Checkpoint(value) => Self::create_checkpoint(builder, value),
                        Self::LatestCheckpoint(value) => {
                            Self::create_latest_checkpoint(builder, value)
                        }
                        Self::Attest(value) => Self::create_attest(builder, value),
                        Self::Subscribe(value) => Self::create_subscribe(builder, value),
                        Self::Health(value) => Self::create_health(builder, value),
                        Self::Stats(value) => Self::create_stats(builder, value),
                        Self::LatencyHistograms(value) => {
                            Self::create_latency_histograms(builder, value)
                        }
                        Self::VerifyStatus(value) => Self::create_verify_status(builder, value),
                        Self::RebuildProjection(value) => {
                            Self::create_rebuild_projection(builder, value)
                        }
                        Self::CryptoDelete(value) => Self::create_crypto_delete(builder, value),
                    }
                }
            }

            impl ::planus::WriteAsOptionalUnion<RequestPayload> for RequestPayload {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<Self>> {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }

            /// Builder for serializing an instance of the [RequestPayload] type.
            ///
            /// Can be created using the [RequestPayload::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct RequestPayloadBuilder<T>(T);

            impl RequestPayloadBuilder<::planus::Uninitialized> {
                /// Creates an instance of the [`Append` variant](RequestPayload#variant.Append).
                #[inline]
                pub fn append<T>(
                    self,
                    value: T,
                ) -> RequestPayloadBuilder<::planus::Initialized<1, T>>
                where
                    T: ::planus::WriteAsOffset<self::Append>,
                {
                    RequestPayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`Activate` variant](RequestPayload#variant.Activate).
                #[inline]
                pub fn activate<T>(
                    self,
                    value: T,
                ) -> RequestPayloadBuilder<::planus::Initialized<2, T>>
                where
                    T: ::planus::WriteAsOffset<self::Activate>,
                {
                    RequestPayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`Transcript` variant](RequestPayload#variant.Transcript).
                #[inline]
                pub fn transcript<T>(
                    self,
                    value: T,
                ) -> RequestPayloadBuilder<::planus::Initialized<3, T>>
                where
                    T: ::planus::WriteAsOffset<self::Transcript>,
                {
                    RequestPayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`Recall` variant](RequestPayload#variant.Recall).
                #[inline]
                pub fn recall<T>(
                    self,
                    value: T,
                ) -> RequestPayloadBuilder<::planus::Initialized<4, T>>
                where
                    T: ::planus::WriteAsOffset<self::Recall>,
                {
                    RequestPayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`AsOf` variant](RequestPayload#variant.AsOf).
                #[inline]
                pub fn as_of<T>(
                    self,
                    value: T,
                ) -> RequestPayloadBuilder<::planus::Initialized<5, T>>
                where
                    T: ::planus::WriteAsOffset<self::AsOf>,
                {
                    RequestPayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`Checkpoint` variant](RequestPayload#variant.Checkpoint).
                #[inline]
                pub fn checkpoint<T>(
                    self,
                    value: T,
                ) -> RequestPayloadBuilder<::planus::Initialized<6, T>>
                where
                    T: ::planus::WriteAsOffset<self::Checkpoint>,
                {
                    RequestPayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`LatestCheckpoint` variant](RequestPayload#variant.LatestCheckpoint).
                #[inline]
                pub fn latest_checkpoint<T>(
                    self,
                    value: T,
                ) -> RequestPayloadBuilder<::planus::Initialized<7, T>>
                where
                    T: ::planus::WriteAsOffset<self::LatestCheckpoint>,
                {
                    RequestPayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`Attest` variant](RequestPayload#variant.Attest).
                #[inline]
                pub fn attest<T>(
                    self,
                    value: T,
                ) -> RequestPayloadBuilder<::planus::Initialized<8, T>>
                where
                    T: ::planus::WriteAsOffset<self::Attest>,
                {
                    RequestPayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`Subscribe` variant](RequestPayload#variant.Subscribe).
                #[inline]
                pub fn subscribe<T>(
                    self,
                    value: T,
                ) -> RequestPayloadBuilder<::planus::Initialized<9, T>>
                where
                    T: ::planus::WriteAsOffset<self::Subscribe>,
                {
                    RequestPayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`Health` variant](RequestPayload#variant.Health).
                #[inline]
                pub fn health<T>(
                    self,
                    value: T,
                ) -> RequestPayloadBuilder<::planus::Initialized<10, T>>
                where
                    T: ::planus::WriteAsOffset<self::Health>,
                {
                    RequestPayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`Stats` variant](RequestPayload#variant.Stats).
                #[inline]
                pub fn stats<T>(
                    self,
                    value: T,
                ) -> RequestPayloadBuilder<::planus::Initialized<11, T>>
                where
                    T: ::planus::WriteAsOffset<self::Stats>,
                {
                    RequestPayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`LatencyHistograms` variant](RequestPayload#variant.LatencyHistograms).
                #[inline]
                pub fn latency_histograms<T>(
                    self,
                    value: T,
                ) -> RequestPayloadBuilder<::planus::Initialized<12, T>>
                where
                    T: ::planus::WriteAsOffset<self::LatencyHistograms>,
                {
                    RequestPayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`VerifyStatus` variant](RequestPayload#variant.VerifyStatus).
                #[inline]
                pub fn verify_status<T>(
                    self,
                    value: T,
                ) -> RequestPayloadBuilder<::planus::Initialized<13, T>>
                where
                    T: ::planus::WriteAsOffset<self::VerifyStatus>,
                {
                    RequestPayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`RebuildProjection` variant](RequestPayload#variant.RebuildProjection).
                #[inline]
                pub fn rebuild_projection<T>(
                    self,
                    value: T,
                ) -> RequestPayloadBuilder<::planus::Initialized<14, T>>
                where
                    T: ::planus::WriteAsOffset<self::RebuildProjection>,
                {
                    RequestPayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`CryptoDelete` variant](RequestPayload#variant.CryptoDelete).
                #[inline]
                pub fn crypto_delete<T>(
                    self,
                    value: T,
                ) -> RequestPayloadBuilder<::planus::Initialized<15, T>>
                where
                    T: ::planus::WriteAsOffset<self::CryptoDelete>,
                {
                    RequestPayloadBuilder(::planus::Initialized(value))
                }
            }

            impl<const N: u8, T> RequestPayloadBuilder<::planus::Initialized<N, T>> {
                /// Finish writing the builder to get an [UnionOffset](::planus::UnionOffset) to a serialized [RequestPayload].
                #[inline]
                pub fn finish(
                    self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<RequestPayload>
                where
                    Self: ::planus::WriteAsUnion<RequestPayload>,
                {
                    ::planus::WriteAsUnion::prepare(&self, builder)
                }
            }

            impl<T> ::planus::WriteAsUnion<RequestPayload>
                for RequestPayloadBuilder<::planus::Initialized<1, T>>
            where
                T: ::planus::WriteAsOffset<self::Append>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<RequestPayload> {
                    ::planus::UnionOffset::new(1, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<RequestPayload>
                for RequestPayloadBuilder<::planus::Initialized<1, T>>
            where
                T: ::planus::WriteAsOffset<self::Append>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<RequestPayload>> {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<RequestPayload>
                for RequestPayloadBuilder<::planus::Initialized<2, T>>
            where
                T: ::planus::WriteAsOffset<self::Activate>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<RequestPayload> {
                    ::planus::UnionOffset::new(2, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<RequestPayload>
                for RequestPayloadBuilder<::planus::Initialized<2, T>>
            where
                T: ::planus::WriteAsOffset<self::Activate>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<RequestPayload>> {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<RequestPayload>
                for RequestPayloadBuilder<::planus::Initialized<3, T>>
            where
                T: ::planus::WriteAsOffset<self::Transcript>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<RequestPayload> {
                    ::planus::UnionOffset::new(3, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<RequestPayload>
                for RequestPayloadBuilder<::planus::Initialized<3, T>>
            where
                T: ::planus::WriteAsOffset<self::Transcript>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<RequestPayload>> {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<RequestPayload>
                for RequestPayloadBuilder<::planus::Initialized<4, T>>
            where
                T: ::planus::WriteAsOffset<self::Recall>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<RequestPayload> {
                    ::planus::UnionOffset::new(4, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<RequestPayload>
                for RequestPayloadBuilder<::planus::Initialized<4, T>>
            where
                T: ::planus::WriteAsOffset<self::Recall>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<RequestPayload>> {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<RequestPayload>
                for RequestPayloadBuilder<::planus::Initialized<5, T>>
            where
                T: ::planus::WriteAsOffset<self::AsOf>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<RequestPayload> {
                    ::planus::UnionOffset::new(5, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<RequestPayload>
                for RequestPayloadBuilder<::planus::Initialized<5, T>>
            where
                T: ::planus::WriteAsOffset<self::AsOf>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<RequestPayload>> {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<RequestPayload>
                for RequestPayloadBuilder<::planus::Initialized<6, T>>
            where
                T: ::planus::WriteAsOffset<self::Checkpoint>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<RequestPayload> {
                    ::planus::UnionOffset::new(6, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<RequestPayload>
                for RequestPayloadBuilder<::planus::Initialized<6, T>>
            where
                T: ::planus::WriteAsOffset<self::Checkpoint>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<RequestPayload>> {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<RequestPayload>
                for RequestPayloadBuilder<::planus::Initialized<7, T>>
            where
                T: ::planus::WriteAsOffset<self::LatestCheckpoint>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<RequestPayload> {
                    ::planus::UnionOffset::new(7, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<RequestPayload>
                for RequestPayloadBuilder<::planus::Initialized<7, T>>
            where
                T: ::planus::WriteAsOffset<self::LatestCheckpoint>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<RequestPayload>> {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<RequestPayload>
                for RequestPayloadBuilder<::planus::Initialized<8, T>>
            where
                T: ::planus::WriteAsOffset<self::Attest>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<RequestPayload> {
                    ::planus::UnionOffset::new(8, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<RequestPayload>
                for RequestPayloadBuilder<::planus::Initialized<8, T>>
            where
                T: ::planus::WriteAsOffset<self::Attest>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<RequestPayload>> {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<RequestPayload>
                for RequestPayloadBuilder<::planus::Initialized<9, T>>
            where
                T: ::planus::WriteAsOffset<self::Subscribe>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<RequestPayload> {
                    ::planus::UnionOffset::new(9, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<RequestPayload>
                for RequestPayloadBuilder<::planus::Initialized<9, T>>
            where
                T: ::planus::WriteAsOffset<self::Subscribe>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<RequestPayload>> {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<RequestPayload>
                for RequestPayloadBuilder<::planus::Initialized<10, T>>
            where
                T: ::planus::WriteAsOffset<self::Health>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<RequestPayload> {
                    ::planus::UnionOffset::new(10, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<RequestPayload>
                for RequestPayloadBuilder<::planus::Initialized<10, T>>
            where
                T: ::planus::WriteAsOffset<self::Health>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<RequestPayload>> {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<RequestPayload>
                for RequestPayloadBuilder<::planus::Initialized<11, T>>
            where
                T: ::planus::WriteAsOffset<self::Stats>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<RequestPayload> {
                    ::planus::UnionOffset::new(11, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<RequestPayload>
                for RequestPayloadBuilder<::planus::Initialized<11, T>>
            where
                T: ::planus::WriteAsOffset<self::Stats>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<RequestPayload>> {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<RequestPayload>
                for RequestPayloadBuilder<::planus::Initialized<12, T>>
            where
                T: ::planus::WriteAsOffset<self::LatencyHistograms>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<RequestPayload> {
                    ::planus::UnionOffset::new(12, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<RequestPayload>
                for RequestPayloadBuilder<::planus::Initialized<12, T>>
            where
                T: ::planus::WriteAsOffset<self::LatencyHistograms>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<RequestPayload>> {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<RequestPayload>
                for RequestPayloadBuilder<::planus::Initialized<13, T>>
            where
                T: ::planus::WriteAsOffset<self::VerifyStatus>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<RequestPayload> {
                    ::planus::UnionOffset::new(13, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<RequestPayload>
                for RequestPayloadBuilder<::planus::Initialized<13, T>>
            where
                T: ::planus::WriteAsOffset<self::VerifyStatus>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<RequestPayload>> {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<RequestPayload>
                for RequestPayloadBuilder<::planus::Initialized<14, T>>
            where
                T: ::planus::WriteAsOffset<self::RebuildProjection>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<RequestPayload> {
                    ::planus::UnionOffset::new(14, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<RequestPayload>
                for RequestPayloadBuilder<::planus::Initialized<14, T>>
            where
                T: ::planus::WriteAsOffset<self::RebuildProjection>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<RequestPayload>> {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<RequestPayload>
                for RequestPayloadBuilder<::planus::Initialized<15, T>>
            where
                T: ::planus::WriteAsOffset<self::CryptoDelete>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<RequestPayload> {
                    ::planus::UnionOffset::new(15, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<RequestPayload>
                for RequestPayloadBuilder<::planus::Initialized<15, T>>
            where
                T: ::planus::WriteAsOffset<self::CryptoDelete>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<RequestPayload>> {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }

            /// Reference to a deserialized [RequestPayload].
            #[derive(Copy, Clone, Debug)]
            pub enum RequestPayloadRef<'a> {
                Append(self::AppendRef<'a>),
                Activate(self::ActivateRef<'a>),
                Transcript(self::TranscriptRef<'a>),
                Recall(self::RecallRef<'a>),
                AsOf(self::AsOfRef<'a>),
                Checkpoint(self::CheckpointRef<'a>),
                LatestCheckpoint(self::LatestCheckpointRef<'a>),
                Attest(self::AttestRef<'a>),
                Subscribe(self::SubscribeRef<'a>),
                Health(self::HealthRef<'a>),
                Stats(self::StatsRef<'a>),
                LatencyHistograms(self::LatencyHistogramsRef<'a>),
                VerifyStatus(self::VerifyStatusRef<'a>),
                RebuildProjection(self::RebuildProjectionRef<'a>),
                CryptoDelete(self::CryptoDeleteRef<'a>),
            }

            impl<'a> ::core::convert::TryFrom<RequestPayloadRef<'a>> for RequestPayload {
                type Error = ::planus::Error;

                fn try_from(value: RequestPayloadRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(match value {
                        RequestPayloadRef::Append(value) => {
                            Self::Append(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        RequestPayloadRef::Activate(value) => {
                            Self::Activate(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        RequestPayloadRef::Transcript(value) => {
                            Self::Transcript(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        RequestPayloadRef::Recall(value) => {
                            Self::Recall(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        RequestPayloadRef::AsOf(value) => {
                            Self::AsOf(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        RequestPayloadRef::Checkpoint(value) => {
                            Self::Checkpoint(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        RequestPayloadRef::LatestCheckpoint(value) => {
                            Self::LatestCheckpoint(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        RequestPayloadRef::Attest(value) => {
                            Self::Attest(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        RequestPayloadRef::Subscribe(value) => {
                            Self::Subscribe(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        RequestPayloadRef::Health(value) => {
                            Self::Health(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        RequestPayloadRef::Stats(value) => {
                            Self::Stats(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        RequestPayloadRef::LatencyHistograms(value) => {
                            Self::LatencyHistograms(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        RequestPayloadRef::VerifyStatus(value) => {
                            Self::VerifyStatus(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        RequestPayloadRef::RebuildProjection(value) => {
                            Self::RebuildProjection(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        RequestPayloadRef::CryptoDelete(value) => {
                            Self::CryptoDelete(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }
                    })
                }
            }

            impl<'a> ::planus::TableReadUnion<'a> for RequestPayloadRef<'a> {
                fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    tag: u8,
                    field_offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::ErrorKind> {
                    match tag {
                        1 => ::core::result::Result::Ok(Self::Append(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        2 => ::core::result::Result::Ok(Self::Activate(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        3 => ::core::result::Result::Ok(Self::Transcript(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        4 => ::core::result::Result::Ok(Self::Recall(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        5 => ::core::result::Result::Ok(Self::AsOf(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        6 => ::core::result::Result::Ok(Self::Checkpoint(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        7 => ::core::result::Result::Ok(Self::LatestCheckpoint(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        8 => ::core::result::Result::Ok(Self::Attest(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        9 => ::core::result::Result::Ok(Self::Subscribe(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        10 => ::core::result::Result::Ok(Self::Health(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        11 => ::core::result::Result::Ok(Self::Stats(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        12 => ::core::result::Result::Ok(Self::LatencyHistograms(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        13 => ::core::result::Result::Ok(Self::VerifyStatus(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        14 => ::core::result::Result::Ok(Self::RebuildProjection(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        15 => ::core::result::Result::Ok(Self::CryptoDelete(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        _ => ::core::result::Result::Err(
                            ::planus::errors::ErrorKind::UnknownUnionTag { tag },
                        ),
                    }
                }
            }

            impl<'a> ::planus::VectorReadUnion<'a> for RequestPayloadRef<'a> {
                const VECTOR_NAME: &'static str = "[RequestPayloadRef]";
            }

            /// The table `Request` in the namespace `hypermind.protocol`
            ///
            /// Generated from these locations:
            /// * Table `Request` in the file `schemas/protocol.fbs:129`
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
            pub struct Request {
                /// The field `request_id` in the table `Request`
                pub request_id: u64,
                /// The field `payload` in the table `Request`
                pub payload: self::RequestPayload,
            }

            impl Request {
                /// Creates a [RequestBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> RequestBuilder<()> {
                    RequestBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_request_id: impl ::planus::WriteAsDefault<u64, u64>,
                    field_payload: impl ::planus::WriteAsUnion<self::RequestPayload>,
                ) -> ::planus::Offset<Self> {
                    let prepared_request_id = field_request_id.prepare(builder, &0);
                    let prepared_payload = field_payload.prepare(builder);

                    let mut table_writer: ::planus::table_writer::TableWriter<10> =
                        ::core::default::Default::default();
                    if prepared_request_id.is_some() {
                        table_writer.write_entry::<u64>(0);
                    }
                    table_writer.write_entry::<::planus::Offset<self::RequestPayload>>(2);
                    table_writer.write_entry::<u8>(1);

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            if let ::core::option::Option::Some(prepared_request_id) =
                                prepared_request_id
                            {
                                object_writer.write::<_, _, 8>(&prepared_request_id);
                            }
                            object_writer.write::<_, _, 4>(&prepared_payload.offset());
                            object_writer.write::<_, _, 1>(&prepared_payload.tag());
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<Request>> for Request {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Request> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<Request>> for Request {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Request>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<Request> for Request {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Request> {
                    Request::create(builder, self.request_id, &self.payload)
                }
            }

            /// Builder for serializing an instance of the [Request] type.
            ///
            /// Can be created using the [Request::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct RequestBuilder<State>(State);

            impl RequestBuilder<()> {
                /// Setter for the [`request_id` field](Request#structfield.request_id).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn request_id<T0>(self, value: T0) -> RequestBuilder<(T0,)>
                where
                    T0: ::planus::WriteAsDefault<u64, u64>,
                {
                    RequestBuilder((value,))
                }

                /// Sets the [`request_id` field](Request#structfield.request_id) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn request_id_as_default(self) -> RequestBuilder<(::planus::DefaultValue,)> {
                    self.request_id(::planus::DefaultValue)
                }
            }

            impl<T0> RequestBuilder<(T0,)> {
                /// Setter for the [`payload` field](Request#structfield.payload).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn payload<T1>(self, value: T1) -> RequestBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAsUnion<self::RequestPayload>,
                {
                    let (v0,) = self.0;
                    RequestBuilder((v0, value))
                }
            }

            impl<T0, T1> RequestBuilder<(T0, T1)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [Request].
                #[inline]
                pub fn finish(self, builder: &mut ::planus::Builder) -> ::planus::Offset<Request>
                where
                    Self: ::planus::WriteAsOffset<Request>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u64, u64>,
                T1: ::planus::WriteAsUnion<self::RequestPayload>,
            > ::planus::WriteAs<::planus::Offset<Request>> for RequestBuilder<(T0, T1)>
            {
                type Prepared = ::planus::Offset<Request>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Request> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u64, u64>,
                T1: ::planus::WriteAsUnion<self::RequestPayload>,
            > ::planus::WriteAsOptional<::planus::Offset<Request>> for RequestBuilder<(T0, T1)>
            {
                type Prepared = ::planus::Offset<Request>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Request>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u64, u64>,
                T1: ::planus::WriteAsUnion<self::RequestPayload>,
            > ::planus::WriteAsOffset<Request> for RequestBuilder<(T0, T1)>
            {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Request> {
                    let (v0, v1) = &self.0;
                    Request::create(builder, v0, v1)
                }
            }

            /// Reference to a deserialized [Request].
            #[derive(Copy, Clone)]
            pub struct RequestRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> RequestRef<'a> {
                /// Getter for the [`request_id` field](Request#structfield.request_id).
                #[inline]
                pub fn request_id(&self) -> ::planus::Result<u64> {
                    ::core::result::Result::Ok(
                        self.0.access(0, "Request", "request_id")?.unwrap_or(0),
                    )
                }

                /// Getter for the [`payload` field](Request#structfield.payload).
                #[inline]
                pub fn payload(&self) -> ::planus::Result<self::RequestPayloadRef<'a>> {
                    self.0.access_union_required(1, "Request", "payload")
                }
            }

            impl<'a> ::core::fmt::Debug for RequestRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("RequestRef");
                    f.field("request_id", &self.request_id());
                    f.field("payload", &self.payload());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<RequestRef<'a>> for Request {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: RequestRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        request_id: ::core::convert::TryInto::try_into(value.request_id()?)?,
                        payload: ::core::convert::TryInto::try_into(value.payload()?)?,
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for RequestRef<'a> {
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

            impl<'a> ::planus::VectorReadInner<'a> for RequestRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[RequestRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<Request>> for Request {
                type Value = ::planus::Offset<Request>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<Request>],
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

            impl<'a> ::planus::ReadAsRoot<'a> for RequestRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[RequestRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `ErrorDetail` in the namespace `hypermind.protocol`
            ///
            /// Generated from these locations:
            /// * Table `ErrorDetail` in the file `schemas/protocol.fbs:134`
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
            pub struct ErrorDetail {
                /// The field `code` in the table `ErrorDetail`
                pub code: u8,
                /// The field `system_error` in the table `ErrorDetail`
                pub system_error: i32,
                /// The field `lsn` in the table `ErrorDetail`
                pub lsn: u64,
                /// The field `offset` in the table `ErrorDetail`
                pub offset: u64,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for ErrorDetail {
                fn default() -> Self {
                    Self {
                        code: 0,
                        system_error: 0,
                        lsn: 0,
                        offset: 0,
                    }
                }
            }

            impl ErrorDetail {
                /// Creates a [ErrorDetailBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> ErrorDetailBuilder<()> {
                    ErrorDetailBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_code: impl ::planus::WriteAsDefault<u8, u8>,
                    field_system_error: impl ::planus::WriteAsDefault<i32, i32>,
                    field_lsn: impl ::planus::WriteAsDefault<u64, u64>,
                    field_offset: impl ::planus::WriteAsDefault<u64, u64>,
                ) -> ::planus::Offset<Self> {
                    let prepared_code = field_code.prepare(builder, &0);
                    let prepared_system_error = field_system_error.prepare(builder, &0);
                    let prepared_lsn = field_lsn.prepare(builder, &0);
                    let prepared_offset = field_offset.prepare(builder, &0);

                    let mut table_writer: ::planus::table_writer::TableWriter<12> =
                        ::core::default::Default::default();
                    if prepared_lsn.is_some() {
                        table_writer.write_entry::<u64>(2);
                    }
                    if prepared_offset.is_some() {
                        table_writer.write_entry::<u64>(3);
                    }
                    if prepared_system_error.is_some() {
                        table_writer.write_entry::<i32>(1);
                    }
                    if prepared_code.is_some() {
                        table_writer.write_entry::<u8>(0);
                    }

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            if let ::core::option::Option::Some(prepared_lsn) = prepared_lsn {
                                object_writer.write::<_, _, 8>(&prepared_lsn);
                            }
                            if let ::core::option::Option::Some(prepared_offset) = prepared_offset {
                                object_writer.write::<_, _, 8>(&prepared_offset);
                            }
                            if let ::core::option::Option::Some(prepared_system_error) =
                                prepared_system_error
                            {
                                object_writer.write::<_, _, 4>(&prepared_system_error);
                            }
                            if let ::core::option::Option::Some(prepared_code) = prepared_code {
                                object_writer.write::<_, _, 1>(&prepared_code);
                            }
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<ErrorDetail>> for ErrorDetail {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<ErrorDetail> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<ErrorDetail>> for ErrorDetail {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<ErrorDetail>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<ErrorDetail> for ErrorDetail {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<ErrorDetail> {
                    ErrorDetail::create(
                        builder,
                        self.code,
                        self.system_error,
                        self.lsn,
                        self.offset,
                    )
                }
            }

            /// Builder for serializing an instance of the [ErrorDetail] type.
            ///
            /// Can be created using the [ErrorDetail::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct ErrorDetailBuilder<State>(State);

            impl ErrorDetailBuilder<()> {
                /// Setter for the [`code` field](ErrorDetail#structfield.code).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn code<T0>(self, value: T0) -> ErrorDetailBuilder<(T0,)>
                where
                    T0: ::planus::WriteAsDefault<u8, u8>,
                {
                    ErrorDetailBuilder((value,))
                }

                /// Sets the [`code` field](ErrorDetail#structfield.code) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn code_as_default(self) -> ErrorDetailBuilder<(::planus::DefaultValue,)> {
                    self.code(::planus::DefaultValue)
                }
            }

            impl<T0> ErrorDetailBuilder<(T0,)> {
                /// Setter for the [`system_error` field](ErrorDetail#structfield.system_error).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn system_error<T1>(self, value: T1) -> ErrorDetailBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAsDefault<i32, i32>,
                {
                    let (v0,) = self.0;
                    ErrorDetailBuilder((v0, value))
                }

                /// Sets the [`system_error` field](ErrorDetail#structfield.system_error) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn system_error_as_default(
                    self,
                ) -> ErrorDetailBuilder<(T0, ::planus::DefaultValue)> {
                    self.system_error(::planus::DefaultValue)
                }
            }

            impl<T0, T1> ErrorDetailBuilder<(T0, T1)> {
                /// Setter for the [`lsn` field](ErrorDetail#structfield.lsn).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn lsn<T2>(self, value: T2) -> ErrorDetailBuilder<(T0, T1, T2)>
                where
                    T2: ::planus::WriteAsDefault<u64, u64>,
                {
                    let (v0, v1) = self.0;
                    ErrorDetailBuilder((v0, v1, value))
                }

                /// Sets the [`lsn` field](ErrorDetail#structfield.lsn) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn lsn_as_default(
                    self,
                ) -> ErrorDetailBuilder<(T0, T1, ::planus::DefaultValue)> {
                    self.lsn(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2> ErrorDetailBuilder<(T0, T1, T2)> {
                /// Setter for the [`offset` field](ErrorDetail#structfield.offset).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn offset<T3>(self, value: T3) -> ErrorDetailBuilder<(T0, T1, T2, T3)>
                where
                    T3: ::planus::WriteAsDefault<u64, u64>,
                {
                    let (v0, v1, v2) = self.0;
                    ErrorDetailBuilder((v0, v1, v2, value))
                }

                /// Sets the [`offset` field](ErrorDetail#structfield.offset) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn offset_as_default(
                    self,
                ) -> ErrorDetailBuilder<(T0, T1, T2, ::planus::DefaultValue)> {
                    self.offset(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2, T3> ErrorDetailBuilder<(T0, T1, T2, T3)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [ErrorDetail].
                #[inline]
                pub fn finish(
                    self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<ErrorDetail>
                where
                    Self: ::planus::WriteAsOffset<ErrorDetail>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u8, u8>,
                T1: ::planus::WriteAsDefault<i32, i32>,
                T2: ::planus::WriteAsDefault<u64, u64>,
                T3: ::planus::WriteAsDefault<u64, u64>,
            > ::planus::WriteAs<::planus::Offset<ErrorDetail>>
                for ErrorDetailBuilder<(T0, T1, T2, T3)>
            {
                type Prepared = ::planus::Offset<ErrorDetail>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<ErrorDetail> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u8, u8>,
                T1: ::planus::WriteAsDefault<i32, i32>,
                T2: ::planus::WriteAsDefault<u64, u64>,
                T3: ::planus::WriteAsDefault<u64, u64>,
            > ::planus::WriteAsOptional<::planus::Offset<ErrorDetail>>
                for ErrorDetailBuilder<(T0, T1, T2, T3)>
            {
                type Prepared = ::planus::Offset<ErrorDetail>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<ErrorDetail>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u8, u8>,
                T1: ::planus::WriteAsDefault<i32, i32>,
                T2: ::planus::WriteAsDefault<u64, u64>,
                T3: ::planus::WriteAsDefault<u64, u64>,
            > ::planus::WriteAsOffset<ErrorDetail> for ErrorDetailBuilder<(T0, T1, T2, T3)>
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<ErrorDetail> {
                    let (v0, v1, v2, v3) = &self.0;
                    ErrorDetail::create(builder, v0, v1, v2, v3)
                }
            }

            /// Reference to a deserialized [ErrorDetail].
            #[derive(Copy, Clone)]
            pub struct ErrorDetailRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> ErrorDetailRef<'a> {
                /// Getter for the [`code` field](ErrorDetail#structfield.code).
                #[inline]
                pub fn code(&self) -> ::planus::Result<u8> {
                    ::core::result::Result::Ok(
                        self.0.access(0, "ErrorDetail", "code")?.unwrap_or(0),
                    )
                }

                /// Getter for the [`system_error` field](ErrorDetail#structfield.system_error).
                #[inline]
                pub fn system_error(&self) -> ::planus::Result<i32> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(1, "ErrorDetail", "system_error")?
                            .unwrap_or(0),
                    )
                }

                /// Getter for the [`lsn` field](ErrorDetail#structfield.lsn).
                #[inline]
                pub fn lsn(&self) -> ::planus::Result<u64> {
                    ::core::result::Result::Ok(self.0.access(2, "ErrorDetail", "lsn")?.unwrap_or(0))
                }

                /// Getter for the [`offset` field](ErrorDetail#structfield.offset).
                #[inline]
                pub fn offset(&self) -> ::planus::Result<u64> {
                    ::core::result::Result::Ok(
                        self.0.access(3, "ErrorDetail", "offset")?.unwrap_or(0),
                    )
                }
            }

            impl<'a> ::core::fmt::Debug for ErrorDetailRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("ErrorDetailRef");
                    f.field("code", &self.code());
                    f.field("system_error", &self.system_error());
                    f.field("lsn", &self.lsn());
                    f.field("offset", &self.offset());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<ErrorDetailRef<'a>> for ErrorDetail {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: ErrorDetailRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        code: ::core::convert::TryInto::try_into(value.code()?)?,
                        system_error: ::core::convert::TryInto::try_into(value.system_error()?)?,
                        lsn: ::core::convert::TryInto::try_into(value.lsn()?)?,
                        offset: ::core::convert::TryInto::try_into(value.offset()?)?,
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for ErrorDetailRef<'a> {
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

            impl<'a> ::planus::VectorReadInner<'a> for ErrorDetailRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[ErrorDetailRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<ErrorDetail>> for ErrorDetail {
                type Value = ::planus::Offset<ErrorDetail>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<ErrorDetail>],
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

            impl<'a> ::planus::ReadAsRoot<'a> for ErrorDetailRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[ErrorDetailRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `AppendAck` in the namespace `hypermind.protocol`
            ///
            /// Generated from these locations:
            /// * Table `AppendAck` in the file `schemas/protocol.fbs:141`
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
            pub struct AppendAck {
                /// The field `client_seq` in the table `AppendAck`
                pub client_seq: u64,
                /// The field `first_lsn` in the table `AppendAck`
                pub first_lsn: u64,
                /// The field `last_lsn` in the table `AppendAck`
                pub last_lsn: u64,
                /// The field `duplicate` in the table `AppendAck`
                pub duplicate: bool,
                /// The field `leaf_count` in the table `AppendAck`
                pub leaf_count: u64,
                /// The field `last_leaf_hash` in the table `AppendAck`
                pub last_leaf_hash: ::core::option::Option<::planus::alloc::vec::Vec<u8>>,
                /// The field `mmr_root` in the table `AppendAck`
                pub mmr_root: ::core::option::Option<::planus::alloc::vec::Vec<u8>>,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for AppendAck {
                fn default() -> Self {
                    Self {
                        client_seq: 0,
                        first_lsn: 0,
                        last_lsn: 0,
                        duplicate: false,
                        leaf_count: 0,
                        last_leaf_hash: ::core::default::Default::default(),
                        mmr_root: ::core::default::Default::default(),
                    }
                }
            }

            impl AppendAck {
                /// Creates a [AppendAckBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> AppendAckBuilder<()> {
                    AppendAckBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_client_seq: impl ::planus::WriteAsDefault<u64, u64>,
                    field_first_lsn: impl ::planus::WriteAsDefault<u64, u64>,
                    field_last_lsn: impl ::planus::WriteAsDefault<u64, u64>,
                    field_duplicate: impl ::planus::WriteAsDefault<bool, bool>,
                    field_leaf_count: impl ::planus::WriteAsDefault<u64, u64>,
                    field_last_leaf_hash: impl ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                    field_mmr_root: impl ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                ) -> ::planus::Offset<Self> {
                    let prepared_client_seq = field_client_seq.prepare(builder, &0);
                    let prepared_first_lsn = field_first_lsn.prepare(builder, &0);
                    let prepared_last_lsn = field_last_lsn.prepare(builder, &0);
                    let prepared_duplicate = field_duplicate.prepare(builder, &false);
                    let prepared_leaf_count = field_leaf_count.prepare(builder, &0);
                    let prepared_last_leaf_hash = field_last_leaf_hash.prepare(builder);
                    let prepared_mmr_root = field_mmr_root.prepare(builder);

                    let mut table_writer: ::planus::table_writer::TableWriter<18> =
                        ::core::default::Default::default();
                    if prepared_client_seq.is_some() {
                        table_writer.write_entry::<u64>(0);
                    }
                    if prepared_first_lsn.is_some() {
                        table_writer.write_entry::<u64>(1);
                    }
                    if prepared_last_lsn.is_some() {
                        table_writer.write_entry::<u64>(2);
                    }
                    if prepared_leaf_count.is_some() {
                        table_writer.write_entry::<u64>(4);
                    }
                    if prepared_last_leaf_hash.is_some() {
                        table_writer.write_entry::<::planus::Offset<[u8]>>(5);
                    }
                    if prepared_mmr_root.is_some() {
                        table_writer.write_entry::<::planus::Offset<[u8]>>(6);
                    }
                    if prepared_duplicate.is_some() {
                        table_writer.write_entry::<bool>(3);
                    }

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            if let ::core::option::Option::Some(prepared_client_seq) =
                                prepared_client_seq
                            {
                                object_writer.write::<_, _, 8>(&prepared_client_seq);
                            }
                            if let ::core::option::Option::Some(prepared_first_lsn) =
                                prepared_first_lsn
                            {
                                object_writer.write::<_, _, 8>(&prepared_first_lsn);
                            }
                            if let ::core::option::Option::Some(prepared_last_lsn) =
                                prepared_last_lsn
                            {
                                object_writer.write::<_, _, 8>(&prepared_last_lsn);
                            }
                            if let ::core::option::Option::Some(prepared_leaf_count) =
                                prepared_leaf_count
                            {
                                object_writer.write::<_, _, 8>(&prepared_leaf_count);
                            }
                            if let ::core::option::Option::Some(prepared_last_leaf_hash) =
                                prepared_last_leaf_hash
                            {
                                object_writer.write::<_, _, 4>(&prepared_last_leaf_hash);
                            }
                            if let ::core::option::Option::Some(prepared_mmr_root) =
                                prepared_mmr_root
                            {
                                object_writer.write::<_, _, 4>(&prepared_mmr_root);
                            }
                            if let ::core::option::Option::Some(prepared_duplicate) =
                                prepared_duplicate
                            {
                                object_writer.write::<_, _, 1>(&prepared_duplicate);
                            }
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<AppendAck>> for AppendAck {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<AppendAck> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<AppendAck>> for AppendAck {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<AppendAck>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<AppendAck> for AppendAck {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<AppendAck> {
                    AppendAck::create(
                        builder,
                        self.client_seq,
                        self.first_lsn,
                        self.last_lsn,
                        self.duplicate,
                        self.leaf_count,
                        &self.last_leaf_hash,
                        &self.mmr_root,
                    )
                }
            }

            /// Builder for serializing an instance of the [AppendAck] type.
            ///
            /// Can be created using the [AppendAck::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct AppendAckBuilder<State>(State);

            impl AppendAckBuilder<()> {
                /// Setter for the [`client_seq` field](AppendAck#structfield.client_seq).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn client_seq<T0>(self, value: T0) -> AppendAckBuilder<(T0,)>
                where
                    T0: ::planus::WriteAsDefault<u64, u64>,
                {
                    AppendAckBuilder((value,))
                }

                /// Sets the [`client_seq` field](AppendAck#structfield.client_seq) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn client_seq_as_default(self) -> AppendAckBuilder<(::planus::DefaultValue,)> {
                    self.client_seq(::planus::DefaultValue)
                }
            }

            impl<T0> AppendAckBuilder<(T0,)> {
                /// Setter for the [`first_lsn` field](AppendAck#structfield.first_lsn).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn first_lsn<T1>(self, value: T1) -> AppendAckBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAsDefault<u64, u64>,
                {
                    let (v0,) = self.0;
                    AppendAckBuilder((v0, value))
                }

                /// Sets the [`first_lsn` field](AppendAck#structfield.first_lsn) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn first_lsn_as_default(
                    self,
                ) -> AppendAckBuilder<(T0, ::planus::DefaultValue)> {
                    self.first_lsn(::planus::DefaultValue)
                }
            }

            impl<T0, T1> AppendAckBuilder<(T0, T1)> {
                /// Setter for the [`last_lsn` field](AppendAck#structfield.last_lsn).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn last_lsn<T2>(self, value: T2) -> AppendAckBuilder<(T0, T1, T2)>
                where
                    T2: ::planus::WriteAsDefault<u64, u64>,
                {
                    let (v0, v1) = self.0;
                    AppendAckBuilder((v0, v1, value))
                }

                /// Sets the [`last_lsn` field](AppendAck#structfield.last_lsn) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn last_lsn_as_default(
                    self,
                ) -> AppendAckBuilder<(T0, T1, ::planus::DefaultValue)> {
                    self.last_lsn(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2> AppendAckBuilder<(T0, T1, T2)> {
                /// Setter for the [`duplicate` field](AppendAck#structfield.duplicate).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn duplicate<T3>(self, value: T3) -> AppendAckBuilder<(T0, T1, T2, T3)>
                where
                    T3: ::planus::WriteAsDefault<bool, bool>,
                {
                    let (v0, v1, v2) = self.0;
                    AppendAckBuilder((v0, v1, v2, value))
                }

                /// Sets the [`duplicate` field](AppendAck#structfield.duplicate) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn duplicate_as_default(
                    self,
                ) -> AppendAckBuilder<(T0, T1, T2, ::planus::DefaultValue)> {
                    self.duplicate(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2, T3> AppendAckBuilder<(T0, T1, T2, T3)> {
                /// Setter for the [`leaf_count` field](AppendAck#structfield.leaf_count).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn leaf_count<T4>(self, value: T4) -> AppendAckBuilder<(T0, T1, T2, T3, T4)>
                where
                    T4: ::planus::WriteAsDefault<u64, u64>,
                {
                    let (v0, v1, v2, v3) = self.0;
                    AppendAckBuilder((v0, v1, v2, v3, value))
                }

                /// Sets the [`leaf_count` field](AppendAck#structfield.leaf_count) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn leaf_count_as_default(
                    self,
                ) -> AppendAckBuilder<(T0, T1, T2, T3, ::planus::DefaultValue)> {
                    self.leaf_count(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2, T3, T4> AppendAckBuilder<(T0, T1, T2, T3, T4)> {
                /// Setter for the [`last_leaf_hash` field](AppendAck#structfield.last_leaf_hash).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn last_leaf_hash<T5>(
                    self,
                    value: T5,
                ) -> AppendAckBuilder<(T0, T1, T2, T3, T4, T5)>
                where
                    T5: ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                {
                    let (v0, v1, v2, v3, v4) = self.0;
                    AppendAckBuilder((v0, v1, v2, v3, v4, value))
                }

                /// Sets the [`last_leaf_hash` field](AppendAck#structfield.last_leaf_hash) to null.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn last_leaf_hash_as_null(self) -> AppendAckBuilder<(T0, T1, T2, T3, T4, ())> {
                    self.last_leaf_hash(())
                }
            }

            impl<T0, T1, T2, T3, T4, T5> AppendAckBuilder<(T0, T1, T2, T3, T4, T5)> {
                /// Setter for the [`mmr_root` field](AppendAck#structfield.mmr_root).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn mmr_root<T6>(
                    self,
                    value: T6,
                ) -> AppendAckBuilder<(T0, T1, T2, T3, T4, T5, T6)>
                where
                    T6: ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                {
                    let (v0, v1, v2, v3, v4, v5) = self.0;
                    AppendAckBuilder((v0, v1, v2, v3, v4, v5, value))
                }

                /// Sets the [`mmr_root` field](AppendAck#structfield.mmr_root) to null.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn mmr_root_as_null(self) -> AppendAckBuilder<(T0, T1, T2, T3, T4, T5, ())> {
                    self.mmr_root(())
                }
            }

            impl<T0, T1, T2, T3, T4, T5, T6> AppendAckBuilder<(T0, T1, T2, T3, T4, T5, T6)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [AppendAck].
                #[inline]
                pub fn finish(self, builder: &mut ::planus::Builder) -> ::planus::Offset<AppendAck>
                where
                    Self: ::planus::WriteAsOffset<AppendAck>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u64, u64>,
                T1: ::planus::WriteAsDefault<u64, u64>,
                T2: ::planus::WriteAsDefault<u64, u64>,
                T3: ::planus::WriteAsDefault<bool, bool>,
                T4: ::planus::WriteAsDefault<u64, u64>,
                T5: ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                T6: ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
            > ::planus::WriteAs<::planus::Offset<AppendAck>>
                for AppendAckBuilder<(T0, T1, T2, T3, T4, T5, T6)>
            {
                type Prepared = ::planus::Offset<AppendAck>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<AppendAck> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u64, u64>,
                T1: ::planus::WriteAsDefault<u64, u64>,
                T2: ::planus::WriteAsDefault<u64, u64>,
                T3: ::planus::WriteAsDefault<bool, bool>,
                T4: ::planus::WriteAsDefault<u64, u64>,
                T5: ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                T6: ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
            > ::planus::WriteAsOptional<::planus::Offset<AppendAck>>
                for AppendAckBuilder<(T0, T1, T2, T3, T4, T5, T6)>
            {
                type Prepared = ::planus::Offset<AppendAck>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<AppendAck>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u64, u64>,
                T1: ::planus::WriteAsDefault<u64, u64>,
                T2: ::planus::WriteAsDefault<u64, u64>,
                T3: ::planus::WriteAsDefault<bool, bool>,
                T4: ::planus::WriteAsDefault<u64, u64>,
                T5: ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                T6: ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
            > ::planus::WriteAsOffset<AppendAck>
                for AppendAckBuilder<(T0, T1, T2, T3, T4, T5, T6)>
            {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<AppendAck> {
                    let (v0, v1, v2, v3, v4, v5, v6) = &self.0;
                    AppendAck::create(builder, v0, v1, v2, v3, v4, v5, v6)
                }
            }

            /// Reference to a deserialized [AppendAck].
            #[derive(Copy, Clone)]
            pub struct AppendAckRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> AppendAckRef<'a> {
                /// Getter for the [`client_seq` field](AppendAck#structfield.client_seq).
                #[inline]
                pub fn client_seq(&self) -> ::planus::Result<u64> {
                    ::core::result::Result::Ok(
                        self.0.access(0, "AppendAck", "client_seq")?.unwrap_or(0),
                    )
                }

                /// Getter for the [`first_lsn` field](AppendAck#structfield.first_lsn).
                #[inline]
                pub fn first_lsn(&self) -> ::planus::Result<u64> {
                    ::core::result::Result::Ok(
                        self.0.access(1, "AppendAck", "first_lsn")?.unwrap_or(0),
                    )
                }

                /// Getter for the [`last_lsn` field](AppendAck#structfield.last_lsn).
                #[inline]
                pub fn last_lsn(&self) -> ::planus::Result<u64> {
                    ::core::result::Result::Ok(
                        self.0.access(2, "AppendAck", "last_lsn")?.unwrap_or(0),
                    )
                }

                /// Getter for the [`duplicate` field](AppendAck#structfield.duplicate).
                #[inline]
                pub fn duplicate(&self) -> ::planus::Result<bool> {
                    ::core::result::Result::Ok(
                        self.0.access(3, "AppendAck", "duplicate")?.unwrap_or(false),
                    )
                }

                /// Getter for the [`leaf_count` field](AppendAck#structfield.leaf_count).
                #[inline]
                pub fn leaf_count(&self) -> ::planus::Result<u64> {
                    ::core::result::Result::Ok(
                        self.0.access(4, "AppendAck", "leaf_count")?.unwrap_or(0),
                    )
                }

                /// Getter for the [`last_leaf_hash` field](AppendAck#structfield.last_leaf_hash).
                #[inline]
                pub fn last_leaf_hash(&self) -> ::planus::Result<::core::option::Option<&'a [u8]>> {
                    self.0.access(5, "AppendAck", "last_leaf_hash")
                }

                /// Getter for the [`mmr_root` field](AppendAck#structfield.mmr_root).
                #[inline]
                pub fn mmr_root(&self) -> ::planus::Result<::core::option::Option<&'a [u8]>> {
                    self.0.access(6, "AppendAck", "mmr_root")
                }
            }

            impl<'a> ::core::fmt::Debug for AppendAckRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("AppendAckRef");
                    f.field("client_seq", &self.client_seq());
                    f.field("first_lsn", &self.first_lsn());
                    f.field("last_lsn", &self.last_lsn());
                    f.field("duplicate", &self.duplicate());
                    f.field("leaf_count", &self.leaf_count());
                    if let ::core::option::Option::Some(field_last_leaf_hash) =
                        self.last_leaf_hash().transpose()
                    {
                        f.field("last_leaf_hash", &field_last_leaf_hash);
                    }
                    if let ::core::option::Option::Some(field_mmr_root) =
                        self.mmr_root().transpose()
                    {
                        f.field("mmr_root", &field_mmr_root);
                    }
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<AppendAckRef<'a>> for AppendAck {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: AppendAckRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        client_seq: ::core::convert::TryInto::try_into(value.client_seq()?)?,
                        first_lsn: ::core::convert::TryInto::try_into(value.first_lsn()?)?,
                        last_lsn: ::core::convert::TryInto::try_into(value.last_lsn()?)?,
                        duplicate: ::core::convert::TryInto::try_into(value.duplicate()?)?,
                        leaf_count: ::core::convert::TryInto::try_into(value.leaf_count()?)?,
                        last_leaf_hash: value.last_leaf_hash()?.map(|v| v.to_vec()),
                        mmr_root: value.mmr_root()?.map(|v| v.to_vec()),
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for AppendAckRef<'a> {
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

            impl<'a> ::planus::VectorReadInner<'a> for AppendAckRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[AppendAckRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<AppendAck>> for AppendAck {
                type Value = ::planus::Offset<AppendAck>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<AppendAck>],
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

            impl<'a> ::planus::ReadAsRoot<'a> for AppendAckRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[AppendAckRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `BytesResult` in the namespace `hypermind.protocol`
            ///
            /// Generated from these locations:
            /// * Table `BytesResult` in the file `schemas/protocol.fbs:151`
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
            pub struct BytesResult {
                /// The field `bytes` in the table `BytesResult`
                pub bytes: ::planus::alloc::vec::Vec<u8>,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for BytesResult {
                fn default() -> Self {
                    Self {
                        bytes: ::core::default::Default::default(),
                    }
                }
            }

            impl BytesResult {
                /// Creates a [BytesResultBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> BytesResultBuilder<()> {
                    BytesResultBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_bytes: impl ::planus::WriteAs<::planus::Offset<[u8]>>,
                ) -> ::planus::Offset<Self> {
                    let prepared_bytes = field_bytes.prepare(builder);

                    let mut table_writer: ::planus::table_writer::TableWriter<6> =
                        ::core::default::Default::default();
                    table_writer.write_entry::<::planus::Offset<[u8]>>(0);

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            object_writer.write::<_, _, 4>(&prepared_bytes);
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<BytesResult>> for BytesResult {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<BytesResult> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<BytesResult>> for BytesResult {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<BytesResult>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<BytesResult> for BytesResult {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<BytesResult> {
                    BytesResult::create(builder, &self.bytes)
                }
            }

            /// Builder for serializing an instance of the [BytesResult] type.
            ///
            /// Can be created using the [BytesResult::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct BytesResultBuilder<State>(State);

            impl BytesResultBuilder<()> {
                /// Setter for the [`bytes` field](BytesResult#structfield.bytes).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn bytes<T0>(self, value: T0) -> BytesResultBuilder<(T0,)>
                where
                    T0: ::planus::WriteAs<::planus::Offset<[u8]>>,
                {
                    BytesResultBuilder((value,))
                }
            }

            impl<T0> BytesResultBuilder<(T0,)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [BytesResult].
                #[inline]
                pub fn finish(
                    self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<BytesResult>
                where
                    Self: ::planus::WriteAsOffset<BytesResult>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<T0: ::planus::WriteAs<::planus::Offset<[u8]>>>
                ::planus::WriteAs<::planus::Offset<BytesResult>> for BytesResultBuilder<(T0,)>
            {
                type Prepared = ::planus::Offset<BytesResult>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<BytesResult> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<T0: ::planus::WriteAs<::planus::Offset<[u8]>>>
                ::planus::WriteAsOptional<::planus::Offset<BytesResult>>
                for BytesResultBuilder<(T0,)>
            {
                type Prepared = ::planus::Offset<BytesResult>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<BytesResult>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<T0: ::planus::WriteAs<::planus::Offset<[u8]>>> ::planus::WriteAsOffset<BytesResult>
                for BytesResultBuilder<(T0,)>
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<BytesResult> {
                    let (v0,) = &self.0;
                    BytesResult::create(builder, v0)
                }
            }

            /// Reference to a deserialized [BytesResult].
            #[derive(Copy, Clone)]
            pub struct BytesResultRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> BytesResultRef<'a> {
                /// Getter for the [`bytes` field](BytesResult#structfield.bytes).
                #[inline]
                pub fn bytes(&self) -> ::planus::Result<&'a [u8]> {
                    self.0.access_required(0, "BytesResult", "bytes")
                }
            }

            impl<'a> ::core::fmt::Debug for BytesResultRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("BytesResultRef");
                    f.field("bytes", &self.bytes());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<BytesResultRef<'a>> for BytesResult {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: BytesResultRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        bytes: value.bytes()?.to_vec(),
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for BytesResultRef<'a> {
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

            impl<'a> ::planus::VectorReadInner<'a> for BytesResultRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[BytesResultRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<BytesResult>> for BytesResult {
                type Value = ::planus::Offset<BytesResult>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<BytesResult>],
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

            impl<'a> ::planus::ReadAsRoot<'a> for BytesResultRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[BytesResultRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `SubscriptionAck` in the namespace `hypermind.protocol`
            ///
            /// Generated from these locations:
            /// * Table `SubscriptionAck` in the file `schemas/protocol.fbs:155`
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
            pub struct SubscriptionAck {
                /// The field `subscription_id` in the table `SubscriptionAck`
                pub subscription_id: u64,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for SubscriptionAck {
                fn default() -> Self {
                    Self { subscription_id: 0 }
                }
            }

            impl SubscriptionAck {
                /// Creates a [SubscriptionAckBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> SubscriptionAckBuilder<()> {
                    SubscriptionAckBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_subscription_id: impl ::planus::WriteAsDefault<u64, u64>,
                ) -> ::planus::Offset<Self> {
                    let prepared_subscription_id = field_subscription_id.prepare(builder, &0);

                    let mut table_writer: ::planus::table_writer::TableWriter<6> =
                        ::core::default::Default::default();
                    if prepared_subscription_id.is_some() {
                        table_writer.write_entry::<u64>(0);
                    }

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            if let ::core::option::Option::Some(prepared_subscription_id) =
                                prepared_subscription_id
                            {
                                object_writer.write::<_, _, 8>(&prepared_subscription_id);
                            }
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<SubscriptionAck>> for SubscriptionAck {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<SubscriptionAck> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<SubscriptionAck>> for SubscriptionAck {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<SubscriptionAck>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<SubscriptionAck> for SubscriptionAck {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<SubscriptionAck> {
                    SubscriptionAck::create(builder, self.subscription_id)
                }
            }

            /// Builder for serializing an instance of the [SubscriptionAck] type.
            ///
            /// Can be created using the [SubscriptionAck::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct SubscriptionAckBuilder<State>(State);

            impl SubscriptionAckBuilder<()> {
                /// Setter for the [`subscription_id` field](SubscriptionAck#structfield.subscription_id).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn subscription_id<T0>(self, value: T0) -> SubscriptionAckBuilder<(T0,)>
                where
                    T0: ::planus::WriteAsDefault<u64, u64>,
                {
                    SubscriptionAckBuilder((value,))
                }

                /// Sets the [`subscription_id` field](SubscriptionAck#structfield.subscription_id) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn subscription_id_as_default(
                    self,
                ) -> SubscriptionAckBuilder<(::planus::DefaultValue,)> {
                    self.subscription_id(::planus::DefaultValue)
                }
            }

            impl<T0> SubscriptionAckBuilder<(T0,)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [SubscriptionAck].
                #[inline]
                pub fn finish(
                    self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<SubscriptionAck>
                where
                    Self: ::planus::WriteAsOffset<SubscriptionAck>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<T0: ::planus::WriteAsDefault<u64, u64>>
                ::planus::WriteAs<::planus::Offset<SubscriptionAck>>
                for SubscriptionAckBuilder<(T0,)>
            {
                type Prepared = ::planus::Offset<SubscriptionAck>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<SubscriptionAck> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<T0: ::planus::WriteAsDefault<u64, u64>>
                ::planus::WriteAsOptional<::planus::Offset<SubscriptionAck>>
                for SubscriptionAckBuilder<(T0,)>
            {
                type Prepared = ::planus::Offset<SubscriptionAck>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<SubscriptionAck>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<T0: ::planus::WriteAsDefault<u64, u64>> ::planus::WriteAsOffset<SubscriptionAck>
                for SubscriptionAckBuilder<(T0,)>
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<SubscriptionAck> {
                    let (v0,) = &self.0;
                    SubscriptionAck::create(builder, v0)
                }
            }

            /// Reference to a deserialized [SubscriptionAck].
            #[derive(Copy, Clone)]
            pub struct SubscriptionAckRef<'a>(
                #[allow(dead_code)] ::planus::table_reader::Table<'a>,
            );

            impl<'a> SubscriptionAckRef<'a> {
                /// Getter for the [`subscription_id` field](SubscriptionAck#structfield.subscription_id).
                #[inline]
                pub fn subscription_id(&self) -> ::planus::Result<u64> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(0, "SubscriptionAck", "subscription_id")?
                            .unwrap_or(0),
                    )
                }
            }

            impl<'a> ::core::fmt::Debug for SubscriptionAckRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("SubscriptionAckRef");
                    f.field("subscription_id", &self.subscription_id());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<SubscriptionAckRef<'a>> for SubscriptionAck {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: SubscriptionAckRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        subscription_id: ::core::convert::TryInto::try_into(
                            value.subscription_id()?,
                        )?,
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for SubscriptionAckRef<'a> {
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

            impl<'a> ::planus::VectorReadInner<'a> for SubscriptionAckRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[SubscriptionAckRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<SubscriptionAck>> for SubscriptionAck {
                type Value = ::planus::Offset<SubscriptionAck>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<SubscriptionAck>],
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

            impl<'a> ::planus::ReadAsRoot<'a> for SubscriptionAckRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[SubscriptionAckRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `HealthResult` in the namespace `hypermind.protocol`
            ///
            /// Generated from these locations:
            /// * Table `HealthResult` in the file `schemas/protocol.fbs:159`
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
            pub struct HealthResult {
                /// The field `ready` in the table `HealthResult`
                pub ready: bool,
                /// The field `actor_count` in the table `HealthResult`
                pub actor_count: u32,
                /// The field `active_connections` in the table `HealthResult`
                pub active_connections: u32,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for HealthResult {
                fn default() -> Self {
                    Self {
                        ready: false,
                        actor_count: 0,
                        active_connections: 0,
                    }
                }
            }

            impl HealthResult {
                /// Creates a [HealthResultBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> HealthResultBuilder<()> {
                    HealthResultBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_ready: impl ::planus::WriteAsDefault<bool, bool>,
                    field_actor_count: impl ::planus::WriteAsDefault<u32, u32>,
                    field_active_connections: impl ::planus::WriteAsDefault<u32, u32>,
                ) -> ::planus::Offset<Self> {
                    let prepared_ready = field_ready.prepare(builder, &false);
                    let prepared_actor_count = field_actor_count.prepare(builder, &0);
                    let prepared_active_connections = field_active_connections.prepare(builder, &0);

                    let mut table_writer: ::planus::table_writer::TableWriter<10> =
                        ::core::default::Default::default();
                    if prepared_actor_count.is_some() {
                        table_writer.write_entry::<u32>(1);
                    }
                    if prepared_active_connections.is_some() {
                        table_writer.write_entry::<u32>(2);
                    }
                    if prepared_ready.is_some() {
                        table_writer.write_entry::<bool>(0);
                    }

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            if let ::core::option::Option::Some(prepared_actor_count) =
                                prepared_actor_count
                            {
                                object_writer.write::<_, _, 4>(&prepared_actor_count);
                            }
                            if let ::core::option::Option::Some(prepared_active_connections) =
                                prepared_active_connections
                            {
                                object_writer.write::<_, _, 4>(&prepared_active_connections);
                            }
                            if let ::core::option::Option::Some(prepared_ready) = prepared_ready {
                                object_writer.write::<_, _, 1>(&prepared_ready);
                            }
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<HealthResult>> for HealthResult {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<HealthResult> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<HealthResult>> for HealthResult {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<HealthResult>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<HealthResult> for HealthResult {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<HealthResult> {
                    HealthResult::create(
                        builder,
                        self.ready,
                        self.actor_count,
                        self.active_connections,
                    )
                }
            }

            /// Builder for serializing an instance of the [HealthResult] type.
            ///
            /// Can be created using the [HealthResult::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct HealthResultBuilder<State>(State);

            impl HealthResultBuilder<()> {
                /// Setter for the [`ready` field](HealthResult#structfield.ready).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn ready<T0>(self, value: T0) -> HealthResultBuilder<(T0,)>
                where
                    T0: ::planus::WriteAsDefault<bool, bool>,
                {
                    HealthResultBuilder((value,))
                }

                /// Sets the [`ready` field](HealthResult#structfield.ready) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn ready_as_default(self) -> HealthResultBuilder<(::planus::DefaultValue,)> {
                    self.ready(::planus::DefaultValue)
                }
            }

            impl<T0> HealthResultBuilder<(T0,)> {
                /// Setter for the [`actor_count` field](HealthResult#structfield.actor_count).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn actor_count<T1>(self, value: T1) -> HealthResultBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAsDefault<u32, u32>,
                {
                    let (v0,) = self.0;
                    HealthResultBuilder((v0, value))
                }

                /// Sets the [`actor_count` field](HealthResult#structfield.actor_count) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn actor_count_as_default(
                    self,
                ) -> HealthResultBuilder<(T0, ::planus::DefaultValue)> {
                    self.actor_count(::planus::DefaultValue)
                }
            }

            impl<T0, T1> HealthResultBuilder<(T0, T1)> {
                /// Setter for the [`active_connections` field](HealthResult#structfield.active_connections).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn active_connections<T2>(self, value: T2) -> HealthResultBuilder<(T0, T1, T2)>
                where
                    T2: ::planus::WriteAsDefault<u32, u32>,
                {
                    let (v0, v1) = self.0;
                    HealthResultBuilder((v0, v1, value))
                }

                /// Sets the [`active_connections` field](HealthResult#structfield.active_connections) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn active_connections_as_default(
                    self,
                ) -> HealthResultBuilder<(T0, T1, ::planus::DefaultValue)> {
                    self.active_connections(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2> HealthResultBuilder<(T0, T1, T2)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [HealthResult].
                #[inline]
                pub fn finish(
                    self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<HealthResult>
                where
                    Self: ::planus::WriteAsOffset<HealthResult>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<bool, bool>,
                T1: ::planus::WriteAsDefault<u32, u32>,
                T2: ::planus::WriteAsDefault<u32, u32>,
            > ::planus::WriteAs<::planus::Offset<HealthResult>>
                for HealthResultBuilder<(T0, T1, T2)>
            {
                type Prepared = ::planus::Offset<HealthResult>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<HealthResult> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<bool, bool>,
                T1: ::planus::WriteAsDefault<u32, u32>,
                T2: ::planus::WriteAsDefault<u32, u32>,
            > ::planus::WriteAsOptional<::planus::Offset<HealthResult>>
                for HealthResultBuilder<(T0, T1, T2)>
            {
                type Prepared = ::planus::Offset<HealthResult>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<HealthResult>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<bool, bool>,
                T1: ::planus::WriteAsDefault<u32, u32>,
                T2: ::planus::WriteAsDefault<u32, u32>,
            > ::planus::WriteAsOffset<HealthResult> for HealthResultBuilder<(T0, T1, T2)>
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<HealthResult> {
                    let (v0, v1, v2) = &self.0;
                    HealthResult::create(builder, v0, v1, v2)
                }
            }

            /// Reference to a deserialized [HealthResult].
            #[derive(Copy, Clone)]
            pub struct HealthResultRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> HealthResultRef<'a> {
                /// Getter for the [`ready` field](HealthResult#structfield.ready).
                #[inline]
                pub fn ready(&self) -> ::planus::Result<bool> {
                    ::core::result::Result::Ok(
                        self.0.access(0, "HealthResult", "ready")?.unwrap_or(false),
                    )
                }

                /// Getter for the [`actor_count` field](HealthResult#structfield.actor_count).
                #[inline]
                pub fn actor_count(&self) -> ::planus::Result<u32> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(1, "HealthResult", "actor_count")?
                            .unwrap_or(0),
                    )
                }

                /// Getter for the [`active_connections` field](HealthResult#structfield.active_connections).
                #[inline]
                pub fn active_connections(&self) -> ::planus::Result<u32> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(2, "HealthResult", "active_connections")?
                            .unwrap_or(0),
                    )
                }
            }

            impl<'a> ::core::fmt::Debug for HealthResultRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("HealthResultRef");
                    f.field("ready", &self.ready());
                    f.field("actor_count", &self.actor_count());
                    f.field("active_connections", &self.active_connections());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<HealthResultRef<'a>> for HealthResult {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: HealthResultRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        ready: ::core::convert::TryInto::try_into(value.ready()?)?,
                        actor_count: ::core::convert::TryInto::try_into(value.actor_count()?)?,
                        active_connections: ::core::convert::TryInto::try_into(
                            value.active_connections()?,
                        )?,
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for HealthResultRef<'a> {
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

            impl<'a> ::planus::VectorReadInner<'a> for HealthResultRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[HealthResultRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<HealthResult>> for HealthResult {
                type Value = ::planus::Offset<HealthResult>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<HealthResult>],
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

            impl<'a> ::planus::ReadAsRoot<'a> for HealthResultRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[HealthResultRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `ProjectionStat` in the namespace `hypermind.protocol`
            ///
            /// Generated from these locations:
            /// * Table `ProjectionStat` in the file `schemas/protocol.fbs:165`
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
            pub struct ProjectionStat {
                /// The field `name` in the table `ProjectionStat`
                pub name: ::planus::alloc::string::String,
                /// The field `applied_lsn` in the table `ProjectionStat`
                pub applied_lsn: u64,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for ProjectionStat {
                fn default() -> Self {
                    Self {
                        name: ::core::default::Default::default(),
                        applied_lsn: 0,
                    }
                }
            }

            impl ProjectionStat {
                /// Creates a [ProjectionStatBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> ProjectionStatBuilder<()> {
                    ProjectionStatBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_name: impl ::planus::WriteAs<::planus::Offset<str>>,
                    field_applied_lsn: impl ::planus::WriteAsDefault<u64, u64>,
                ) -> ::planus::Offset<Self> {
                    let prepared_name = field_name.prepare(builder);
                    let prepared_applied_lsn = field_applied_lsn.prepare(builder, &0);

                    let mut table_writer: ::planus::table_writer::TableWriter<8> =
                        ::core::default::Default::default();
                    if prepared_applied_lsn.is_some() {
                        table_writer.write_entry::<u64>(1);
                    }
                    table_writer.write_entry::<::planus::Offset<str>>(0);

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            if let ::core::option::Option::Some(prepared_applied_lsn) =
                                prepared_applied_lsn
                            {
                                object_writer.write::<_, _, 8>(&prepared_applied_lsn);
                            }
                            object_writer.write::<_, _, 4>(&prepared_name);
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<ProjectionStat>> for ProjectionStat {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<ProjectionStat> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<ProjectionStat>> for ProjectionStat {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<ProjectionStat>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<ProjectionStat> for ProjectionStat {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<ProjectionStat> {
                    ProjectionStat::create(builder, &self.name, self.applied_lsn)
                }
            }

            /// Builder for serializing an instance of the [ProjectionStat] type.
            ///
            /// Can be created using the [ProjectionStat::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct ProjectionStatBuilder<State>(State);

            impl ProjectionStatBuilder<()> {
                /// Setter for the [`name` field](ProjectionStat#structfield.name).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn name<T0>(self, value: T0) -> ProjectionStatBuilder<(T0,)>
                where
                    T0: ::planus::WriteAs<::planus::Offset<str>>,
                {
                    ProjectionStatBuilder((value,))
                }
            }

            impl<T0> ProjectionStatBuilder<(T0,)> {
                /// Setter for the [`applied_lsn` field](ProjectionStat#structfield.applied_lsn).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn applied_lsn<T1>(self, value: T1) -> ProjectionStatBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAsDefault<u64, u64>,
                {
                    let (v0,) = self.0;
                    ProjectionStatBuilder((v0, value))
                }

                /// Sets the [`applied_lsn` field](ProjectionStat#structfield.applied_lsn) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn applied_lsn_as_default(
                    self,
                ) -> ProjectionStatBuilder<(T0, ::planus::DefaultValue)> {
                    self.applied_lsn(::planus::DefaultValue)
                }
            }

            impl<T0, T1> ProjectionStatBuilder<(T0, T1)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [ProjectionStat].
                #[inline]
                pub fn finish(
                    self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<ProjectionStat>
                where
                    Self: ::planus::WriteAsOffset<ProjectionStat>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<str>>,
                T1: ::planus::WriteAsDefault<u64, u64>,
            > ::planus::WriteAs<::planus::Offset<ProjectionStat>>
                for ProjectionStatBuilder<(T0, T1)>
            {
                type Prepared = ::planus::Offset<ProjectionStat>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<ProjectionStat> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<str>>,
                T1: ::planus::WriteAsDefault<u64, u64>,
            > ::planus::WriteAsOptional<::planus::Offset<ProjectionStat>>
                for ProjectionStatBuilder<(T0, T1)>
            {
                type Prepared = ::planus::Offset<ProjectionStat>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<ProjectionStat>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<str>>,
                T1: ::planus::WriteAsDefault<u64, u64>,
            > ::planus::WriteAsOffset<ProjectionStat> for ProjectionStatBuilder<(T0, T1)>
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<ProjectionStat> {
                    let (v0, v1) = &self.0;
                    ProjectionStat::create(builder, v0, v1)
                }
            }

            /// Reference to a deserialized [ProjectionStat].
            #[derive(Copy, Clone)]
            pub struct ProjectionStatRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> ProjectionStatRef<'a> {
                /// Getter for the [`name` field](ProjectionStat#structfield.name).
                #[inline]
                pub fn name(&self) -> ::planus::Result<&'a ::core::primitive::str> {
                    self.0.access_required(0, "ProjectionStat", "name")
                }

                /// Getter for the [`applied_lsn` field](ProjectionStat#structfield.applied_lsn).
                #[inline]
                pub fn applied_lsn(&self) -> ::planus::Result<u64> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(1, "ProjectionStat", "applied_lsn")?
                            .unwrap_or(0),
                    )
                }
            }

            impl<'a> ::core::fmt::Debug for ProjectionStatRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("ProjectionStatRef");
                    f.field("name", &self.name());
                    f.field("applied_lsn", &self.applied_lsn());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<ProjectionStatRef<'a>> for ProjectionStat {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: ProjectionStatRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        name: ::core::convert::Into::into(value.name()?),
                        applied_lsn: ::core::convert::TryInto::try_into(value.applied_lsn()?)?,
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for ProjectionStatRef<'a> {
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

            impl<'a> ::planus::VectorReadInner<'a> for ProjectionStatRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[ProjectionStatRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<ProjectionStat>> for ProjectionStat {
                type Value = ::planus::Offset<ProjectionStat>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<ProjectionStat>],
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

            impl<'a> ::planus::ReadAsRoot<'a> for ProjectionStatRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[ProjectionStatRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `StatsResult` in the namespace `hypermind.protocol`
            ///
            /// Generated from these locations:
            /// * Table `StatsResult` in the file `schemas/protocol.fbs:170`
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
            pub struct StatsResult {
                /// The field `actor` in the table `StatsResult`
                pub actor: u16,
                /// The field `log_events` in the table `StatsResult`
                pub log_events: u64,
                /// The field `log_bytes` in the table `StatsResult`
                pub log_bytes: u64,
                /// The field `projection_stats` in the table `StatsResult`
                pub projection_stats: ::planus::alloc::vec::Vec<self::ProjectionStat>,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for StatsResult {
                fn default() -> Self {
                    Self {
                        actor: 0,
                        log_events: 0,
                        log_bytes: 0,
                        projection_stats: ::core::default::Default::default(),
                    }
                }
            }

            impl StatsResult {
                /// Creates a [StatsResultBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> StatsResultBuilder<()> {
                    StatsResultBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_actor: impl ::planus::WriteAsDefault<u16, u16>,
                    field_log_events: impl ::planus::WriteAsDefault<u64, u64>,
                    field_log_bytes: impl ::planus::WriteAsDefault<u64, u64>,
                    field_projection_stats: impl ::planus::WriteAs<
                        ::planus::Offset<[::planus::Offset<self::ProjectionStat>]>,
                    >,
                ) -> ::planus::Offset<Self> {
                    let prepared_actor = field_actor.prepare(builder, &0);
                    let prepared_log_events = field_log_events.prepare(builder, &0);
                    let prepared_log_bytes = field_log_bytes.prepare(builder, &0);
                    let prepared_projection_stats = field_projection_stats.prepare(builder);

                    let mut table_writer: ::planus::table_writer::TableWriter<12> =
                        ::core::default::Default::default();
                    if prepared_log_events.is_some() {
                        table_writer.write_entry::<u64>(1);
                    }
                    if prepared_log_bytes.is_some() {
                        table_writer.write_entry::<u64>(2);
                    }
                    table_writer
                        .write_entry::<::planus::Offset<[::planus::Offset<self::ProjectionStat>]>>(
                            3,
                        );
                    if prepared_actor.is_some() {
                        table_writer.write_entry::<u16>(0);
                    }

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            if let ::core::option::Option::Some(prepared_log_events) =
                                prepared_log_events
                            {
                                object_writer.write::<_, _, 8>(&prepared_log_events);
                            }
                            if let ::core::option::Option::Some(prepared_log_bytes) =
                                prepared_log_bytes
                            {
                                object_writer.write::<_, _, 8>(&prepared_log_bytes);
                            }
                            object_writer.write::<_, _, 4>(&prepared_projection_stats);
                            if let ::core::option::Option::Some(prepared_actor) = prepared_actor {
                                object_writer.write::<_, _, 2>(&prepared_actor);
                            }
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<StatsResult>> for StatsResult {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<StatsResult> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<StatsResult>> for StatsResult {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<StatsResult>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<StatsResult> for StatsResult {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<StatsResult> {
                    StatsResult::create(
                        builder,
                        self.actor,
                        self.log_events,
                        self.log_bytes,
                        &self.projection_stats,
                    )
                }
            }

            /// Builder for serializing an instance of the [StatsResult] type.
            ///
            /// Can be created using the [StatsResult::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct StatsResultBuilder<State>(State);

            impl StatsResultBuilder<()> {
                /// Setter for the [`actor` field](StatsResult#structfield.actor).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn actor<T0>(self, value: T0) -> StatsResultBuilder<(T0,)>
                where
                    T0: ::planus::WriteAsDefault<u16, u16>,
                {
                    StatsResultBuilder((value,))
                }

                /// Sets the [`actor` field](StatsResult#structfield.actor) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn actor_as_default(self) -> StatsResultBuilder<(::planus::DefaultValue,)> {
                    self.actor(::planus::DefaultValue)
                }
            }

            impl<T0> StatsResultBuilder<(T0,)> {
                /// Setter for the [`log_events` field](StatsResult#structfield.log_events).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn log_events<T1>(self, value: T1) -> StatsResultBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAsDefault<u64, u64>,
                {
                    let (v0,) = self.0;
                    StatsResultBuilder((v0, value))
                }

                /// Sets the [`log_events` field](StatsResult#structfield.log_events) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn log_events_as_default(
                    self,
                ) -> StatsResultBuilder<(T0, ::planus::DefaultValue)> {
                    self.log_events(::planus::DefaultValue)
                }
            }

            impl<T0, T1> StatsResultBuilder<(T0, T1)> {
                /// Setter for the [`log_bytes` field](StatsResult#structfield.log_bytes).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn log_bytes<T2>(self, value: T2) -> StatsResultBuilder<(T0, T1, T2)>
                where
                    T2: ::planus::WriteAsDefault<u64, u64>,
                {
                    let (v0, v1) = self.0;
                    StatsResultBuilder((v0, v1, value))
                }

                /// Sets the [`log_bytes` field](StatsResult#structfield.log_bytes) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn log_bytes_as_default(
                    self,
                ) -> StatsResultBuilder<(T0, T1, ::planus::DefaultValue)> {
                    self.log_bytes(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2> StatsResultBuilder<(T0, T1, T2)> {
                /// Setter for the [`projection_stats` field](StatsResult#structfield.projection_stats).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn projection_stats<T3>(self, value: T3) -> StatsResultBuilder<(T0, T1, T2, T3)>
                where
                    T3: ::planus::WriteAs<
                            ::planus::Offset<[::planus::Offset<self::ProjectionStat>]>,
                        >,
                {
                    let (v0, v1, v2) = self.0;
                    StatsResultBuilder((v0, v1, v2, value))
                }
            }

            impl<T0, T1, T2, T3> StatsResultBuilder<(T0, T1, T2, T3)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [StatsResult].
                #[inline]
                pub fn finish(
                    self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<StatsResult>
                where
                    Self: ::planus::WriteAsOffset<StatsResult>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u16, u16>,
                T1: ::planus::WriteAsDefault<u64, u64>,
                T2: ::planus::WriteAsDefault<u64, u64>,
                T3: ::planus::WriteAs<::planus::Offset<[::planus::Offset<self::ProjectionStat>]>>,
            > ::planus::WriteAs<::planus::Offset<StatsResult>>
                for StatsResultBuilder<(T0, T1, T2, T3)>
            {
                type Prepared = ::planus::Offset<StatsResult>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<StatsResult> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u16, u16>,
                T1: ::planus::WriteAsDefault<u64, u64>,
                T2: ::planus::WriteAsDefault<u64, u64>,
                T3: ::planus::WriteAs<::planus::Offset<[::planus::Offset<self::ProjectionStat>]>>,
            > ::planus::WriteAsOptional<::planus::Offset<StatsResult>>
                for StatsResultBuilder<(T0, T1, T2, T3)>
            {
                type Prepared = ::planus::Offset<StatsResult>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<StatsResult>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u16, u16>,
                T1: ::planus::WriteAsDefault<u64, u64>,
                T2: ::planus::WriteAsDefault<u64, u64>,
                T3: ::planus::WriteAs<::planus::Offset<[::planus::Offset<self::ProjectionStat>]>>,
            > ::planus::WriteAsOffset<StatsResult> for StatsResultBuilder<(T0, T1, T2, T3)>
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<StatsResult> {
                    let (v0, v1, v2, v3) = &self.0;
                    StatsResult::create(builder, v0, v1, v2, v3)
                }
            }

            /// Reference to a deserialized [StatsResult].
            #[derive(Copy, Clone)]
            pub struct StatsResultRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> StatsResultRef<'a> {
                /// Getter for the [`actor` field](StatsResult#structfield.actor).
                #[inline]
                pub fn actor(&self) -> ::planus::Result<u16> {
                    ::core::result::Result::Ok(
                        self.0.access(0, "StatsResult", "actor")?.unwrap_or(0),
                    )
                }

                /// Getter for the [`log_events` field](StatsResult#structfield.log_events).
                #[inline]
                pub fn log_events(&self) -> ::planus::Result<u64> {
                    ::core::result::Result::Ok(
                        self.0.access(1, "StatsResult", "log_events")?.unwrap_or(0),
                    )
                }

                /// Getter for the [`log_bytes` field](StatsResult#structfield.log_bytes).
                #[inline]
                pub fn log_bytes(&self) -> ::planus::Result<u64> {
                    ::core::result::Result::Ok(
                        self.0.access(2, "StatsResult", "log_bytes")?.unwrap_or(0),
                    )
                }

                /// Getter for the [`projection_stats` field](StatsResult#structfield.projection_stats).
                #[inline]
                pub fn projection_stats(
                    &self,
                ) -> ::planus::Result<
                    ::planus::Vector<'a, ::planus::Result<self::ProjectionStatRef<'a>>>,
                > {
                    self.0.access_required(3, "StatsResult", "projection_stats")
                }
            }

            impl<'a> ::core::fmt::Debug for StatsResultRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("StatsResultRef");
                    f.field("actor", &self.actor());
                    f.field("log_events", &self.log_events());
                    f.field("log_bytes", &self.log_bytes());
                    f.field("projection_stats", &self.projection_stats());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<StatsResultRef<'a>> for StatsResult {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: StatsResultRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        actor: ::core::convert::TryInto::try_into(value.actor()?)?,
                        log_events: ::core::convert::TryInto::try_into(value.log_events()?)?,
                        log_bytes: ::core::convert::TryInto::try_into(value.log_bytes()?)?,
                        projection_stats: value.projection_stats()?.to_vec_result()?,
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for StatsResultRef<'a> {
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

            impl<'a> ::planus::VectorReadInner<'a> for StatsResultRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[StatsResultRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<StatsResult>> for StatsResult {
                type Value = ::planus::Offset<StatsResult>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<StatsResult>],
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

            impl<'a> ::planus::ReadAsRoot<'a> for StatsResultRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[StatsResultRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `LatencyBucket` in the namespace `hypermind.protocol`
            ///
            /// Generated from these locations:
            /// * Table `LatencyBucket` in the file `schemas/protocol.fbs:177`
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
            pub struct LatencyBucket {
                /// The field `operation` in the table `LatencyBucket`
                pub operation: ::planus::alloc::string::String,
                /// The field `upper_bound_ns` in the table `LatencyBucket`
                pub upper_bound_ns: u64,
                /// The field `count` in the table `LatencyBucket`
                pub count: u64,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for LatencyBucket {
                fn default() -> Self {
                    Self {
                        operation: ::core::default::Default::default(),
                        upper_bound_ns: 0,
                        count: 0,
                    }
                }
            }

            impl LatencyBucket {
                /// Creates a [LatencyBucketBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> LatencyBucketBuilder<()> {
                    LatencyBucketBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_operation: impl ::planus::WriteAs<::planus::Offset<str>>,
                    field_upper_bound_ns: impl ::planus::WriteAsDefault<u64, u64>,
                    field_count: impl ::planus::WriteAsDefault<u64, u64>,
                ) -> ::planus::Offset<Self> {
                    let prepared_operation = field_operation.prepare(builder);
                    let prepared_upper_bound_ns = field_upper_bound_ns.prepare(builder, &0);
                    let prepared_count = field_count.prepare(builder, &0);

                    let mut table_writer: ::planus::table_writer::TableWriter<10> =
                        ::core::default::Default::default();
                    if prepared_upper_bound_ns.is_some() {
                        table_writer.write_entry::<u64>(1);
                    }
                    if prepared_count.is_some() {
                        table_writer.write_entry::<u64>(2);
                    }
                    table_writer.write_entry::<::planus::Offset<str>>(0);

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            if let ::core::option::Option::Some(prepared_upper_bound_ns) =
                                prepared_upper_bound_ns
                            {
                                object_writer.write::<_, _, 8>(&prepared_upper_bound_ns);
                            }
                            if let ::core::option::Option::Some(prepared_count) = prepared_count {
                                object_writer.write::<_, _, 8>(&prepared_count);
                            }
                            object_writer.write::<_, _, 4>(&prepared_operation);
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<LatencyBucket>> for LatencyBucket {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<LatencyBucket> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<LatencyBucket>> for LatencyBucket {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<LatencyBucket>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<LatencyBucket> for LatencyBucket {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<LatencyBucket> {
                    LatencyBucket::create(builder, &self.operation, self.upper_bound_ns, self.count)
                }
            }

            /// Builder for serializing an instance of the [LatencyBucket] type.
            ///
            /// Can be created using the [LatencyBucket::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct LatencyBucketBuilder<State>(State);

            impl LatencyBucketBuilder<()> {
                /// Setter for the [`operation` field](LatencyBucket#structfield.operation).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn operation<T0>(self, value: T0) -> LatencyBucketBuilder<(T0,)>
                where
                    T0: ::planus::WriteAs<::planus::Offset<str>>,
                {
                    LatencyBucketBuilder((value,))
                }
            }

            impl<T0> LatencyBucketBuilder<(T0,)> {
                /// Setter for the [`upper_bound_ns` field](LatencyBucket#structfield.upper_bound_ns).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn upper_bound_ns<T1>(self, value: T1) -> LatencyBucketBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAsDefault<u64, u64>,
                {
                    let (v0,) = self.0;
                    LatencyBucketBuilder((v0, value))
                }

                /// Sets the [`upper_bound_ns` field](LatencyBucket#structfield.upper_bound_ns) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn upper_bound_ns_as_default(
                    self,
                ) -> LatencyBucketBuilder<(T0, ::planus::DefaultValue)> {
                    self.upper_bound_ns(::planus::DefaultValue)
                }
            }

            impl<T0, T1> LatencyBucketBuilder<(T0, T1)> {
                /// Setter for the [`count` field](LatencyBucket#structfield.count).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn count<T2>(self, value: T2) -> LatencyBucketBuilder<(T0, T1, T2)>
                where
                    T2: ::planus::WriteAsDefault<u64, u64>,
                {
                    let (v0, v1) = self.0;
                    LatencyBucketBuilder((v0, v1, value))
                }

                /// Sets the [`count` field](LatencyBucket#structfield.count) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn count_as_default(
                    self,
                ) -> LatencyBucketBuilder<(T0, T1, ::planus::DefaultValue)> {
                    self.count(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2> LatencyBucketBuilder<(T0, T1, T2)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [LatencyBucket].
                #[inline]
                pub fn finish(
                    self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<LatencyBucket>
                where
                    Self: ::planus::WriteAsOffset<LatencyBucket>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<str>>,
                T1: ::planus::WriteAsDefault<u64, u64>,
                T2: ::planus::WriteAsDefault<u64, u64>,
            > ::planus::WriteAs<::planus::Offset<LatencyBucket>>
                for LatencyBucketBuilder<(T0, T1, T2)>
            {
                type Prepared = ::planus::Offset<LatencyBucket>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<LatencyBucket> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<str>>,
                T1: ::planus::WriteAsDefault<u64, u64>,
                T2: ::planus::WriteAsDefault<u64, u64>,
            > ::planus::WriteAsOptional<::planus::Offset<LatencyBucket>>
                for LatencyBucketBuilder<(T0, T1, T2)>
            {
                type Prepared = ::planus::Offset<LatencyBucket>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<LatencyBucket>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<str>>,
                T1: ::planus::WriteAsDefault<u64, u64>,
                T2: ::planus::WriteAsDefault<u64, u64>,
            > ::planus::WriteAsOffset<LatencyBucket> for LatencyBucketBuilder<(T0, T1, T2)>
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<LatencyBucket> {
                    let (v0, v1, v2) = &self.0;
                    LatencyBucket::create(builder, v0, v1, v2)
                }
            }

            /// Reference to a deserialized [LatencyBucket].
            #[derive(Copy, Clone)]
            pub struct LatencyBucketRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> LatencyBucketRef<'a> {
                /// Getter for the [`operation` field](LatencyBucket#structfield.operation).
                #[inline]
                pub fn operation(&self) -> ::planus::Result<&'a ::core::primitive::str> {
                    self.0.access_required(0, "LatencyBucket", "operation")
                }

                /// Getter for the [`upper_bound_ns` field](LatencyBucket#structfield.upper_bound_ns).
                #[inline]
                pub fn upper_bound_ns(&self) -> ::planus::Result<u64> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(1, "LatencyBucket", "upper_bound_ns")?
                            .unwrap_or(0),
                    )
                }

                /// Getter for the [`count` field](LatencyBucket#structfield.count).
                #[inline]
                pub fn count(&self) -> ::planus::Result<u64> {
                    ::core::result::Result::Ok(
                        self.0.access(2, "LatencyBucket", "count")?.unwrap_or(0),
                    )
                }
            }

            impl<'a> ::core::fmt::Debug for LatencyBucketRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("LatencyBucketRef");
                    f.field("operation", &self.operation());
                    f.field("upper_bound_ns", &self.upper_bound_ns());
                    f.field("count", &self.count());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<LatencyBucketRef<'a>> for LatencyBucket {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: LatencyBucketRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        operation: ::core::convert::Into::into(value.operation()?),
                        upper_bound_ns: ::core::convert::TryInto::try_into(
                            value.upper_bound_ns()?,
                        )?,
                        count: ::core::convert::TryInto::try_into(value.count()?)?,
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for LatencyBucketRef<'a> {
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

            impl<'a> ::planus::VectorReadInner<'a> for LatencyBucketRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[LatencyBucketRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<LatencyBucket>> for LatencyBucket {
                type Value = ::planus::Offset<LatencyBucket>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<LatencyBucket>],
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

            impl<'a> ::planus::ReadAsRoot<'a> for LatencyBucketRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[LatencyBucketRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `LatencyResult` in the namespace `hypermind.protocol`
            ///
            /// Generated from these locations:
            /// * Table `LatencyResult` in the file `schemas/protocol.fbs:183`
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
            pub struct LatencyResult {
                /// The field `buckets` in the table `LatencyResult`
                pub buckets: ::planus::alloc::vec::Vec<self::LatencyBucket>,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for LatencyResult {
                fn default() -> Self {
                    Self {
                        buckets: ::core::default::Default::default(),
                    }
                }
            }

            impl LatencyResult {
                /// Creates a [LatencyResultBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> LatencyResultBuilder<()> {
                    LatencyResultBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_buckets: impl ::planus::WriteAs<
                        ::planus::Offset<[::planus::Offset<self::LatencyBucket>]>,
                    >,
                ) -> ::planus::Offset<Self> {
                    let prepared_buckets = field_buckets.prepare(builder);

                    let mut table_writer: ::planus::table_writer::TableWriter<6> =
                        ::core::default::Default::default();
                    table_writer
                        .write_entry::<::planus::Offset<[::planus::Offset<self::LatencyBucket>]>>(
                            0,
                        );

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            object_writer.write::<_, _, 4>(&prepared_buckets);
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<LatencyResult>> for LatencyResult {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<LatencyResult> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<LatencyResult>> for LatencyResult {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<LatencyResult>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<LatencyResult> for LatencyResult {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<LatencyResult> {
                    LatencyResult::create(builder, &self.buckets)
                }
            }

            /// Builder for serializing an instance of the [LatencyResult] type.
            ///
            /// Can be created using the [LatencyResult::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct LatencyResultBuilder<State>(State);

            impl LatencyResultBuilder<()> {
                /// Setter for the [`buckets` field](LatencyResult#structfield.buckets).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn buckets<T0>(self, value: T0) -> LatencyResultBuilder<(T0,)>
                where
                    T0: ::planus::WriteAs<::planus::Offset<[::planus::Offset<self::LatencyBucket>]>>,
                {
                    LatencyResultBuilder((value,))
                }
            }

            impl<T0> LatencyResultBuilder<(T0,)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [LatencyResult].
                #[inline]
                pub fn finish(
                    self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<LatencyResult>
                where
                    Self: ::planus::WriteAsOffset<LatencyResult>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<T0: ::planus::WriteAs<::planus::Offset<[::planus::Offset<self::LatencyBucket>]>>>
                ::planus::WriteAs<::planus::Offset<LatencyResult>> for LatencyResultBuilder<(T0,)>
            {
                type Prepared = ::planus::Offset<LatencyResult>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<LatencyResult> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<T0: ::planus::WriteAs<::planus::Offset<[::planus::Offset<self::LatencyBucket>]>>>
                ::planus::WriteAsOptional<::planus::Offset<LatencyResult>>
                for LatencyResultBuilder<(T0,)>
            {
                type Prepared = ::planus::Offset<LatencyResult>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<LatencyResult>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<T0: ::planus::WriteAs<::planus::Offset<[::planus::Offset<self::LatencyBucket>]>>>
                ::planus::WriteAsOffset<LatencyResult> for LatencyResultBuilder<(T0,)>
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<LatencyResult> {
                    let (v0,) = &self.0;
                    LatencyResult::create(builder, v0)
                }
            }

            /// Reference to a deserialized [LatencyResult].
            #[derive(Copy, Clone)]
            pub struct LatencyResultRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> LatencyResultRef<'a> {
                /// Getter for the [`buckets` field](LatencyResult#structfield.buckets).
                #[inline]
                pub fn buckets(
                    &self,
                ) -> ::planus::Result<
                    ::planus::Vector<'a, ::planus::Result<self::LatencyBucketRef<'a>>>,
                > {
                    self.0.access_required(0, "LatencyResult", "buckets")
                }
            }

            impl<'a> ::core::fmt::Debug for LatencyResultRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("LatencyResultRef");
                    f.field("buckets", &self.buckets());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<LatencyResultRef<'a>> for LatencyResult {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: LatencyResultRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        buckets: value.buckets()?.to_vec_result()?,
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for LatencyResultRef<'a> {
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

            impl<'a> ::planus::VectorReadInner<'a> for LatencyResultRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[LatencyResultRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<LatencyResult>> for LatencyResult {
                type Value = ::planus::Offset<LatencyResult>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<LatencyResult>],
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

            impl<'a> ::planus::ReadAsRoot<'a> for LatencyResultRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[LatencyResultRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `VerifyResult` in the namespace `hypermind.protocol`
            ///
            /// Generated from these locations:
            /// * Table `VerifyResult` in the file `schemas/protocol.fbs:187`
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
            pub struct VerifyResult {
                /// The field `actor` in the table `VerifyResult`
                pub actor: u16,
                /// The field `root` in the table `VerifyResult`
                pub root: ::planus::alloc::vec::Vec<u8>,
                /// The field `leaf_count` in the table `VerifyResult`
                pub leaf_count: u64,
                /// The field `last_checkpoint_lsn` in the table `VerifyResult`
                pub last_checkpoint_lsn: u64,
                /// The field `verified` in the table `VerifyResult`
                pub verified: bool,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for VerifyResult {
                fn default() -> Self {
                    Self {
                        actor: 0,
                        root: ::core::default::Default::default(),
                        leaf_count: 0,
                        last_checkpoint_lsn: 0,
                        verified: false,
                    }
                }
            }

            impl VerifyResult {
                /// Creates a [VerifyResultBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> VerifyResultBuilder<()> {
                    VerifyResultBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_actor: impl ::planus::WriteAsDefault<u16, u16>,
                    field_root: impl ::planus::WriteAs<::planus::Offset<[u8]>>,
                    field_leaf_count: impl ::planus::WriteAsDefault<u64, u64>,
                    field_last_checkpoint_lsn: impl ::planus::WriteAsDefault<u64, u64>,
                    field_verified: impl ::planus::WriteAsDefault<bool, bool>,
                ) -> ::planus::Offset<Self> {
                    let prepared_actor = field_actor.prepare(builder, &0);
                    let prepared_root = field_root.prepare(builder);
                    let prepared_leaf_count = field_leaf_count.prepare(builder, &0);
                    let prepared_last_checkpoint_lsn =
                        field_last_checkpoint_lsn.prepare(builder, &0);
                    let prepared_verified = field_verified.prepare(builder, &false);

                    let mut table_writer: ::planus::table_writer::TableWriter<14> =
                        ::core::default::Default::default();
                    if prepared_leaf_count.is_some() {
                        table_writer.write_entry::<u64>(2);
                    }
                    if prepared_last_checkpoint_lsn.is_some() {
                        table_writer.write_entry::<u64>(3);
                    }
                    table_writer.write_entry::<::planus::Offset<[u8]>>(1);
                    if prepared_actor.is_some() {
                        table_writer.write_entry::<u16>(0);
                    }
                    if prepared_verified.is_some() {
                        table_writer.write_entry::<bool>(4);
                    }

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            if let ::core::option::Option::Some(prepared_leaf_count) =
                                prepared_leaf_count
                            {
                                object_writer.write::<_, _, 8>(&prepared_leaf_count);
                            }
                            if let ::core::option::Option::Some(prepared_last_checkpoint_lsn) =
                                prepared_last_checkpoint_lsn
                            {
                                object_writer.write::<_, _, 8>(&prepared_last_checkpoint_lsn);
                            }
                            object_writer.write::<_, _, 4>(&prepared_root);
                            if let ::core::option::Option::Some(prepared_actor) = prepared_actor {
                                object_writer.write::<_, _, 2>(&prepared_actor);
                            }
                            if let ::core::option::Option::Some(prepared_verified) =
                                prepared_verified
                            {
                                object_writer.write::<_, _, 1>(&prepared_verified);
                            }
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<VerifyResult>> for VerifyResult {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<VerifyResult> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<VerifyResult>> for VerifyResult {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<VerifyResult>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<VerifyResult> for VerifyResult {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<VerifyResult> {
                    VerifyResult::create(
                        builder,
                        self.actor,
                        &self.root,
                        self.leaf_count,
                        self.last_checkpoint_lsn,
                        self.verified,
                    )
                }
            }

            /// Builder for serializing an instance of the [VerifyResult] type.
            ///
            /// Can be created using the [VerifyResult::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct VerifyResultBuilder<State>(State);

            impl VerifyResultBuilder<()> {
                /// Setter for the [`actor` field](VerifyResult#structfield.actor).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn actor<T0>(self, value: T0) -> VerifyResultBuilder<(T0,)>
                where
                    T0: ::planus::WriteAsDefault<u16, u16>,
                {
                    VerifyResultBuilder((value,))
                }

                /// Sets the [`actor` field](VerifyResult#structfield.actor) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn actor_as_default(self) -> VerifyResultBuilder<(::planus::DefaultValue,)> {
                    self.actor(::planus::DefaultValue)
                }
            }

            impl<T0> VerifyResultBuilder<(T0,)> {
                /// Setter for the [`root` field](VerifyResult#structfield.root).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn root<T1>(self, value: T1) -> VerifyResultBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAs<::planus::Offset<[u8]>>,
                {
                    let (v0,) = self.0;
                    VerifyResultBuilder((v0, value))
                }
            }

            impl<T0, T1> VerifyResultBuilder<(T0, T1)> {
                /// Setter for the [`leaf_count` field](VerifyResult#structfield.leaf_count).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn leaf_count<T2>(self, value: T2) -> VerifyResultBuilder<(T0, T1, T2)>
                where
                    T2: ::planus::WriteAsDefault<u64, u64>,
                {
                    let (v0, v1) = self.0;
                    VerifyResultBuilder((v0, v1, value))
                }

                /// Sets the [`leaf_count` field](VerifyResult#structfield.leaf_count) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn leaf_count_as_default(
                    self,
                ) -> VerifyResultBuilder<(T0, T1, ::planus::DefaultValue)> {
                    self.leaf_count(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2> VerifyResultBuilder<(T0, T1, T2)> {
                /// Setter for the [`last_checkpoint_lsn` field](VerifyResult#structfield.last_checkpoint_lsn).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn last_checkpoint_lsn<T3>(
                    self,
                    value: T3,
                ) -> VerifyResultBuilder<(T0, T1, T2, T3)>
                where
                    T3: ::planus::WriteAsDefault<u64, u64>,
                {
                    let (v0, v1, v2) = self.0;
                    VerifyResultBuilder((v0, v1, v2, value))
                }

                /// Sets the [`last_checkpoint_lsn` field](VerifyResult#structfield.last_checkpoint_lsn) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn last_checkpoint_lsn_as_default(
                    self,
                ) -> VerifyResultBuilder<(T0, T1, T2, ::planus::DefaultValue)> {
                    self.last_checkpoint_lsn(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2, T3> VerifyResultBuilder<(T0, T1, T2, T3)> {
                /// Setter for the [`verified` field](VerifyResult#structfield.verified).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn verified<T4>(self, value: T4) -> VerifyResultBuilder<(T0, T1, T2, T3, T4)>
                where
                    T4: ::planus::WriteAsDefault<bool, bool>,
                {
                    let (v0, v1, v2, v3) = self.0;
                    VerifyResultBuilder((v0, v1, v2, v3, value))
                }

                /// Sets the [`verified` field](VerifyResult#structfield.verified) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn verified_as_default(
                    self,
                ) -> VerifyResultBuilder<(T0, T1, T2, T3, ::planus::DefaultValue)> {
                    self.verified(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2, T3, T4> VerifyResultBuilder<(T0, T1, T2, T3, T4)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [VerifyResult].
                #[inline]
                pub fn finish(
                    self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<VerifyResult>
                where
                    Self: ::planus::WriteAsOffset<VerifyResult>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u16, u16>,
                T1: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T2: ::planus::WriteAsDefault<u64, u64>,
                T3: ::planus::WriteAsDefault<u64, u64>,
                T4: ::planus::WriteAsDefault<bool, bool>,
            > ::planus::WriteAs<::planus::Offset<VerifyResult>>
                for VerifyResultBuilder<(T0, T1, T2, T3, T4)>
            {
                type Prepared = ::planus::Offset<VerifyResult>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<VerifyResult> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u16, u16>,
                T1: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T2: ::planus::WriteAsDefault<u64, u64>,
                T3: ::planus::WriteAsDefault<u64, u64>,
                T4: ::planus::WriteAsDefault<bool, bool>,
            > ::planus::WriteAsOptional<::planus::Offset<VerifyResult>>
                for VerifyResultBuilder<(T0, T1, T2, T3, T4)>
            {
                type Prepared = ::planus::Offset<VerifyResult>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<VerifyResult>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u16, u16>,
                T1: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T2: ::planus::WriteAsDefault<u64, u64>,
                T3: ::planus::WriteAsDefault<u64, u64>,
                T4: ::planus::WriteAsDefault<bool, bool>,
            > ::planus::WriteAsOffset<VerifyResult> for VerifyResultBuilder<(T0, T1, T2, T3, T4)>
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<VerifyResult> {
                    let (v0, v1, v2, v3, v4) = &self.0;
                    VerifyResult::create(builder, v0, v1, v2, v3, v4)
                }
            }

            /// Reference to a deserialized [VerifyResult].
            #[derive(Copy, Clone)]
            pub struct VerifyResultRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> VerifyResultRef<'a> {
                /// Getter for the [`actor` field](VerifyResult#structfield.actor).
                #[inline]
                pub fn actor(&self) -> ::planus::Result<u16> {
                    ::core::result::Result::Ok(
                        self.0.access(0, "VerifyResult", "actor")?.unwrap_or(0),
                    )
                }

                /// Getter for the [`root` field](VerifyResult#structfield.root).
                #[inline]
                pub fn root(&self) -> ::planus::Result<&'a [u8]> {
                    self.0.access_required(1, "VerifyResult", "root")
                }

                /// Getter for the [`leaf_count` field](VerifyResult#structfield.leaf_count).
                #[inline]
                pub fn leaf_count(&self) -> ::planus::Result<u64> {
                    ::core::result::Result::Ok(
                        self.0.access(2, "VerifyResult", "leaf_count")?.unwrap_or(0),
                    )
                }

                /// Getter for the [`last_checkpoint_lsn` field](VerifyResult#structfield.last_checkpoint_lsn).
                #[inline]
                pub fn last_checkpoint_lsn(&self) -> ::planus::Result<u64> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(3, "VerifyResult", "last_checkpoint_lsn")?
                            .unwrap_or(0),
                    )
                }

                /// Getter for the [`verified` field](VerifyResult#structfield.verified).
                #[inline]
                pub fn verified(&self) -> ::planus::Result<bool> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(4, "VerifyResult", "verified")?
                            .unwrap_or(false),
                    )
                }
            }

            impl<'a> ::core::fmt::Debug for VerifyResultRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("VerifyResultRef");
                    f.field("actor", &self.actor());
                    f.field("root", &self.root());
                    f.field("leaf_count", &self.leaf_count());
                    f.field("last_checkpoint_lsn", &self.last_checkpoint_lsn());
                    f.field("verified", &self.verified());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<VerifyResultRef<'a>> for VerifyResult {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: VerifyResultRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        actor: ::core::convert::TryInto::try_into(value.actor()?)?,
                        root: value.root()?.to_vec(),
                        leaf_count: ::core::convert::TryInto::try_into(value.leaf_count()?)?,
                        last_checkpoint_lsn: ::core::convert::TryInto::try_into(
                            value.last_checkpoint_lsn()?,
                        )?,
                        verified: ::core::convert::TryInto::try_into(value.verified()?)?,
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for VerifyResultRef<'a> {
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

            impl<'a> ::planus::VectorReadInner<'a> for VerifyResultRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[VerifyResultRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<VerifyResult>> for VerifyResult {
                type Value = ::planus::Offset<VerifyResult>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<VerifyResult>],
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

            impl<'a> ::planus::ReadAsRoot<'a> for VerifyResultRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[VerifyResultRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `RebuildResult` in the namespace `hypermind.protocol`
            ///
            /// Generated from these locations:
            /// * Table `RebuildResult` in the file `schemas/protocol.fbs:195`
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
            pub struct RebuildResult {
                /// The field `actor` in the table `RebuildResult`
                pub actor: u16,
                /// The field `name` in the table `RebuildResult`
                pub name: ::planus::alloc::string::String,
                /// The field `applied_lsn` in the table `RebuildResult`
                pub applied_lsn: u64,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for RebuildResult {
                fn default() -> Self {
                    Self {
                        actor: 0,
                        name: ::core::default::Default::default(),
                        applied_lsn: 0,
                    }
                }
            }

            impl RebuildResult {
                /// Creates a [RebuildResultBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> RebuildResultBuilder<()> {
                    RebuildResultBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_actor: impl ::planus::WriteAsDefault<u16, u16>,
                    field_name: impl ::planus::WriteAs<::planus::Offset<str>>,
                    field_applied_lsn: impl ::planus::WriteAsDefault<u64, u64>,
                ) -> ::planus::Offset<Self> {
                    let prepared_actor = field_actor.prepare(builder, &0);
                    let prepared_name = field_name.prepare(builder);
                    let prepared_applied_lsn = field_applied_lsn.prepare(builder, &0);

                    let mut table_writer: ::planus::table_writer::TableWriter<10> =
                        ::core::default::Default::default();
                    if prepared_applied_lsn.is_some() {
                        table_writer.write_entry::<u64>(2);
                    }
                    table_writer.write_entry::<::planus::Offset<str>>(1);
                    if prepared_actor.is_some() {
                        table_writer.write_entry::<u16>(0);
                    }

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            if let ::core::option::Option::Some(prepared_applied_lsn) =
                                prepared_applied_lsn
                            {
                                object_writer.write::<_, _, 8>(&prepared_applied_lsn);
                            }
                            object_writer.write::<_, _, 4>(&prepared_name);
                            if let ::core::option::Option::Some(prepared_actor) = prepared_actor {
                                object_writer.write::<_, _, 2>(&prepared_actor);
                            }
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<RebuildResult>> for RebuildResult {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<RebuildResult> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<RebuildResult>> for RebuildResult {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<RebuildResult>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<RebuildResult> for RebuildResult {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<RebuildResult> {
                    RebuildResult::create(builder, self.actor, &self.name, self.applied_lsn)
                }
            }

            /// Builder for serializing an instance of the [RebuildResult] type.
            ///
            /// Can be created using the [RebuildResult::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct RebuildResultBuilder<State>(State);

            impl RebuildResultBuilder<()> {
                /// Setter for the [`actor` field](RebuildResult#structfield.actor).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn actor<T0>(self, value: T0) -> RebuildResultBuilder<(T0,)>
                where
                    T0: ::planus::WriteAsDefault<u16, u16>,
                {
                    RebuildResultBuilder((value,))
                }

                /// Sets the [`actor` field](RebuildResult#structfield.actor) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn actor_as_default(self) -> RebuildResultBuilder<(::planus::DefaultValue,)> {
                    self.actor(::planus::DefaultValue)
                }
            }

            impl<T0> RebuildResultBuilder<(T0,)> {
                /// Setter for the [`name` field](RebuildResult#structfield.name).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn name<T1>(self, value: T1) -> RebuildResultBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAs<::planus::Offset<str>>,
                {
                    let (v0,) = self.0;
                    RebuildResultBuilder((v0, value))
                }
            }

            impl<T0, T1> RebuildResultBuilder<(T0, T1)> {
                /// Setter for the [`applied_lsn` field](RebuildResult#structfield.applied_lsn).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn applied_lsn<T2>(self, value: T2) -> RebuildResultBuilder<(T0, T1, T2)>
                where
                    T2: ::planus::WriteAsDefault<u64, u64>,
                {
                    let (v0, v1) = self.0;
                    RebuildResultBuilder((v0, v1, value))
                }

                /// Sets the [`applied_lsn` field](RebuildResult#structfield.applied_lsn) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn applied_lsn_as_default(
                    self,
                ) -> RebuildResultBuilder<(T0, T1, ::planus::DefaultValue)> {
                    self.applied_lsn(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2> RebuildResultBuilder<(T0, T1, T2)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [RebuildResult].
                #[inline]
                pub fn finish(
                    self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<RebuildResult>
                where
                    Self: ::planus::WriteAsOffset<RebuildResult>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u16, u16>,
                T1: ::planus::WriteAs<::planus::Offset<str>>,
                T2: ::planus::WriteAsDefault<u64, u64>,
            > ::planus::WriteAs<::planus::Offset<RebuildResult>>
                for RebuildResultBuilder<(T0, T1, T2)>
            {
                type Prepared = ::planus::Offset<RebuildResult>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<RebuildResult> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u16, u16>,
                T1: ::planus::WriteAs<::planus::Offset<str>>,
                T2: ::planus::WriteAsDefault<u64, u64>,
            > ::planus::WriteAsOptional<::planus::Offset<RebuildResult>>
                for RebuildResultBuilder<(T0, T1, T2)>
            {
                type Prepared = ::planus::Offset<RebuildResult>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<RebuildResult>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u16, u16>,
                T1: ::planus::WriteAs<::planus::Offset<str>>,
                T2: ::planus::WriteAsDefault<u64, u64>,
            > ::planus::WriteAsOffset<RebuildResult> for RebuildResultBuilder<(T0, T1, T2)>
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<RebuildResult> {
                    let (v0, v1, v2) = &self.0;
                    RebuildResult::create(builder, v0, v1, v2)
                }
            }

            /// Reference to a deserialized [RebuildResult].
            #[derive(Copy, Clone)]
            pub struct RebuildResultRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> RebuildResultRef<'a> {
                /// Getter for the [`actor` field](RebuildResult#structfield.actor).
                #[inline]
                pub fn actor(&self) -> ::planus::Result<u16> {
                    ::core::result::Result::Ok(
                        self.0.access(0, "RebuildResult", "actor")?.unwrap_or(0),
                    )
                }

                /// Getter for the [`name` field](RebuildResult#structfield.name).
                #[inline]
                pub fn name(&self) -> ::planus::Result<&'a ::core::primitive::str> {
                    self.0.access_required(1, "RebuildResult", "name")
                }

                /// Getter for the [`applied_lsn` field](RebuildResult#structfield.applied_lsn).
                #[inline]
                pub fn applied_lsn(&self) -> ::planus::Result<u64> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(2, "RebuildResult", "applied_lsn")?
                            .unwrap_or(0),
                    )
                }
            }

            impl<'a> ::core::fmt::Debug for RebuildResultRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("RebuildResultRef");
                    f.field("actor", &self.actor());
                    f.field("name", &self.name());
                    f.field("applied_lsn", &self.applied_lsn());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<RebuildResultRef<'a>> for RebuildResult {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: RebuildResultRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        actor: ::core::convert::TryInto::try_into(value.actor()?)?,
                        name: ::core::convert::Into::into(value.name()?),
                        applied_lsn: ::core::convert::TryInto::try_into(value.applied_lsn()?)?,
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for RebuildResultRef<'a> {
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

            impl<'a> ::planus::VectorReadInner<'a> for RebuildResultRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[RebuildResultRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<RebuildResult>> for RebuildResult {
                type Value = ::planus::Offset<RebuildResult>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<RebuildResult>],
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

            impl<'a> ::planus::ReadAsRoot<'a> for RebuildResultRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[RebuildResultRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `DeleteResult` in the namespace `hypermind.protocol`
            ///
            /// Generated from these locations:
            /// * Table `DeleteResult` in the file `schemas/protocol.fbs:201`
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
            pub struct DeleteResult {
                /// The field `actor` in the table `DeleteResult`
                pub actor: u16,
                /// The field `receipt` in the table `DeleteResult`
                pub receipt: ::planus::alloc::vec::Vec<u8>,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for DeleteResult {
                fn default() -> Self {
                    Self {
                        actor: 0,
                        receipt: ::core::default::Default::default(),
                    }
                }
            }

            impl DeleteResult {
                /// Creates a [DeleteResultBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> DeleteResultBuilder<()> {
                    DeleteResultBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_actor: impl ::planus::WriteAsDefault<u16, u16>,
                    field_receipt: impl ::planus::WriteAs<::planus::Offset<[u8]>>,
                ) -> ::planus::Offset<Self> {
                    let prepared_actor = field_actor.prepare(builder, &0);
                    let prepared_receipt = field_receipt.prepare(builder);

                    let mut table_writer: ::planus::table_writer::TableWriter<8> =
                        ::core::default::Default::default();
                    table_writer.write_entry::<::planus::Offset<[u8]>>(1);
                    if prepared_actor.is_some() {
                        table_writer.write_entry::<u16>(0);
                    }

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            object_writer.write::<_, _, 4>(&prepared_receipt);
                            if let ::core::option::Option::Some(prepared_actor) = prepared_actor {
                                object_writer.write::<_, _, 2>(&prepared_actor);
                            }
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<DeleteResult>> for DeleteResult {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<DeleteResult> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<DeleteResult>> for DeleteResult {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<DeleteResult>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<DeleteResult> for DeleteResult {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<DeleteResult> {
                    DeleteResult::create(builder, self.actor, &self.receipt)
                }
            }

            /// Builder for serializing an instance of the [DeleteResult] type.
            ///
            /// Can be created using the [DeleteResult::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct DeleteResultBuilder<State>(State);

            impl DeleteResultBuilder<()> {
                /// Setter for the [`actor` field](DeleteResult#structfield.actor).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn actor<T0>(self, value: T0) -> DeleteResultBuilder<(T0,)>
                where
                    T0: ::planus::WriteAsDefault<u16, u16>,
                {
                    DeleteResultBuilder((value,))
                }

                /// Sets the [`actor` field](DeleteResult#structfield.actor) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn actor_as_default(self) -> DeleteResultBuilder<(::planus::DefaultValue,)> {
                    self.actor(::planus::DefaultValue)
                }
            }

            impl<T0> DeleteResultBuilder<(T0,)> {
                /// Setter for the [`receipt` field](DeleteResult#structfield.receipt).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn receipt<T1>(self, value: T1) -> DeleteResultBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAs<::planus::Offset<[u8]>>,
                {
                    let (v0,) = self.0;
                    DeleteResultBuilder((v0, value))
                }
            }

            impl<T0, T1> DeleteResultBuilder<(T0, T1)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [DeleteResult].
                #[inline]
                pub fn finish(
                    self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<DeleteResult>
                where
                    Self: ::planus::WriteAsOffset<DeleteResult>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u16, u16>,
                T1: ::planus::WriteAs<::planus::Offset<[u8]>>,
            > ::planus::WriteAs<::planus::Offset<DeleteResult>> for DeleteResultBuilder<(T0, T1)>
            {
                type Prepared = ::planus::Offset<DeleteResult>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<DeleteResult> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u16, u16>,
                T1: ::planus::WriteAs<::planus::Offset<[u8]>>,
            > ::planus::WriteAsOptional<::planus::Offset<DeleteResult>>
                for DeleteResultBuilder<(T0, T1)>
            {
                type Prepared = ::planus::Offset<DeleteResult>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<DeleteResult>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u16, u16>,
                T1: ::planus::WriteAs<::planus::Offset<[u8]>>,
            > ::planus::WriteAsOffset<DeleteResult> for DeleteResultBuilder<(T0, T1)>
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<DeleteResult> {
                    let (v0, v1) = &self.0;
                    DeleteResult::create(builder, v0, v1)
                }
            }

            /// Reference to a deserialized [DeleteResult].
            #[derive(Copy, Clone)]
            pub struct DeleteResultRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> DeleteResultRef<'a> {
                /// Getter for the [`actor` field](DeleteResult#structfield.actor).
                #[inline]
                pub fn actor(&self) -> ::planus::Result<u16> {
                    ::core::result::Result::Ok(
                        self.0.access(0, "DeleteResult", "actor")?.unwrap_or(0),
                    )
                }

                /// Getter for the [`receipt` field](DeleteResult#structfield.receipt).
                #[inline]
                pub fn receipt(&self) -> ::planus::Result<&'a [u8]> {
                    self.0.access_required(1, "DeleteResult", "receipt")
                }
            }

            impl<'a> ::core::fmt::Debug for DeleteResultRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("DeleteResultRef");
                    f.field("actor", &self.actor());
                    f.field("receipt", &self.receipt());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<DeleteResultRef<'a>> for DeleteResult {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: DeleteResultRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        actor: ::core::convert::TryInto::try_into(value.actor()?)?,
                        receipt: value.receipt()?.to_vec(),
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for DeleteResultRef<'a> {
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

            impl<'a> ::planus::VectorReadInner<'a> for DeleteResultRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[DeleteResultRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<DeleteResult>> for DeleteResult {
                type Value = ::planus::Offset<DeleteResult>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<DeleteResult>],
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

            impl<'a> ::planus::ReadAsRoot<'a> for DeleteResultRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[DeleteResultRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `FrameRecord` in the namespace `hypermind.protocol`
            ///
            /// Generated from these locations:
            /// * Table `FrameRecord` in the file `schemas/protocol.fbs:206`
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
            pub struct FrameRecord {
                /// The field `lsn` in the table `FrameRecord`
                pub lsn: u64,
                /// The field `kind` in the table `FrameRecord`
                pub kind: u8,
                /// The field `wall_timestamp_ns` in the table `FrameRecord`
                pub wall_timestamp_ns: i64,
                /// The field `actor` in the table `FrameRecord`
                pub actor: u16,
                /// The field `conversation` in the table `FrameRecord`
                pub conversation: ::planus::alloc::vec::Vec<u8>,
                /// The field `payload` in the table `FrameRecord`
                pub payload: ::planus::alloc::vec::Vec<u8>,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for FrameRecord {
                fn default() -> Self {
                    Self {
                        lsn: 0,
                        kind: 0,
                        wall_timestamp_ns: 0,
                        actor: 0,
                        conversation: ::core::default::Default::default(),
                        payload: ::core::default::Default::default(),
                    }
                }
            }

            impl FrameRecord {
                /// Creates a [FrameRecordBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> FrameRecordBuilder<()> {
                    FrameRecordBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_lsn: impl ::planus::WriteAsDefault<u64, u64>,
                    field_kind: impl ::planus::WriteAsDefault<u8, u8>,
                    field_wall_timestamp_ns: impl ::planus::WriteAsDefault<i64, i64>,
                    field_actor: impl ::planus::WriteAsDefault<u16, u16>,
                    field_conversation: impl ::planus::WriteAs<::planus::Offset<[u8]>>,
                    field_payload: impl ::planus::WriteAs<::planus::Offset<[u8]>>,
                ) -> ::planus::Offset<Self> {
                    let prepared_lsn = field_lsn.prepare(builder, &0);
                    let prepared_kind = field_kind.prepare(builder, &0);
                    let prepared_wall_timestamp_ns = field_wall_timestamp_ns.prepare(builder, &0);
                    let prepared_actor = field_actor.prepare(builder, &0);
                    let prepared_conversation = field_conversation.prepare(builder);
                    let prepared_payload = field_payload.prepare(builder);

                    let mut table_writer: ::planus::table_writer::TableWriter<16> =
                        ::core::default::Default::default();
                    if prepared_lsn.is_some() {
                        table_writer.write_entry::<u64>(0);
                    }
                    if prepared_wall_timestamp_ns.is_some() {
                        table_writer.write_entry::<i64>(2);
                    }
                    table_writer.write_entry::<::planus::Offset<[u8]>>(4);
                    table_writer.write_entry::<::planus::Offset<[u8]>>(5);
                    if prepared_actor.is_some() {
                        table_writer.write_entry::<u16>(3);
                    }
                    if prepared_kind.is_some() {
                        table_writer.write_entry::<u8>(1);
                    }

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            if let ::core::option::Option::Some(prepared_lsn) = prepared_lsn {
                                object_writer.write::<_, _, 8>(&prepared_lsn);
                            }
                            if let ::core::option::Option::Some(prepared_wall_timestamp_ns) =
                                prepared_wall_timestamp_ns
                            {
                                object_writer.write::<_, _, 8>(&prepared_wall_timestamp_ns);
                            }
                            object_writer.write::<_, _, 4>(&prepared_conversation);
                            object_writer.write::<_, _, 4>(&prepared_payload);
                            if let ::core::option::Option::Some(prepared_actor) = prepared_actor {
                                object_writer.write::<_, _, 2>(&prepared_actor);
                            }
                            if let ::core::option::Option::Some(prepared_kind) = prepared_kind {
                                object_writer.write::<_, _, 1>(&prepared_kind);
                            }
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<FrameRecord>> for FrameRecord {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<FrameRecord> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<FrameRecord>> for FrameRecord {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<FrameRecord>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<FrameRecord> for FrameRecord {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<FrameRecord> {
                    FrameRecord::create(
                        builder,
                        self.lsn,
                        self.kind,
                        self.wall_timestamp_ns,
                        self.actor,
                        &self.conversation,
                        &self.payload,
                    )
                }
            }

            /// Builder for serializing an instance of the [FrameRecord] type.
            ///
            /// Can be created using the [FrameRecord::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct FrameRecordBuilder<State>(State);

            impl FrameRecordBuilder<()> {
                /// Setter for the [`lsn` field](FrameRecord#structfield.lsn).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn lsn<T0>(self, value: T0) -> FrameRecordBuilder<(T0,)>
                where
                    T0: ::planus::WriteAsDefault<u64, u64>,
                {
                    FrameRecordBuilder((value,))
                }

                /// Sets the [`lsn` field](FrameRecord#structfield.lsn) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn lsn_as_default(self) -> FrameRecordBuilder<(::planus::DefaultValue,)> {
                    self.lsn(::planus::DefaultValue)
                }
            }

            impl<T0> FrameRecordBuilder<(T0,)> {
                /// Setter for the [`kind` field](FrameRecord#structfield.kind).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn kind<T1>(self, value: T1) -> FrameRecordBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAsDefault<u8, u8>,
                {
                    let (v0,) = self.0;
                    FrameRecordBuilder((v0, value))
                }

                /// Sets the [`kind` field](FrameRecord#structfield.kind) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn kind_as_default(self) -> FrameRecordBuilder<(T0, ::planus::DefaultValue)> {
                    self.kind(::planus::DefaultValue)
                }
            }

            impl<T0, T1> FrameRecordBuilder<(T0, T1)> {
                /// Setter for the [`wall_timestamp_ns` field](FrameRecord#structfield.wall_timestamp_ns).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn wall_timestamp_ns<T2>(self, value: T2) -> FrameRecordBuilder<(T0, T1, T2)>
                where
                    T2: ::planus::WriteAsDefault<i64, i64>,
                {
                    let (v0, v1) = self.0;
                    FrameRecordBuilder((v0, v1, value))
                }

                /// Sets the [`wall_timestamp_ns` field](FrameRecord#structfield.wall_timestamp_ns) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn wall_timestamp_ns_as_default(
                    self,
                ) -> FrameRecordBuilder<(T0, T1, ::planus::DefaultValue)> {
                    self.wall_timestamp_ns(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2> FrameRecordBuilder<(T0, T1, T2)> {
                /// Setter for the [`actor` field](FrameRecord#structfield.actor).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn actor<T3>(self, value: T3) -> FrameRecordBuilder<(T0, T1, T2, T3)>
                where
                    T3: ::planus::WriteAsDefault<u16, u16>,
                {
                    let (v0, v1, v2) = self.0;
                    FrameRecordBuilder((v0, v1, v2, value))
                }

                /// Sets the [`actor` field](FrameRecord#structfield.actor) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn actor_as_default(
                    self,
                ) -> FrameRecordBuilder<(T0, T1, T2, ::planus::DefaultValue)> {
                    self.actor(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2, T3> FrameRecordBuilder<(T0, T1, T2, T3)> {
                /// Setter for the [`conversation` field](FrameRecord#structfield.conversation).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn conversation<T4>(self, value: T4) -> FrameRecordBuilder<(T0, T1, T2, T3, T4)>
                where
                    T4: ::planus::WriteAs<::planus::Offset<[u8]>>,
                {
                    let (v0, v1, v2, v3) = self.0;
                    FrameRecordBuilder((v0, v1, v2, v3, value))
                }
            }

            impl<T0, T1, T2, T3, T4> FrameRecordBuilder<(T0, T1, T2, T3, T4)> {
                /// Setter for the [`payload` field](FrameRecord#structfield.payload).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn payload<T5>(self, value: T5) -> FrameRecordBuilder<(T0, T1, T2, T3, T4, T5)>
                where
                    T5: ::planus::WriteAs<::planus::Offset<[u8]>>,
                {
                    let (v0, v1, v2, v3, v4) = self.0;
                    FrameRecordBuilder((v0, v1, v2, v3, v4, value))
                }
            }

            impl<T0, T1, T2, T3, T4, T5> FrameRecordBuilder<(T0, T1, T2, T3, T4, T5)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [FrameRecord].
                #[inline]
                pub fn finish(
                    self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<FrameRecord>
                where
                    Self: ::planus::WriteAsOffset<FrameRecord>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u64, u64>,
                T1: ::planus::WriteAsDefault<u8, u8>,
                T2: ::planus::WriteAsDefault<i64, i64>,
                T3: ::planus::WriteAsDefault<u16, u16>,
                T4: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T5: ::planus::WriteAs<::planus::Offset<[u8]>>,
            > ::planus::WriteAs<::planus::Offset<FrameRecord>>
                for FrameRecordBuilder<(T0, T1, T2, T3, T4, T5)>
            {
                type Prepared = ::planus::Offset<FrameRecord>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<FrameRecord> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u64, u64>,
                T1: ::planus::WriteAsDefault<u8, u8>,
                T2: ::planus::WriteAsDefault<i64, i64>,
                T3: ::planus::WriteAsDefault<u16, u16>,
                T4: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T5: ::planus::WriteAs<::planus::Offset<[u8]>>,
            > ::planus::WriteAsOptional<::planus::Offset<FrameRecord>>
                for FrameRecordBuilder<(T0, T1, T2, T3, T4, T5)>
            {
                type Prepared = ::planus::Offset<FrameRecord>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<FrameRecord>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u64, u64>,
                T1: ::planus::WriteAsDefault<u8, u8>,
                T2: ::planus::WriteAsDefault<i64, i64>,
                T3: ::planus::WriteAsDefault<u16, u16>,
                T4: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T5: ::planus::WriteAs<::planus::Offset<[u8]>>,
            > ::planus::WriteAsOffset<FrameRecord>
                for FrameRecordBuilder<(T0, T1, T2, T3, T4, T5)>
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<FrameRecord> {
                    let (v0, v1, v2, v3, v4, v5) = &self.0;
                    FrameRecord::create(builder, v0, v1, v2, v3, v4, v5)
                }
            }

            /// Reference to a deserialized [FrameRecord].
            #[derive(Copy, Clone)]
            pub struct FrameRecordRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> FrameRecordRef<'a> {
                /// Getter for the [`lsn` field](FrameRecord#structfield.lsn).
                #[inline]
                pub fn lsn(&self) -> ::planus::Result<u64> {
                    ::core::result::Result::Ok(self.0.access(0, "FrameRecord", "lsn")?.unwrap_or(0))
                }

                /// Getter for the [`kind` field](FrameRecord#structfield.kind).
                #[inline]
                pub fn kind(&self) -> ::planus::Result<u8> {
                    ::core::result::Result::Ok(
                        self.0.access(1, "FrameRecord", "kind")?.unwrap_or(0),
                    )
                }

                /// Getter for the [`wall_timestamp_ns` field](FrameRecord#structfield.wall_timestamp_ns).
                #[inline]
                pub fn wall_timestamp_ns(&self) -> ::planus::Result<i64> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(2, "FrameRecord", "wall_timestamp_ns")?
                            .unwrap_or(0),
                    )
                }

                /// Getter for the [`actor` field](FrameRecord#structfield.actor).
                #[inline]
                pub fn actor(&self) -> ::planus::Result<u16> {
                    ::core::result::Result::Ok(
                        self.0.access(3, "FrameRecord", "actor")?.unwrap_or(0),
                    )
                }

                /// Getter for the [`conversation` field](FrameRecord#structfield.conversation).
                #[inline]
                pub fn conversation(&self) -> ::planus::Result<&'a [u8]> {
                    self.0.access_required(4, "FrameRecord", "conversation")
                }

                /// Getter for the [`payload` field](FrameRecord#structfield.payload).
                #[inline]
                pub fn payload(&self) -> ::planus::Result<&'a [u8]> {
                    self.0.access_required(5, "FrameRecord", "payload")
                }
            }

            impl<'a> ::core::fmt::Debug for FrameRecordRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("FrameRecordRef");
                    f.field("lsn", &self.lsn());
                    f.field("kind", &self.kind());
                    f.field("wall_timestamp_ns", &self.wall_timestamp_ns());
                    f.field("actor", &self.actor());
                    f.field("conversation", &self.conversation());
                    f.field("payload", &self.payload());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<FrameRecordRef<'a>> for FrameRecord {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: FrameRecordRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        lsn: ::core::convert::TryInto::try_into(value.lsn()?)?,
                        kind: ::core::convert::TryInto::try_into(value.kind()?)?,
                        wall_timestamp_ns: ::core::convert::TryInto::try_into(
                            value.wall_timestamp_ns()?,
                        )?,
                        actor: ::core::convert::TryInto::try_into(value.actor()?)?,
                        conversation: value.conversation()?.to_vec(),
                        payload: value.payload()?.to_vec(),
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for FrameRecordRef<'a> {
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

            impl<'a> ::planus::VectorReadInner<'a> for FrameRecordRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[FrameRecordRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<FrameRecord>> for FrameRecord {
                type Value = ::planus::Offset<FrameRecord>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<FrameRecord>],
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

            impl<'a> ::planus::ReadAsRoot<'a> for FrameRecordRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[FrameRecordRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `TranscriptResult` in the namespace `hypermind.protocol`
            ///
            /// Generated from these locations:
            /// * Table `TranscriptResult` in the file `schemas/protocol.fbs:215`
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
            pub struct TranscriptResult {
                /// The field `records` in the table `TranscriptResult`
                pub records: ::planus::alloc::vec::Vec<self::FrameRecord>,
                /// The field `truncated` in the table `TranscriptResult`
                pub truncated: bool,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for TranscriptResult {
                fn default() -> Self {
                    Self {
                        records: ::core::default::Default::default(),
                        truncated: false,
                    }
                }
            }

            impl TranscriptResult {
                /// Creates a [TranscriptResultBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> TranscriptResultBuilder<()> {
                    TranscriptResultBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_records: impl ::planus::WriteAs<
                        ::planus::Offset<[::planus::Offset<self::FrameRecord>]>,
                    >,
                    field_truncated: impl ::planus::WriteAsDefault<bool, bool>,
                ) -> ::planus::Offset<Self> {
                    let prepared_records = field_records.prepare(builder);
                    let prepared_truncated = field_truncated.prepare(builder, &false);

                    let mut table_writer: ::planus::table_writer::TableWriter<8> =
                        ::core::default::Default::default();
                    table_writer
                        .write_entry::<::planus::Offset<[::planus::Offset<self::FrameRecord>]>>(0);
                    if prepared_truncated.is_some() {
                        table_writer.write_entry::<bool>(1);
                    }

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            object_writer.write::<_, _, 4>(&prepared_records);
                            if let ::core::option::Option::Some(prepared_truncated) =
                                prepared_truncated
                            {
                                object_writer.write::<_, _, 1>(&prepared_truncated);
                            }
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<TranscriptResult>> for TranscriptResult {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<TranscriptResult> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<TranscriptResult>> for TranscriptResult {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<TranscriptResult>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<TranscriptResult> for TranscriptResult {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<TranscriptResult> {
                    TranscriptResult::create(builder, &self.records, self.truncated)
                }
            }

            /// Builder for serializing an instance of the [TranscriptResult] type.
            ///
            /// Can be created using the [TranscriptResult::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct TranscriptResultBuilder<State>(State);

            impl TranscriptResultBuilder<()> {
                /// Setter for the [`records` field](TranscriptResult#structfield.records).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn records<T0>(self, value: T0) -> TranscriptResultBuilder<(T0,)>
                where
                    T0: ::planus::WriteAs<::planus::Offset<[::planus::Offset<self::FrameRecord>]>>,
                {
                    TranscriptResultBuilder((value,))
                }
            }

            impl<T0> TranscriptResultBuilder<(T0,)> {
                /// Setter for the [`truncated` field](TranscriptResult#structfield.truncated).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn truncated<T1>(self, value: T1) -> TranscriptResultBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAsDefault<bool, bool>,
                {
                    let (v0,) = self.0;
                    TranscriptResultBuilder((v0, value))
                }

                /// Sets the [`truncated` field](TranscriptResult#structfield.truncated) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn truncated_as_default(
                    self,
                ) -> TranscriptResultBuilder<(T0, ::planus::DefaultValue)> {
                    self.truncated(::planus::DefaultValue)
                }
            }

            impl<T0, T1> TranscriptResultBuilder<(T0, T1)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [TranscriptResult].
                #[inline]
                pub fn finish(
                    self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<TranscriptResult>
                where
                    Self: ::planus::WriteAsOffset<TranscriptResult>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<[::planus::Offset<self::FrameRecord>]>>,
                T1: ::planus::WriteAsDefault<bool, bool>,
            > ::planus::WriteAs<::planus::Offset<TranscriptResult>>
                for TranscriptResultBuilder<(T0, T1)>
            {
                type Prepared = ::planus::Offset<TranscriptResult>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<TranscriptResult> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<[::planus::Offset<self::FrameRecord>]>>,
                T1: ::planus::WriteAsDefault<bool, bool>,
            > ::planus::WriteAsOptional<::planus::Offset<TranscriptResult>>
                for TranscriptResultBuilder<(T0, T1)>
            {
                type Prepared = ::planus::Offset<TranscriptResult>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<TranscriptResult>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAs<::planus::Offset<[::planus::Offset<self::FrameRecord>]>>,
                T1: ::planus::WriteAsDefault<bool, bool>,
            > ::planus::WriteAsOffset<TranscriptResult> for TranscriptResultBuilder<(T0, T1)>
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<TranscriptResult> {
                    let (v0, v1) = &self.0;
                    TranscriptResult::create(builder, v0, v1)
                }
            }

            /// Reference to a deserialized [TranscriptResult].
            #[derive(Copy, Clone)]
            pub struct TranscriptResultRef<'a>(
                #[allow(dead_code)] ::planus::table_reader::Table<'a>,
            );

            impl<'a> TranscriptResultRef<'a> {
                /// Getter for the [`records` field](TranscriptResult#structfield.records).
                #[inline]
                pub fn records(
                    &self,
                ) -> ::planus::Result<
                    ::planus::Vector<'a, ::planus::Result<self::FrameRecordRef<'a>>>,
                > {
                    self.0.access_required(0, "TranscriptResult", "records")
                }

                /// Getter for the [`truncated` field](TranscriptResult#structfield.truncated).
                #[inline]
                pub fn truncated(&self) -> ::planus::Result<bool> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(1, "TranscriptResult", "truncated")?
                            .unwrap_or(false),
                    )
                }
            }

            impl<'a> ::core::fmt::Debug for TranscriptResultRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("TranscriptResultRef");
                    f.field("records", &self.records());
                    f.field("truncated", &self.truncated());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<TranscriptResultRef<'a>> for TranscriptResult {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: TranscriptResultRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        records: value.records()?.to_vec_result()?,
                        truncated: ::core::convert::TryInto::try_into(value.truncated()?)?,
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for TranscriptResultRef<'a> {
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

            impl<'a> ::planus::VectorReadInner<'a> for TranscriptResultRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[TranscriptResultRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<TranscriptResult>> for TranscriptResult {
                type Value = ::planus::Offset<TranscriptResult>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<TranscriptResult>],
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

            impl<'a> ::planus::ReadAsRoot<'a> for TranscriptResultRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[TranscriptResultRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `TemporalWindowRecord` in the namespace `hypermind.protocol`
            ///
            /// Generated from these locations:
            /// * Table `TemporalWindowRecord` in the file `schemas/protocol.fbs:220`
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
            pub struct TemporalWindowRecord {
                /// The field `level` in the table `TemporalWindowRecord`
                pub level: u8,
                /// The field `start_ns` in the table `TemporalWindowRecord`
                pub start_ns: i64,
                /// The field `end_ns` in the table `TemporalWindowRecord`
                pub end_ns: i64,
                /// The field `member_count` in the table `TemporalWindowRecord`
                pub member_count: u64,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for TemporalWindowRecord {
                fn default() -> Self {
                    Self {
                        level: 0,
                        start_ns: 0,
                        end_ns: 0,
                        member_count: 0,
                    }
                }
            }

            impl TemporalWindowRecord {
                /// Creates a [TemporalWindowRecordBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> TemporalWindowRecordBuilder<()> {
                    TemporalWindowRecordBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_level: impl ::planus::WriteAsDefault<u8, u8>,
                    field_start_ns: impl ::planus::WriteAsDefault<i64, i64>,
                    field_end_ns: impl ::planus::WriteAsDefault<i64, i64>,
                    field_member_count: impl ::planus::WriteAsDefault<u64, u64>,
                ) -> ::planus::Offset<Self> {
                    let prepared_level = field_level.prepare(builder, &0);
                    let prepared_start_ns = field_start_ns.prepare(builder, &0);
                    let prepared_end_ns = field_end_ns.prepare(builder, &0);
                    let prepared_member_count = field_member_count.prepare(builder, &0);

                    let mut table_writer: ::planus::table_writer::TableWriter<12> =
                        ::core::default::Default::default();
                    if prepared_start_ns.is_some() {
                        table_writer.write_entry::<i64>(1);
                    }
                    if prepared_end_ns.is_some() {
                        table_writer.write_entry::<i64>(2);
                    }
                    if prepared_member_count.is_some() {
                        table_writer.write_entry::<u64>(3);
                    }
                    if prepared_level.is_some() {
                        table_writer.write_entry::<u8>(0);
                    }

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            if let ::core::option::Option::Some(prepared_start_ns) =
                                prepared_start_ns
                            {
                                object_writer.write::<_, _, 8>(&prepared_start_ns);
                            }
                            if let ::core::option::Option::Some(prepared_end_ns) = prepared_end_ns {
                                object_writer.write::<_, _, 8>(&prepared_end_ns);
                            }
                            if let ::core::option::Option::Some(prepared_member_count) =
                                prepared_member_count
                            {
                                object_writer.write::<_, _, 8>(&prepared_member_count);
                            }
                            if let ::core::option::Option::Some(prepared_level) = prepared_level {
                                object_writer.write::<_, _, 1>(&prepared_level);
                            }
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<TemporalWindowRecord>> for TemporalWindowRecord {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<TemporalWindowRecord> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<TemporalWindowRecord>> for TemporalWindowRecord {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<TemporalWindowRecord>>
                {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<TemporalWindowRecord> for TemporalWindowRecord {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<TemporalWindowRecord> {
                    TemporalWindowRecord::create(
                        builder,
                        self.level,
                        self.start_ns,
                        self.end_ns,
                        self.member_count,
                    )
                }
            }

            /// Builder for serializing an instance of the [TemporalWindowRecord] type.
            ///
            /// Can be created using the [TemporalWindowRecord::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct TemporalWindowRecordBuilder<State>(State);

            impl TemporalWindowRecordBuilder<()> {
                /// Setter for the [`level` field](TemporalWindowRecord#structfield.level).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn level<T0>(self, value: T0) -> TemporalWindowRecordBuilder<(T0,)>
                where
                    T0: ::planus::WriteAsDefault<u8, u8>,
                {
                    TemporalWindowRecordBuilder((value,))
                }

                /// Sets the [`level` field](TemporalWindowRecord#structfield.level) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn level_as_default(
                    self,
                ) -> TemporalWindowRecordBuilder<(::planus::DefaultValue,)> {
                    self.level(::planus::DefaultValue)
                }
            }

            impl<T0> TemporalWindowRecordBuilder<(T0,)> {
                /// Setter for the [`start_ns` field](TemporalWindowRecord#structfield.start_ns).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn start_ns<T1>(self, value: T1) -> TemporalWindowRecordBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAsDefault<i64, i64>,
                {
                    let (v0,) = self.0;
                    TemporalWindowRecordBuilder((v0, value))
                }

                /// Sets the [`start_ns` field](TemporalWindowRecord#structfield.start_ns) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn start_ns_as_default(
                    self,
                ) -> TemporalWindowRecordBuilder<(T0, ::planus::DefaultValue)> {
                    self.start_ns(::planus::DefaultValue)
                }
            }

            impl<T0, T1> TemporalWindowRecordBuilder<(T0, T1)> {
                /// Setter for the [`end_ns` field](TemporalWindowRecord#structfield.end_ns).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn end_ns<T2>(self, value: T2) -> TemporalWindowRecordBuilder<(T0, T1, T2)>
                where
                    T2: ::planus::WriteAsDefault<i64, i64>,
                {
                    let (v0, v1) = self.0;
                    TemporalWindowRecordBuilder((v0, v1, value))
                }

                /// Sets the [`end_ns` field](TemporalWindowRecord#structfield.end_ns) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn end_ns_as_default(
                    self,
                ) -> TemporalWindowRecordBuilder<(T0, T1, ::planus::DefaultValue)> {
                    self.end_ns(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2> TemporalWindowRecordBuilder<(T0, T1, T2)> {
                /// Setter for the [`member_count` field](TemporalWindowRecord#structfield.member_count).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn member_count<T3>(
                    self,
                    value: T3,
                ) -> TemporalWindowRecordBuilder<(T0, T1, T2, T3)>
                where
                    T3: ::planus::WriteAsDefault<u64, u64>,
                {
                    let (v0, v1, v2) = self.0;
                    TemporalWindowRecordBuilder((v0, v1, v2, value))
                }

                /// Sets the [`member_count` field](TemporalWindowRecord#structfield.member_count) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn member_count_as_default(
                    self,
                ) -> TemporalWindowRecordBuilder<(T0, T1, T2, ::planus::DefaultValue)>
                {
                    self.member_count(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2, T3> TemporalWindowRecordBuilder<(T0, T1, T2, T3)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [TemporalWindowRecord].
                #[inline]
                pub fn finish(
                    self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<TemporalWindowRecord>
                where
                    Self: ::planus::WriteAsOffset<TemporalWindowRecord>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u8, u8>,
                T1: ::planus::WriteAsDefault<i64, i64>,
                T2: ::planus::WriteAsDefault<i64, i64>,
                T3: ::planus::WriteAsDefault<u64, u64>,
            > ::planus::WriteAs<::planus::Offset<TemporalWindowRecord>>
                for TemporalWindowRecordBuilder<(T0, T1, T2, T3)>
            {
                type Prepared = ::planus::Offset<TemporalWindowRecord>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<TemporalWindowRecord> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u8, u8>,
                T1: ::planus::WriteAsDefault<i64, i64>,
                T2: ::planus::WriteAsDefault<i64, i64>,
                T3: ::planus::WriteAsDefault<u64, u64>,
            > ::planus::WriteAsOptional<::planus::Offset<TemporalWindowRecord>>
                for TemporalWindowRecordBuilder<(T0, T1, T2, T3)>
            {
                type Prepared = ::planus::Offset<TemporalWindowRecord>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<TemporalWindowRecord>>
                {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u8, u8>,
                T1: ::planus::WriteAsDefault<i64, i64>,
                T2: ::planus::WriteAsDefault<i64, i64>,
                T3: ::planus::WriteAsDefault<u64, u64>,
            > ::planus::WriteAsOffset<TemporalWindowRecord>
                for TemporalWindowRecordBuilder<(T0, T1, T2, T3)>
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<TemporalWindowRecord> {
                    let (v0, v1, v2, v3) = &self.0;
                    TemporalWindowRecord::create(builder, v0, v1, v2, v3)
                }
            }

            /// Reference to a deserialized [TemporalWindowRecord].
            #[derive(Copy, Clone)]
            pub struct TemporalWindowRecordRef<'a>(
                #[allow(dead_code)] ::planus::table_reader::Table<'a>,
            );

            impl<'a> TemporalWindowRecordRef<'a> {
                /// Getter for the [`level` field](TemporalWindowRecord#structfield.level).
                #[inline]
                pub fn level(&self) -> ::planus::Result<u8> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(0, "TemporalWindowRecord", "level")?
                            .unwrap_or(0),
                    )
                }

                /// Getter for the [`start_ns` field](TemporalWindowRecord#structfield.start_ns).
                #[inline]
                pub fn start_ns(&self) -> ::planus::Result<i64> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(1, "TemporalWindowRecord", "start_ns")?
                            .unwrap_or(0),
                    )
                }

                /// Getter for the [`end_ns` field](TemporalWindowRecord#structfield.end_ns).
                #[inline]
                pub fn end_ns(&self) -> ::planus::Result<i64> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(2, "TemporalWindowRecord", "end_ns")?
                            .unwrap_or(0),
                    )
                }

                /// Getter for the [`member_count` field](TemporalWindowRecord#structfield.member_count).
                #[inline]
                pub fn member_count(&self) -> ::planus::Result<u64> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(3, "TemporalWindowRecord", "member_count")?
                            .unwrap_or(0),
                    )
                }
            }

            impl<'a> ::core::fmt::Debug for TemporalWindowRecordRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("TemporalWindowRecordRef");
                    f.field("level", &self.level());
                    f.field("start_ns", &self.start_ns());
                    f.field("end_ns", &self.end_ns());
                    f.field("member_count", &self.member_count());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<TemporalWindowRecordRef<'a>> for TemporalWindowRecord {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: TemporalWindowRecordRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        level: ::core::convert::TryInto::try_into(value.level()?)?,
                        start_ns: ::core::convert::TryInto::try_into(value.start_ns()?)?,
                        end_ns: ::core::convert::TryInto::try_into(value.end_ns()?)?,
                        member_count: ::core::convert::TryInto::try_into(value.member_count()?)?,
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for TemporalWindowRecordRef<'a> {
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

            impl<'a> ::planus::VectorReadInner<'a> for TemporalWindowRecordRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[TemporalWindowRecordRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<TemporalWindowRecord>> for TemporalWindowRecord {
                type Value = ::planus::Offset<TemporalWindowRecord>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<TemporalWindowRecord>],
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

            impl<'a> ::planus::ReadAsRoot<'a> for TemporalWindowRecordRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[TemporalWindowRecordRef]",
                            "read_as_root",
                            0,
                        )
                    })
                }
            }

            /// The table `RecallResult` in the namespace `hypermind.protocol`
            ///
            /// Generated from these locations:
            /// * Table `RecallResult` in the file `schemas/protocol.fbs:227`
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
            pub struct RecallResult {
                /// The field `windows` in the table `RecallResult`
                pub windows:
                    ::core::option::Option<::planus::alloc::vec::Vec<self::TemporalWindowRecord>>,
                /// The field `members` in the table `RecallResult`
                pub members: ::core::option::Option<::planus::alloc::vec::Vec<u64>>,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for RecallResult {
                fn default() -> Self {
                    Self {
                        windows: ::core::default::Default::default(),
                        members: ::core::default::Default::default(),
                    }
                }
            }

            impl RecallResult {
                /// Creates a [RecallResultBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> RecallResultBuilder<()> {
                    RecallResultBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_windows: impl ::planus::WriteAsOptional<
                        ::planus::Offset<[::planus::Offset<self::TemporalWindowRecord>]>,
                    >,
                    field_members: impl ::planus::WriteAsOptional<::planus::Offset<[u64]>>,
                ) -> ::planus::Offset<Self> {
                    let prepared_windows = field_windows.prepare(builder);
                    let prepared_members = field_members.prepare(builder);

                    let mut table_writer: ::planus::table_writer::TableWriter<8> =
                        ::core::default::Default::default();
                    if prepared_windows.is_some() {
                        table_writer.write_entry::<::planus::Offset<
                            [::planus::Offset<self::TemporalWindowRecord>],
                        >>(0);
                    }
                    if prepared_members.is_some() {
                        table_writer.write_entry::<::planus::Offset<[u64]>>(1);
                    }

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            if let ::core::option::Option::Some(prepared_windows) = prepared_windows
                            {
                                object_writer.write::<_, _, 4>(&prepared_windows);
                            }
                            if let ::core::option::Option::Some(prepared_members) = prepared_members
                            {
                                object_writer.write::<_, _, 4>(&prepared_members);
                            }
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<RecallResult>> for RecallResult {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<RecallResult> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<RecallResult>> for RecallResult {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<RecallResult>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<RecallResult> for RecallResult {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<RecallResult> {
                    RecallResult::create(builder, &self.windows, &self.members)
                }
            }

            /// Builder for serializing an instance of the [RecallResult] type.
            ///
            /// Can be created using the [RecallResult::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct RecallResultBuilder<State>(State);

            impl RecallResultBuilder<()> {
                /// Setter for the [`windows` field](RecallResult#structfield.windows).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn windows<T0>(self, value: T0) -> RecallResultBuilder<(T0,)>
                where
                    T0: ::planus::WriteAsOptional<
                            ::planus::Offset<[::planus::Offset<self::TemporalWindowRecord>]>,
                        >,
                {
                    RecallResultBuilder((value,))
                }

                /// Sets the [`windows` field](RecallResult#structfield.windows) to null.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn windows_as_null(self) -> RecallResultBuilder<((),)> {
                    self.windows(())
                }
            }

            impl<T0> RecallResultBuilder<(T0,)> {
                /// Setter for the [`members` field](RecallResult#structfield.members).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn members<T1>(self, value: T1) -> RecallResultBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAsOptional<::planus::Offset<[u64]>>,
                {
                    let (v0,) = self.0;
                    RecallResultBuilder((v0, value))
                }

                /// Sets the [`members` field](RecallResult#structfield.members) to null.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn members_as_null(self) -> RecallResultBuilder<(T0, ())> {
                    self.members(())
                }
            }

            impl<T0, T1> RecallResultBuilder<(T0, T1)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [RecallResult].
                #[inline]
                pub fn finish(
                    self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<RecallResult>
                where
                    Self: ::planus::WriteAsOffset<RecallResult>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsOptional<
                        ::planus::Offset<[::planus::Offset<self::TemporalWindowRecord>]>,
                    >,
                T1: ::planus::WriteAsOptional<::planus::Offset<[u64]>>,
            > ::planus::WriteAs<::planus::Offset<RecallResult>> for RecallResultBuilder<(T0, T1)>
            {
                type Prepared = ::planus::Offset<RecallResult>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<RecallResult> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsOptional<
                        ::planus::Offset<[::planus::Offset<self::TemporalWindowRecord>]>,
                    >,
                T1: ::planus::WriteAsOptional<::planus::Offset<[u64]>>,
            > ::planus::WriteAsOptional<::planus::Offset<RecallResult>>
                for RecallResultBuilder<(T0, T1)>
            {
                type Prepared = ::planus::Offset<RecallResult>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<RecallResult>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAsOptional<
                        ::planus::Offset<[::planus::Offset<self::TemporalWindowRecord>]>,
                    >,
                T1: ::planus::WriteAsOptional<::planus::Offset<[u64]>>,
            > ::planus::WriteAsOffset<RecallResult> for RecallResultBuilder<(T0, T1)>
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<RecallResult> {
                    let (v0, v1) = &self.0;
                    RecallResult::create(builder, v0, v1)
                }
            }

            /// Reference to a deserialized [RecallResult].
            #[derive(Copy, Clone)]
            pub struct RecallResultRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> RecallResultRef<'a> {
                /// Getter for the [`windows` field](RecallResult#structfield.windows).
                #[inline]
                pub fn windows(
                    &self,
                ) -> ::planus::Result<
                    ::core::option::Option<
                        ::planus::Vector<'a, ::planus::Result<self::TemporalWindowRecordRef<'a>>>,
                    >,
                > {
                    self.0.access(0, "RecallResult", "windows")
                }

                /// Getter for the [`members` field](RecallResult#structfield.members).
                #[inline]
                pub fn members(
                    &self,
                ) -> ::planus::Result<::core::option::Option<::planus::Vector<'a, u64>>>
                {
                    self.0.access(1, "RecallResult", "members")
                }
            }

            impl<'a> ::core::fmt::Debug for RecallResultRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("RecallResultRef");
                    if let ::core::option::Option::Some(field_windows) = self.windows().transpose()
                    {
                        f.field("windows", &field_windows);
                    }
                    if let ::core::option::Option::Some(field_members) = self.members().transpose()
                    {
                        f.field("members", &field_members);
                    }
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<RecallResultRef<'a>> for RecallResult {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: RecallResultRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        windows: if let ::core::option::Option::Some(windows) = value.windows()? {
                            ::core::option::Option::Some(windows.to_vec_result()?)
                        } else {
                            ::core::option::Option::None
                        },
                        members: if let ::core::option::Option::Some(members) = value.members()? {
                            ::core::option::Option::Some(members.to_vec()?)
                        } else {
                            ::core::option::Option::None
                        },
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for RecallResultRef<'a> {
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

            impl<'a> ::planus::VectorReadInner<'a> for RecallResultRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[RecallResultRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<RecallResult>> for RecallResult {
                type Value = ::planus::Offset<RecallResult>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<RecallResult>],
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

            impl<'a> ::planus::ReadAsRoot<'a> for RecallResultRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[RecallResultRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `BeliefProvenanceRecord` in the namespace `hypermind.protocol`
            ///
            /// Generated from these locations:
            /// * Table `BeliefProvenanceRecord` in the file `schemas/protocol.fbs:232`
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
            pub struct BeliefProvenanceRecord {
                /// The field `first_lsn` in the table `BeliefProvenanceRecord`
                pub first_lsn: u64,
                /// The field `last_lsn` in the table `BeliefProvenanceRecord`
                pub last_lsn: u64,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for BeliefProvenanceRecord {
                fn default() -> Self {
                    Self {
                        first_lsn: 0,
                        last_lsn: 0,
                    }
                }
            }

            impl BeliefProvenanceRecord {
                /// Creates a [BeliefProvenanceRecordBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> BeliefProvenanceRecordBuilder<()> {
                    BeliefProvenanceRecordBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_first_lsn: impl ::planus::WriteAsDefault<u64, u64>,
                    field_last_lsn: impl ::planus::WriteAsDefault<u64, u64>,
                ) -> ::planus::Offset<Self> {
                    let prepared_first_lsn = field_first_lsn.prepare(builder, &0);
                    let prepared_last_lsn = field_last_lsn.prepare(builder, &0);

                    let mut table_writer: ::planus::table_writer::TableWriter<8> =
                        ::core::default::Default::default();
                    if prepared_first_lsn.is_some() {
                        table_writer.write_entry::<u64>(0);
                    }
                    if prepared_last_lsn.is_some() {
                        table_writer.write_entry::<u64>(1);
                    }

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            if let ::core::option::Option::Some(prepared_first_lsn) =
                                prepared_first_lsn
                            {
                                object_writer.write::<_, _, 8>(&prepared_first_lsn);
                            }
                            if let ::core::option::Option::Some(prepared_last_lsn) =
                                prepared_last_lsn
                            {
                                object_writer.write::<_, _, 8>(&prepared_last_lsn);
                            }
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<BeliefProvenanceRecord>> for BeliefProvenanceRecord {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<BeliefProvenanceRecord> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<BeliefProvenanceRecord>>
                for BeliefProvenanceRecord
            {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<BeliefProvenanceRecord>>
                {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<BeliefProvenanceRecord> for BeliefProvenanceRecord {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<BeliefProvenanceRecord> {
                    BeliefProvenanceRecord::create(builder, self.first_lsn, self.last_lsn)
                }
            }

            /// Builder for serializing an instance of the [BeliefProvenanceRecord] type.
            ///
            /// Can be created using the [BeliefProvenanceRecord::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct BeliefProvenanceRecordBuilder<State>(State);

            impl BeliefProvenanceRecordBuilder<()> {
                /// Setter for the [`first_lsn` field](BeliefProvenanceRecord#structfield.first_lsn).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn first_lsn<T0>(self, value: T0) -> BeliefProvenanceRecordBuilder<(T0,)>
                where
                    T0: ::planus::WriteAsDefault<u64, u64>,
                {
                    BeliefProvenanceRecordBuilder((value,))
                }

                /// Sets the [`first_lsn` field](BeliefProvenanceRecord#structfield.first_lsn) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn first_lsn_as_default(
                    self,
                ) -> BeliefProvenanceRecordBuilder<(::planus::DefaultValue,)> {
                    self.first_lsn(::planus::DefaultValue)
                }
            }

            impl<T0> BeliefProvenanceRecordBuilder<(T0,)> {
                /// Setter for the [`last_lsn` field](BeliefProvenanceRecord#structfield.last_lsn).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn last_lsn<T1>(self, value: T1) -> BeliefProvenanceRecordBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAsDefault<u64, u64>,
                {
                    let (v0,) = self.0;
                    BeliefProvenanceRecordBuilder((v0, value))
                }

                /// Sets the [`last_lsn` field](BeliefProvenanceRecord#structfield.last_lsn) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn last_lsn_as_default(
                    self,
                ) -> BeliefProvenanceRecordBuilder<(T0, ::planus::DefaultValue)> {
                    self.last_lsn(::planus::DefaultValue)
                }
            }

            impl<T0, T1> BeliefProvenanceRecordBuilder<(T0, T1)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [BeliefProvenanceRecord].
                #[inline]
                pub fn finish(
                    self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<BeliefProvenanceRecord>
                where
                    Self: ::planus::WriteAsOffset<BeliefProvenanceRecord>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<T0: ::planus::WriteAsDefault<u64, u64>, T1: ::planus::WriteAsDefault<u64, u64>>
                ::planus::WriteAs<::planus::Offset<BeliefProvenanceRecord>>
                for BeliefProvenanceRecordBuilder<(T0, T1)>
            {
                type Prepared = ::planus::Offset<BeliefProvenanceRecord>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<BeliefProvenanceRecord> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<T0: ::planus::WriteAsDefault<u64, u64>, T1: ::planus::WriteAsDefault<u64, u64>>
                ::planus::WriteAsOptional<::planus::Offset<BeliefProvenanceRecord>>
                for BeliefProvenanceRecordBuilder<(T0, T1)>
            {
                type Prepared = ::planus::Offset<BeliefProvenanceRecord>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<BeliefProvenanceRecord>>
                {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<T0: ::planus::WriteAsDefault<u64, u64>, T1: ::planus::WriteAsDefault<u64, u64>>
                ::planus::WriteAsOffset<BeliefProvenanceRecord>
                for BeliefProvenanceRecordBuilder<(T0, T1)>
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<BeliefProvenanceRecord> {
                    let (v0, v1) = &self.0;
                    BeliefProvenanceRecord::create(builder, v0, v1)
                }
            }

            /// Reference to a deserialized [BeliefProvenanceRecord].
            #[derive(Copy, Clone)]
            pub struct BeliefProvenanceRecordRef<'a>(
                #[allow(dead_code)] ::planus::table_reader::Table<'a>,
            );

            impl<'a> BeliefProvenanceRecordRef<'a> {
                /// Getter for the [`first_lsn` field](BeliefProvenanceRecord#structfield.first_lsn).
                #[inline]
                pub fn first_lsn(&self) -> ::planus::Result<u64> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(0, "BeliefProvenanceRecord", "first_lsn")?
                            .unwrap_or(0),
                    )
                }

                /// Getter for the [`last_lsn` field](BeliefProvenanceRecord#structfield.last_lsn).
                #[inline]
                pub fn last_lsn(&self) -> ::planus::Result<u64> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(1, "BeliefProvenanceRecord", "last_lsn")?
                            .unwrap_or(0),
                    )
                }
            }

            impl<'a> ::core::fmt::Debug for BeliefProvenanceRecordRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("BeliefProvenanceRecordRef");
                    f.field("first_lsn", &self.first_lsn());
                    f.field("last_lsn", &self.last_lsn());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<BeliefProvenanceRecordRef<'a>> for BeliefProvenanceRecord {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: BeliefProvenanceRecordRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        first_lsn: ::core::convert::TryInto::try_into(value.first_lsn()?)?,
                        last_lsn: ::core::convert::TryInto::try_into(value.last_lsn()?)?,
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for BeliefProvenanceRecordRef<'a> {
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

            impl<'a> ::planus::VectorReadInner<'a> for BeliefProvenanceRecordRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[BeliefProvenanceRecordRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<BeliefProvenanceRecord>>
                for BeliefProvenanceRecord
            {
                type Value = ::planus::Offset<BeliefProvenanceRecord>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<BeliefProvenanceRecord>],
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

            impl<'a> ::planus::ReadAsRoot<'a> for BeliefProvenanceRecordRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[BeliefProvenanceRecordRef]",
                            "read_as_root",
                            0,
                        )
                    })
                }
            }

            /// The table `BeliefConflictRecord` in the namespace `hypermind.protocol`
            ///
            /// Generated from these locations:
            /// * Table `BeliefConflictRecord` in the file `schemas/protocol.fbs:237`
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
            pub struct BeliefConflictRecord {
                /// The field `other_type` in the table `BeliefConflictRecord`
                pub other_type: u8,
                /// The field `other_canonical_identity` in the table `BeliefConflictRecord`
                pub other_canonical_identity: ::planus::alloc::string::String,
                /// The field `created_lsn` in the table `BeliefConflictRecord`
                pub created_lsn: u64,
                /// The field `resolved_lsn` in the table `BeliefConflictRecord`
                pub resolved_lsn: u64,
                /// The field `obligated_surfacing` in the table `BeliefConflictRecord`
                pub obligated_surfacing: bool,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for BeliefConflictRecord {
                fn default() -> Self {
                    Self {
                        other_type: 0,
                        other_canonical_identity: ::core::default::Default::default(),
                        created_lsn: 0,
                        resolved_lsn: 0,
                        obligated_surfacing: false,
                    }
                }
            }

            impl BeliefConflictRecord {
                /// Creates a [BeliefConflictRecordBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> BeliefConflictRecordBuilder<()> {
                    BeliefConflictRecordBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_other_type: impl ::planus::WriteAsDefault<u8, u8>,
                    field_other_canonical_identity: impl ::planus::WriteAs<::planus::Offset<str>>,
                    field_created_lsn: impl ::planus::WriteAsDefault<u64, u64>,
                    field_resolved_lsn: impl ::planus::WriteAsDefault<u64, u64>,
                    field_obligated_surfacing: impl ::planus::WriteAsDefault<bool, bool>,
                ) -> ::planus::Offset<Self> {
                    let prepared_other_type = field_other_type.prepare(builder, &0);
                    let prepared_other_canonical_identity =
                        field_other_canonical_identity.prepare(builder);
                    let prepared_created_lsn = field_created_lsn.prepare(builder, &0);
                    let prepared_resolved_lsn = field_resolved_lsn.prepare(builder, &0);
                    let prepared_obligated_surfacing =
                        field_obligated_surfacing.prepare(builder, &false);

                    let mut table_writer: ::planus::table_writer::TableWriter<14> =
                        ::core::default::Default::default();
                    if prepared_created_lsn.is_some() {
                        table_writer.write_entry::<u64>(2);
                    }
                    if prepared_resolved_lsn.is_some() {
                        table_writer.write_entry::<u64>(3);
                    }
                    table_writer.write_entry::<::planus::Offset<str>>(1);
                    if prepared_other_type.is_some() {
                        table_writer.write_entry::<u8>(0);
                    }
                    if prepared_obligated_surfacing.is_some() {
                        table_writer.write_entry::<bool>(4);
                    }

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            if let ::core::option::Option::Some(prepared_created_lsn) =
                                prepared_created_lsn
                            {
                                object_writer.write::<_, _, 8>(&prepared_created_lsn);
                            }
                            if let ::core::option::Option::Some(prepared_resolved_lsn) =
                                prepared_resolved_lsn
                            {
                                object_writer.write::<_, _, 8>(&prepared_resolved_lsn);
                            }
                            object_writer.write::<_, _, 4>(&prepared_other_canonical_identity);
                            if let ::core::option::Option::Some(prepared_other_type) =
                                prepared_other_type
                            {
                                object_writer.write::<_, _, 1>(&prepared_other_type);
                            }
                            if let ::core::option::Option::Some(prepared_obligated_surfacing) =
                                prepared_obligated_surfacing
                            {
                                object_writer.write::<_, _, 1>(&prepared_obligated_surfacing);
                            }
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<BeliefConflictRecord>> for BeliefConflictRecord {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<BeliefConflictRecord> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<BeliefConflictRecord>> for BeliefConflictRecord {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<BeliefConflictRecord>>
                {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<BeliefConflictRecord> for BeliefConflictRecord {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<BeliefConflictRecord> {
                    BeliefConflictRecord::create(
                        builder,
                        self.other_type,
                        &self.other_canonical_identity,
                        self.created_lsn,
                        self.resolved_lsn,
                        self.obligated_surfacing,
                    )
                }
            }

            /// Builder for serializing an instance of the [BeliefConflictRecord] type.
            ///
            /// Can be created using the [BeliefConflictRecord::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct BeliefConflictRecordBuilder<State>(State);

            impl BeliefConflictRecordBuilder<()> {
                /// Setter for the [`other_type` field](BeliefConflictRecord#structfield.other_type).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn other_type<T0>(self, value: T0) -> BeliefConflictRecordBuilder<(T0,)>
                where
                    T0: ::planus::WriteAsDefault<u8, u8>,
                {
                    BeliefConflictRecordBuilder((value,))
                }

                /// Sets the [`other_type` field](BeliefConflictRecord#structfield.other_type) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn other_type_as_default(
                    self,
                ) -> BeliefConflictRecordBuilder<(::planus::DefaultValue,)> {
                    self.other_type(::planus::DefaultValue)
                }
            }

            impl<T0> BeliefConflictRecordBuilder<(T0,)> {
                /// Setter for the [`other_canonical_identity` field](BeliefConflictRecord#structfield.other_canonical_identity).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn other_canonical_identity<T1>(
                    self,
                    value: T1,
                ) -> BeliefConflictRecordBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAs<::planus::Offset<str>>,
                {
                    let (v0,) = self.0;
                    BeliefConflictRecordBuilder((v0, value))
                }
            }

            impl<T0, T1> BeliefConflictRecordBuilder<(T0, T1)> {
                /// Setter for the [`created_lsn` field](BeliefConflictRecord#structfield.created_lsn).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn created_lsn<T2>(self, value: T2) -> BeliefConflictRecordBuilder<(T0, T1, T2)>
                where
                    T2: ::planus::WriteAsDefault<u64, u64>,
                {
                    let (v0, v1) = self.0;
                    BeliefConflictRecordBuilder((v0, v1, value))
                }

                /// Sets the [`created_lsn` field](BeliefConflictRecord#structfield.created_lsn) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn created_lsn_as_default(
                    self,
                ) -> BeliefConflictRecordBuilder<(T0, T1, ::planus::DefaultValue)> {
                    self.created_lsn(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2> BeliefConflictRecordBuilder<(T0, T1, T2)> {
                /// Setter for the [`resolved_lsn` field](BeliefConflictRecord#structfield.resolved_lsn).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn resolved_lsn<T3>(
                    self,
                    value: T3,
                ) -> BeliefConflictRecordBuilder<(T0, T1, T2, T3)>
                where
                    T3: ::planus::WriteAsDefault<u64, u64>,
                {
                    let (v0, v1, v2) = self.0;
                    BeliefConflictRecordBuilder((v0, v1, v2, value))
                }

                /// Sets the [`resolved_lsn` field](BeliefConflictRecord#structfield.resolved_lsn) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn resolved_lsn_as_default(
                    self,
                ) -> BeliefConflictRecordBuilder<(T0, T1, T2, ::planus::DefaultValue)>
                {
                    self.resolved_lsn(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2, T3> BeliefConflictRecordBuilder<(T0, T1, T2, T3)> {
                /// Setter for the [`obligated_surfacing` field](BeliefConflictRecord#structfield.obligated_surfacing).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn obligated_surfacing<T4>(
                    self,
                    value: T4,
                ) -> BeliefConflictRecordBuilder<(T0, T1, T2, T3, T4)>
                where
                    T4: ::planus::WriteAsDefault<bool, bool>,
                {
                    let (v0, v1, v2, v3) = self.0;
                    BeliefConflictRecordBuilder((v0, v1, v2, v3, value))
                }

                /// Sets the [`obligated_surfacing` field](BeliefConflictRecord#structfield.obligated_surfacing) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn obligated_surfacing_as_default(
                    self,
                ) -> BeliefConflictRecordBuilder<(T0, T1, T2, T3, ::planus::DefaultValue)>
                {
                    self.obligated_surfacing(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2, T3, T4> BeliefConflictRecordBuilder<(T0, T1, T2, T3, T4)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [BeliefConflictRecord].
                #[inline]
                pub fn finish(
                    self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<BeliefConflictRecord>
                where
                    Self: ::planus::WriteAsOffset<BeliefConflictRecord>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u8, u8>,
                T1: ::planus::WriteAs<::planus::Offset<str>>,
                T2: ::planus::WriteAsDefault<u64, u64>,
                T3: ::planus::WriteAsDefault<u64, u64>,
                T4: ::planus::WriteAsDefault<bool, bool>,
            > ::planus::WriteAs<::planus::Offset<BeliefConflictRecord>>
                for BeliefConflictRecordBuilder<(T0, T1, T2, T3, T4)>
            {
                type Prepared = ::planus::Offset<BeliefConflictRecord>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<BeliefConflictRecord> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u8, u8>,
                T1: ::planus::WriteAs<::planus::Offset<str>>,
                T2: ::planus::WriteAsDefault<u64, u64>,
                T3: ::planus::WriteAsDefault<u64, u64>,
                T4: ::planus::WriteAsDefault<bool, bool>,
            > ::planus::WriteAsOptional<::planus::Offset<BeliefConflictRecord>>
                for BeliefConflictRecordBuilder<(T0, T1, T2, T3, T4)>
            {
                type Prepared = ::planus::Offset<BeliefConflictRecord>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<BeliefConflictRecord>>
                {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u8, u8>,
                T1: ::planus::WriteAs<::planus::Offset<str>>,
                T2: ::planus::WriteAsDefault<u64, u64>,
                T3: ::planus::WriteAsDefault<u64, u64>,
                T4: ::planus::WriteAsDefault<bool, bool>,
            > ::planus::WriteAsOffset<BeliefConflictRecord>
                for BeliefConflictRecordBuilder<(T0, T1, T2, T3, T4)>
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<BeliefConflictRecord> {
                    let (v0, v1, v2, v3, v4) = &self.0;
                    BeliefConflictRecord::create(builder, v0, v1, v2, v3, v4)
                }
            }

            /// Reference to a deserialized [BeliefConflictRecord].
            #[derive(Copy, Clone)]
            pub struct BeliefConflictRecordRef<'a>(
                #[allow(dead_code)] ::planus::table_reader::Table<'a>,
            );

            impl<'a> BeliefConflictRecordRef<'a> {
                /// Getter for the [`other_type` field](BeliefConflictRecord#structfield.other_type).
                #[inline]
                pub fn other_type(&self) -> ::planus::Result<u8> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(0, "BeliefConflictRecord", "other_type")?
                            .unwrap_or(0),
                    )
                }

                /// Getter for the [`other_canonical_identity` field](BeliefConflictRecord#structfield.other_canonical_identity).
                #[inline]
                pub fn other_canonical_identity(
                    &self,
                ) -> ::planus::Result<&'a ::core::primitive::str> {
                    self.0
                        .access_required(1, "BeliefConflictRecord", "other_canonical_identity")
                }

                /// Getter for the [`created_lsn` field](BeliefConflictRecord#structfield.created_lsn).
                #[inline]
                pub fn created_lsn(&self) -> ::planus::Result<u64> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(2, "BeliefConflictRecord", "created_lsn")?
                            .unwrap_or(0),
                    )
                }

                /// Getter for the [`resolved_lsn` field](BeliefConflictRecord#structfield.resolved_lsn).
                #[inline]
                pub fn resolved_lsn(&self) -> ::planus::Result<u64> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(3, "BeliefConflictRecord", "resolved_lsn")?
                            .unwrap_or(0),
                    )
                }

                /// Getter for the [`obligated_surfacing` field](BeliefConflictRecord#structfield.obligated_surfacing).
                #[inline]
                pub fn obligated_surfacing(&self) -> ::planus::Result<bool> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(4, "BeliefConflictRecord", "obligated_surfacing")?
                            .unwrap_or(false),
                    )
                }
            }

            impl<'a> ::core::fmt::Debug for BeliefConflictRecordRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("BeliefConflictRecordRef");
                    f.field("other_type", &self.other_type());
                    f.field("other_canonical_identity", &self.other_canonical_identity());
                    f.field("created_lsn", &self.created_lsn());
                    f.field("resolved_lsn", &self.resolved_lsn());
                    f.field("obligated_surfacing", &self.obligated_surfacing());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<BeliefConflictRecordRef<'a>> for BeliefConflictRecord {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: BeliefConflictRecordRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        other_type: ::core::convert::TryInto::try_into(value.other_type()?)?,
                        other_canonical_identity: ::core::convert::Into::into(
                            value.other_canonical_identity()?,
                        ),
                        created_lsn: ::core::convert::TryInto::try_into(value.created_lsn()?)?,
                        resolved_lsn: ::core::convert::TryInto::try_into(value.resolved_lsn()?)?,
                        obligated_surfacing: ::core::convert::TryInto::try_into(
                            value.obligated_surfacing()?,
                        )?,
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for BeliefConflictRecordRef<'a> {
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

            impl<'a> ::planus::VectorReadInner<'a> for BeliefConflictRecordRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[BeliefConflictRecordRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<BeliefConflictRecord>> for BeliefConflictRecord {
                type Value = ::planus::Offset<BeliefConflictRecord>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<BeliefConflictRecord>],
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

            impl<'a> ::planus::ReadAsRoot<'a> for BeliefConflictRecordRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[BeliefConflictRecordRef]",
                            "read_as_root",
                            0,
                        )
                    })
                }
            }

            /// The table `BeliefResult` in the namespace `hypermind.protocol`
            ///
            /// Generated from these locations:
            /// * Table `BeliefResult` in the file `schemas/protocol.fbs:245`
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
            pub struct BeliefResult {
                /// The field `present` in the table `BeliefResult`
                pub present: bool,
                /// The field `belief_type` in the table `BeliefResult`
                pub belief_type: u8,
                /// The field `belief_id` in the table `BeliefResult`
                pub belief_id: ::core::option::Option<::planus::alloc::vec::Vec<u8>>,
                /// The field `canonical_identity` in the table `BeliefResult`
                pub canonical_identity: ::core::option::Option<::planus::alloc::string::String>,
                /// The field `conflict_domain` in the table `BeliefResult`
                pub conflict_domain: ::core::option::Option<::planus::alloc::string::String>,
                /// The field `value` in the table `BeliefResult`
                pub value: ::core::option::Option<::planus::alloc::vec::Vec<u8>>,
                /// The field `claim` in the table `BeliefResult`
                pub claim: u8,
                /// The field `valid_from_ns` in the table `BeliefResult`
                pub valid_from_ns: i64,
                /// The field `valid_to_ns` in the table `BeliefResult`
                pub valid_to_ns: i64,
                /// The field `transaction_lsn` in the table `BeliefResult`
                pub transaction_lsn: u64,
                /// The field `version` in the table `BeliefResult`
                pub version: u64,
                /// The field `supersedes_version` in the table `BeliefResult`
                pub supersedes_version: u64,
                /// The field `provenance` in the table `BeliefResult`
                pub provenance:
                    ::core::option::Option<::planus::alloc::vec::Vec<self::BeliefProvenanceRecord>>,
                /// The field `conflict_edges` in the table `BeliefResult`
                pub conflict_edges:
                    ::core::option::Option<::planus::alloc::vec::Vec<self::BeliefConflictRecord>>,
                /// The field `tombstoned` in the table `BeliefResult`
                pub tombstoned: bool,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for BeliefResult {
                fn default() -> Self {
                    Self {
                        present: false,
                        belief_type: 0,
                        belief_id: ::core::default::Default::default(),
                        canonical_identity: ::core::default::Default::default(),
                        conflict_domain: ::core::default::Default::default(),
                        value: ::core::default::Default::default(),
                        claim: 0,
                        valid_from_ns: 0,
                        valid_to_ns: 0,
                        transaction_lsn: 0,
                        version: 0,
                        supersedes_version: 0,
                        provenance: ::core::default::Default::default(),
                        conflict_edges: ::core::default::Default::default(),
                        tombstoned: false,
                    }
                }
            }

            impl BeliefResult {
                /// Creates a [BeliefResultBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> BeliefResultBuilder<()> {
                    BeliefResultBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_present: impl ::planus::WriteAsDefault<bool, bool>,
                    field_belief_type: impl ::planus::WriteAsDefault<u8, u8>,
                    field_belief_id: impl ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                    field_canonical_identity: impl ::planus::WriteAsOptional<
                        ::planus::Offset<::core::primitive::str>,
                    >,
                    field_conflict_domain: impl ::planus::WriteAsOptional<
                        ::planus::Offset<::core::primitive::str>,
                    >,
                    field_value: impl ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                    field_claim: impl ::planus::WriteAsDefault<u8, u8>,
                    field_valid_from_ns: impl ::planus::WriteAsDefault<i64, i64>,
                    field_valid_to_ns: impl ::planus::WriteAsDefault<i64, i64>,
                    field_transaction_lsn: impl ::planus::WriteAsDefault<u64, u64>,
                    field_version: impl ::planus::WriteAsDefault<u64, u64>,
                    field_supersedes_version: impl ::planus::WriteAsDefault<u64, u64>,
                    field_provenance: impl ::planus::WriteAsOptional<
                        ::planus::Offset<[::planus::Offset<self::BeliefProvenanceRecord>]>,
                    >,
                    field_conflict_edges: impl ::planus::WriteAsOptional<
                        ::planus::Offset<[::planus::Offset<self::BeliefConflictRecord>]>,
                    >,
                    field_tombstoned: impl ::planus::WriteAsDefault<bool, bool>,
                ) -> ::planus::Offset<Self> {
                    let prepared_present = field_present.prepare(builder, &false);
                    let prepared_belief_type = field_belief_type.prepare(builder, &0);
                    let prepared_belief_id = field_belief_id.prepare(builder);
                    let prepared_canonical_identity = field_canonical_identity.prepare(builder);
                    let prepared_conflict_domain = field_conflict_domain.prepare(builder);
                    let prepared_value = field_value.prepare(builder);
                    let prepared_claim = field_claim.prepare(builder, &0);
                    let prepared_valid_from_ns = field_valid_from_ns.prepare(builder, &0);
                    let prepared_valid_to_ns = field_valid_to_ns.prepare(builder, &0);
                    let prepared_transaction_lsn = field_transaction_lsn.prepare(builder, &0);
                    let prepared_version = field_version.prepare(builder, &0);
                    let prepared_supersedes_version = field_supersedes_version.prepare(builder, &0);
                    let prepared_provenance = field_provenance.prepare(builder);
                    let prepared_conflict_edges = field_conflict_edges.prepare(builder);
                    let prepared_tombstoned = field_tombstoned.prepare(builder, &false);

                    let mut table_writer: ::planus::table_writer::TableWriter<34> =
                        ::core::default::Default::default();
                    if prepared_valid_from_ns.is_some() {
                        table_writer.write_entry::<i64>(7);
                    }
                    if prepared_valid_to_ns.is_some() {
                        table_writer.write_entry::<i64>(8);
                    }
                    if prepared_transaction_lsn.is_some() {
                        table_writer.write_entry::<u64>(9);
                    }
                    if prepared_version.is_some() {
                        table_writer.write_entry::<u64>(10);
                    }
                    if prepared_supersedes_version.is_some() {
                        table_writer.write_entry::<u64>(11);
                    }
                    if prepared_belief_id.is_some() {
                        table_writer.write_entry::<::planus::Offset<[u8]>>(2);
                    }
                    if prepared_canonical_identity.is_some() {
                        table_writer.write_entry::<::planus::Offset<str>>(3);
                    }
                    if prepared_conflict_domain.is_some() {
                        table_writer.write_entry::<::planus::Offset<str>>(4);
                    }
                    if prepared_value.is_some() {
                        table_writer.write_entry::<::planus::Offset<[u8]>>(5);
                    }
                    if prepared_provenance.is_some() {
                        table_writer.write_entry::<::planus::Offset<
                            [::planus::Offset<self::BeliefProvenanceRecord>],
                        >>(12);
                    }
                    if prepared_conflict_edges.is_some() {
                        table_writer.write_entry::<::planus::Offset<
                            [::planus::Offset<self::BeliefConflictRecord>],
                        >>(13);
                    }
                    if prepared_present.is_some() {
                        table_writer.write_entry::<bool>(0);
                    }
                    if prepared_belief_type.is_some() {
                        table_writer.write_entry::<u8>(1);
                    }
                    if prepared_claim.is_some() {
                        table_writer.write_entry::<u8>(6);
                    }
                    if prepared_tombstoned.is_some() {
                        table_writer.write_entry::<bool>(14);
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
                            if let ::core::option::Option::Some(prepared_transaction_lsn) =
                                prepared_transaction_lsn
                            {
                                object_writer.write::<_, _, 8>(&prepared_transaction_lsn);
                            }
                            if let ::core::option::Option::Some(prepared_version) = prepared_version
                            {
                                object_writer.write::<_, _, 8>(&prepared_version);
                            }
                            if let ::core::option::Option::Some(prepared_supersedes_version) =
                                prepared_supersedes_version
                            {
                                object_writer.write::<_, _, 8>(&prepared_supersedes_version);
                            }
                            if let ::core::option::Option::Some(prepared_belief_id) =
                                prepared_belief_id
                            {
                                object_writer.write::<_, _, 4>(&prepared_belief_id);
                            }
                            if let ::core::option::Option::Some(prepared_canonical_identity) =
                                prepared_canonical_identity
                            {
                                object_writer.write::<_, _, 4>(&prepared_canonical_identity);
                            }
                            if let ::core::option::Option::Some(prepared_conflict_domain) =
                                prepared_conflict_domain
                            {
                                object_writer.write::<_, _, 4>(&prepared_conflict_domain);
                            }
                            if let ::core::option::Option::Some(prepared_value) = prepared_value {
                                object_writer.write::<_, _, 4>(&prepared_value);
                            }
                            if let ::core::option::Option::Some(prepared_provenance) =
                                prepared_provenance
                            {
                                object_writer.write::<_, _, 4>(&prepared_provenance);
                            }
                            if let ::core::option::Option::Some(prepared_conflict_edges) =
                                prepared_conflict_edges
                            {
                                object_writer.write::<_, _, 4>(&prepared_conflict_edges);
                            }
                            if let ::core::option::Option::Some(prepared_present) = prepared_present
                            {
                                object_writer.write::<_, _, 1>(&prepared_present);
                            }
                            if let ::core::option::Option::Some(prepared_belief_type) =
                                prepared_belief_type
                            {
                                object_writer.write::<_, _, 1>(&prepared_belief_type);
                            }
                            if let ::core::option::Option::Some(prepared_claim) = prepared_claim {
                                object_writer.write::<_, _, 1>(&prepared_claim);
                            }
                            if let ::core::option::Option::Some(prepared_tombstoned) =
                                prepared_tombstoned
                            {
                                object_writer.write::<_, _, 1>(&prepared_tombstoned);
                            }
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<BeliefResult>> for BeliefResult {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<BeliefResult> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<BeliefResult>> for BeliefResult {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<BeliefResult>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<BeliefResult> for BeliefResult {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<BeliefResult> {
                    BeliefResult::create(
                        builder,
                        self.present,
                        self.belief_type,
                        &self.belief_id,
                        &self.canonical_identity,
                        &self.conflict_domain,
                        &self.value,
                        self.claim,
                        self.valid_from_ns,
                        self.valid_to_ns,
                        self.transaction_lsn,
                        self.version,
                        self.supersedes_version,
                        &self.provenance,
                        &self.conflict_edges,
                        self.tombstoned,
                    )
                }
            }

            /// Builder for serializing an instance of the [BeliefResult] type.
            ///
            /// Can be created using the [BeliefResult::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct BeliefResultBuilder<State>(State);

            impl BeliefResultBuilder<()> {
                /// Setter for the [`present` field](BeliefResult#structfield.present).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn present<T0>(self, value: T0) -> BeliefResultBuilder<(T0,)>
                where
                    T0: ::planus::WriteAsDefault<bool, bool>,
                {
                    BeliefResultBuilder((value,))
                }

                /// Sets the [`present` field](BeliefResult#structfield.present) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn present_as_default(self) -> BeliefResultBuilder<(::planus::DefaultValue,)> {
                    self.present(::planus::DefaultValue)
                }
            }

            impl<T0> BeliefResultBuilder<(T0,)> {
                /// Setter for the [`belief_type` field](BeliefResult#structfield.belief_type).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn belief_type<T1>(self, value: T1) -> BeliefResultBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAsDefault<u8, u8>,
                {
                    let (v0,) = self.0;
                    BeliefResultBuilder((v0, value))
                }

                /// Sets the [`belief_type` field](BeliefResult#structfield.belief_type) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn belief_type_as_default(
                    self,
                ) -> BeliefResultBuilder<(T0, ::planus::DefaultValue)> {
                    self.belief_type(::planus::DefaultValue)
                }
            }

            impl<T0, T1> BeliefResultBuilder<(T0, T1)> {
                /// Setter for the [`belief_id` field](BeliefResult#structfield.belief_id).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn belief_id<T2>(self, value: T2) -> BeliefResultBuilder<(T0, T1, T2)>
                where
                    T2: ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                {
                    let (v0, v1) = self.0;
                    BeliefResultBuilder((v0, v1, value))
                }

                /// Sets the [`belief_id` field](BeliefResult#structfield.belief_id) to null.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn belief_id_as_null(self) -> BeliefResultBuilder<(T0, T1, ())> {
                    self.belief_id(())
                }
            }

            impl<T0, T1, T2> BeliefResultBuilder<(T0, T1, T2)> {
                /// Setter for the [`canonical_identity` field](BeliefResult#structfield.canonical_identity).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn canonical_identity<T3>(
                    self,
                    value: T3,
                ) -> BeliefResultBuilder<(T0, T1, T2, T3)>
                where
                    T3: ::planus::WriteAsOptional<::planus::Offset<::core::primitive::str>>,
                {
                    let (v0, v1, v2) = self.0;
                    BeliefResultBuilder((v0, v1, v2, value))
                }

                /// Sets the [`canonical_identity` field](BeliefResult#structfield.canonical_identity) to null.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn canonical_identity_as_null(self) -> BeliefResultBuilder<(T0, T1, T2, ())> {
                    self.canonical_identity(())
                }
            }

            impl<T0, T1, T2, T3> BeliefResultBuilder<(T0, T1, T2, T3)> {
                /// Setter for the [`conflict_domain` field](BeliefResult#structfield.conflict_domain).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn conflict_domain<T4>(
                    self,
                    value: T4,
                ) -> BeliefResultBuilder<(T0, T1, T2, T3, T4)>
                where
                    T4: ::planus::WriteAsOptional<::planus::Offset<::core::primitive::str>>,
                {
                    let (v0, v1, v2, v3) = self.0;
                    BeliefResultBuilder((v0, v1, v2, v3, value))
                }

                /// Sets the [`conflict_domain` field](BeliefResult#structfield.conflict_domain) to null.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn conflict_domain_as_null(self) -> BeliefResultBuilder<(T0, T1, T2, T3, ())> {
                    self.conflict_domain(())
                }
            }

            impl<T0, T1, T2, T3, T4> BeliefResultBuilder<(T0, T1, T2, T3, T4)> {
                /// Setter for the [`value` field](BeliefResult#structfield.value).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn value<T5>(self, value: T5) -> BeliefResultBuilder<(T0, T1, T2, T3, T4, T5)>
                where
                    T5: ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                {
                    let (v0, v1, v2, v3, v4) = self.0;
                    BeliefResultBuilder((v0, v1, v2, v3, v4, value))
                }

                /// Sets the [`value` field](BeliefResult#structfield.value) to null.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn value_as_null(self) -> BeliefResultBuilder<(T0, T1, T2, T3, T4, ())> {
                    self.value(())
                }
            }

            impl<T0, T1, T2, T3, T4, T5> BeliefResultBuilder<(T0, T1, T2, T3, T4, T5)> {
                /// Setter for the [`claim` field](BeliefResult#structfield.claim).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn claim<T6>(
                    self,
                    value: T6,
                ) -> BeliefResultBuilder<(T0, T1, T2, T3, T4, T5, T6)>
                where
                    T6: ::planus::WriteAsDefault<u8, u8>,
                {
                    let (v0, v1, v2, v3, v4, v5) = self.0;
                    BeliefResultBuilder((v0, v1, v2, v3, v4, v5, value))
                }

                /// Sets the [`claim` field](BeliefResult#structfield.claim) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn claim_as_default(
                    self,
                ) -> BeliefResultBuilder<(T0, T1, T2, T3, T4, T5, ::planus::DefaultValue)>
                {
                    self.claim(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2, T3, T4, T5, T6> BeliefResultBuilder<(T0, T1, T2, T3, T4, T5, T6)> {
                /// Setter for the [`valid_from_ns` field](BeliefResult#structfield.valid_from_ns).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn valid_from_ns<T7>(
                    self,
                    value: T7,
                ) -> BeliefResultBuilder<(T0, T1, T2, T3, T4, T5, T6, T7)>
                where
                    T7: ::planus::WriteAsDefault<i64, i64>,
                {
                    let (v0, v1, v2, v3, v4, v5, v6) = self.0;
                    BeliefResultBuilder((v0, v1, v2, v3, v4, v5, v6, value))
                }

                /// Sets the [`valid_from_ns` field](BeliefResult#structfield.valid_from_ns) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn valid_from_ns_as_default(
                    self,
                ) -> BeliefResultBuilder<(T0, T1, T2, T3, T4, T5, T6, ::planus::DefaultValue)>
                {
                    self.valid_from_ns(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2, T3, T4, T5, T6, T7> BeliefResultBuilder<(T0, T1, T2, T3, T4, T5, T6, T7)> {
                /// Setter for the [`valid_to_ns` field](BeliefResult#structfield.valid_to_ns).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn valid_to_ns<T8>(
                    self,
                    value: T8,
                ) -> BeliefResultBuilder<(T0, T1, T2, T3, T4, T5, T6, T7, T8)>
                where
                    T8: ::planus::WriteAsDefault<i64, i64>,
                {
                    let (v0, v1, v2, v3, v4, v5, v6, v7) = self.0;
                    BeliefResultBuilder((v0, v1, v2, v3, v4, v5, v6, v7, value))
                }

                /// Sets the [`valid_to_ns` field](BeliefResult#structfield.valid_to_ns) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn valid_to_ns_as_default(
                    self,
                ) -> BeliefResultBuilder<(T0, T1, T2, T3, T4, T5, T6, T7, ::planus::DefaultValue)>
                {
                    self.valid_to_ns(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2, T3, T4, T5, T6, T7, T8> BeliefResultBuilder<(T0, T1, T2, T3, T4, T5, T6, T7, T8)> {
                /// Setter for the [`transaction_lsn` field](BeliefResult#structfield.transaction_lsn).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn transaction_lsn<T9>(
                    self,
                    value: T9,
                ) -> BeliefResultBuilder<(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9)>
                where
                    T9: ::planus::WriteAsDefault<u64, u64>,
                {
                    let (v0, v1, v2, v3, v4, v5, v6, v7, v8) = self.0;
                    BeliefResultBuilder((v0, v1, v2, v3, v4, v5, v6, v7, v8, value))
                }

                /// Sets the [`transaction_lsn` field](BeliefResult#structfield.transaction_lsn) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn transaction_lsn_as_default(
                    self,
                ) -> BeliefResultBuilder<(T0, T1, T2, T3, T4, T5, T6, T7, T8, ::planus::DefaultValue)>
                {
                    self.transaction_lsn(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9>
                BeliefResultBuilder<(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9)>
            {
                /// Setter for the [`version` field](BeliefResult#structfield.version).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn version<T10>(
                    self,
                    value: T10,
                ) -> BeliefResultBuilder<(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10)>
                where
                    T10: ::planus::WriteAsDefault<u64, u64>,
                {
                    let (v0, v1, v2, v3, v4, v5, v6, v7, v8, v9) = self.0;
                    BeliefResultBuilder((v0, v1, v2, v3, v4, v5, v6, v7, v8, v9, value))
                }

                /// Sets the [`version` field](BeliefResult#structfield.version) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn version_as_default(
                    self,
                ) -> BeliefResultBuilder<(
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
                    self.version(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10>
                BeliefResultBuilder<(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10)>
            {
                /// Setter for the [`supersedes_version` field](BeliefResult#structfield.supersedes_version).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn supersedes_version<T11>(
                    self,
                    value: T11,
                ) -> BeliefResultBuilder<(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11)>
                where
                    T11: ::planus::WriteAsDefault<u64, u64>,
                {
                    let (v0, v1, v2, v3, v4, v5, v6, v7, v8, v9, v10) = self.0;
                    BeliefResultBuilder((v0, v1, v2, v3, v4, v5, v6, v7, v8, v9, v10, value))
                }

                /// Sets the [`supersedes_version` field](BeliefResult#structfield.supersedes_version) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn supersedes_version_as_default(
                    self,
                ) -> BeliefResultBuilder<(
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
                    self.supersedes_version(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11>
                BeliefResultBuilder<(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11)>
            {
                /// Setter for the [`provenance` field](BeliefResult#structfield.provenance).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn provenance<T12>(
                    self,
                    value: T12,
                ) -> BeliefResultBuilder<(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12)>
                where
                    T12: ::planus::WriteAsOptional<
                            ::planus::Offset<[::planus::Offset<self::BeliefProvenanceRecord>]>,
                        >,
                {
                    let (v0, v1, v2, v3, v4, v5, v6, v7, v8, v9, v10, v11) = self.0;
                    BeliefResultBuilder((v0, v1, v2, v3, v4, v5, v6, v7, v8, v9, v10, v11, value))
                }

                /// Sets the [`provenance` field](BeliefResult#structfield.provenance) to null.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn provenance_as_null(
                    self,
                ) -> BeliefResultBuilder<(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, ())>
                {
                    self.provenance(())
                }
            }

            impl<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12>
                BeliefResultBuilder<(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12)>
            {
                /// Setter for the [`conflict_edges` field](BeliefResult#structfield.conflict_edges).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn conflict_edges<T13>(
                    self,
                    value: T13,
                ) -> BeliefResultBuilder<(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13)>
                where
                    T13: ::planus::WriteAsOptional<
                            ::planus::Offset<[::planus::Offset<self::BeliefConflictRecord>]>,
                        >,
                {
                    let (v0, v1, v2, v3, v4, v5, v6, v7, v8, v9, v10, v11, v12) = self.0;
                    BeliefResultBuilder((
                        v0, v1, v2, v3, v4, v5, v6, v7, v8, v9, v10, v11, v12, value,
                    ))
                }

                /// Sets the [`conflict_edges` field](BeliefResult#structfield.conflict_edges) to null.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn conflict_edges_as_null(
                    self,
                ) -> BeliefResultBuilder<(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, ())>
                {
                    self.conflict_edges(())
                }
            }

            impl<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13>
                BeliefResultBuilder<(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13)>
            {
                /// Setter for the [`tombstoned` field](BeliefResult#structfield.tombstoned).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn tombstoned<T14>(
                    self,
                    value: T14,
                ) -> BeliefResultBuilder<(
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
                    T12,
                    T13,
                    T14,
                )>
                where
                    T14: ::planus::WriteAsDefault<bool, bool>,
                {
                    let (v0, v1, v2, v3, v4, v5, v6, v7, v8, v9, v10, v11, v12, v13) = self.0;
                    BeliefResultBuilder((
                        v0, v1, v2, v3, v4, v5, v6, v7, v8, v9, v10, v11, v12, v13, value,
                    ))
                }

                /// Sets the [`tombstoned` field](BeliefResult#structfield.tombstoned) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn tombstoned_as_default(
                    self,
                ) -> BeliefResultBuilder<(
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
                    T12,
                    T13,
                    ::planus::DefaultValue,
                )> {
                    self.tombstoned(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14>
                BeliefResultBuilder<(
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
                    T12,
                    T13,
                    T14,
                )>
            {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [BeliefResult].
                #[inline]
                pub fn finish(
                    self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<BeliefResult>
                where
                    Self: ::planus::WriteAsOffset<BeliefResult>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<bool, bool>,
                T1: ::planus::WriteAsDefault<u8, u8>,
                T2: ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                T3: ::planus::WriteAsOptional<::planus::Offset<::core::primitive::str>>,
                T4: ::planus::WriteAsOptional<::planus::Offset<::core::primitive::str>>,
                T5: ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                T6: ::planus::WriteAsDefault<u8, u8>,
                T7: ::planus::WriteAsDefault<i64, i64>,
                T8: ::planus::WriteAsDefault<i64, i64>,
                T9: ::planus::WriteAsDefault<u64, u64>,
                T10: ::planus::WriteAsDefault<u64, u64>,
                T11: ::planus::WriteAsDefault<u64, u64>,
                T12: ::planus::WriteAsOptional<
                        ::planus::Offset<[::planus::Offset<self::BeliefProvenanceRecord>]>,
                    >,
                T13: ::planus::WriteAsOptional<
                        ::planus::Offset<[::planus::Offset<self::BeliefConflictRecord>]>,
                    >,
                T14: ::planus::WriteAsDefault<bool, bool>,
            > ::planus::WriteAs<::planus::Offset<BeliefResult>>
                for BeliefResultBuilder<(
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
                    T12,
                    T13,
                    T14,
                )>
            {
                type Prepared = ::planus::Offset<BeliefResult>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<BeliefResult> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<bool, bool>,
                T1: ::planus::WriteAsDefault<u8, u8>,
                T2: ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                T3: ::planus::WriteAsOptional<::planus::Offset<::core::primitive::str>>,
                T4: ::planus::WriteAsOptional<::planus::Offset<::core::primitive::str>>,
                T5: ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                T6: ::planus::WriteAsDefault<u8, u8>,
                T7: ::planus::WriteAsDefault<i64, i64>,
                T8: ::planus::WriteAsDefault<i64, i64>,
                T9: ::planus::WriteAsDefault<u64, u64>,
                T10: ::planus::WriteAsDefault<u64, u64>,
                T11: ::planus::WriteAsDefault<u64, u64>,
                T12: ::planus::WriteAsOptional<
                        ::planus::Offset<[::planus::Offset<self::BeliefProvenanceRecord>]>,
                    >,
                T13: ::planus::WriteAsOptional<
                        ::planus::Offset<[::planus::Offset<self::BeliefConflictRecord>]>,
                    >,
                T14: ::planus::WriteAsDefault<bool, bool>,
            > ::planus::WriteAsOptional<::planus::Offset<BeliefResult>>
                for BeliefResultBuilder<(
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
                    T12,
                    T13,
                    T14,
                )>
            {
                type Prepared = ::planus::Offset<BeliefResult>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<BeliefResult>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<bool, bool>,
                T1: ::planus::WriteAsDefault<u8, u8>,
                T2: ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                T3: ::planus::WriteAsOptional<::planus::Offset<::core::primitive::str>>,
                T4: ::planus::WriteAsOptional<::planus::Offset<::core::primitive::str>>,
                T5: ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                T6: ::planus::WriteAsDefault<u8, u8>,
                T7: ::planus::WriteAsDefault<i64, i64>,
                T8: ::planus::WriteAsDefault<i64, i64>,
                T9: ::planus::WriteAsDefault<u64, u64>,
                T10: ::planus::WriteAsDefault<u64, u64>,
                T11: ::planus::WriteAsDefault<u64, u64>,
                T12: ::planus::WriteAsOptional<
                        ::planus::Offset<[::planus::Offset<self::BeliefProvenanceRecord>]>,
                    >,
                T13: ::planus::WriteAsOptional<
                        ::planus::Offset<[::planus::Offset<self::BeliefConflictRecord>]>,
                    >,
                T14: ::planus::WriteAsDefault<bool, bool>,
            > ::planus::WriteAsOffset<BeliefResult>
                for BeliefResultBuilder<(
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
                    T12,
                    T13,
                    T14,
                )>
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<BeliefResult> {
                    let (v0, v1, v2, v3, v4, v5, v6, v7, v8, v9, v10, v11, v12, v13, v14) = &self.0;
                    BeliefResult::create(
                        builder, v0, v1, v2, v3, v4, v5, v6, v7, v8, v9, v10, v11, v12, v13, v14,
                    )
                }
            }

            /// Reference to a deserialized [BeliefResult].
            #[derive(Copy, Clone)]
            pub struct BeliefResultRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> BeliefResultRef<'a> {
                /// Getter for the [`present` field](BeliefResult#structfield.present).
                #[inline]
                pub fn present(&self) -> ::planus::Result<bool> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(0, "BeliefResult", "present")?
                            .unwrap_or(false),
                    )
                }

                /// Getter for the [`belief_type` field](BeliefResult#structfield.belief_type).
                #[inline]
                pub fn belief_type(&self) -> ::planus::Result<u8> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(1, "BeliefResult", "belief_type")?
                            .unwrap_or(0),
                    )
                }

                /// Getter for the [`belief_id` field](BeliefResult#structfield.belief_id).
                #[inline]
                pub fn belief_id(&self) -> ::planus::Result<::core::option::Option<&'a [u8]>> {
                    self.0.access(2, "BeliefResult", "belief_id")
                }

                /// Getter for the [`canonical_identity` field](BeliefResult#structfield.canonical_identity).
                #[inline]
                pub fn canonical_identity(
                    &self,
                ) -> ::planus::Result<::core::option::Option<&'a ::core::primitive::str>>
                {
                    self.0.access(3, "BeliefResult", "canonical_identity")
                }

                /// Getter for the [`conflict_domain` field](BeliefResult#structfield.conflict_domain).
                #[inline]
                pub fn conflict_domain(
                    &self,
                ) -> ::planus::Result<::core::option::Option<&'a ::core::primitive::str>>
                {
                    self.0.access(4, "BeliefResult", "conflict_domain")
                }

                /// Getter for the [`value` field](BeliefResult#structfield.value).
                #[inline]
                pub fn value(&self) -> ::planus::Result<::core::option::Option<&'a [u8]>> {
                    self.0.access(5, "BeliefResult", "value")
                }

                /// Getter for the [`claim` field](BeliefResult#structfield.claim).
                #[inline]
                pub fn claim(&self) -> ::planus::Result<u8> {
                    ::core::result::Result::Ok(
                        self.0.access(6, "BeliefResult", "claim")?.unwrap_or(0),
                    )
                }

                /// Getter for the [`valid_from_ns` field](BeliefResult#structfield.valid_from_ns).
                #[inline]
                pub fn valid_from_ns(&self) -> ::planus::Result<i64> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(7, "BeliefResult", "valid_from_ns")?
                            .unwrap_or(0),
                    )
                }

                /// Getter for the [`valid_to_ns` field](BeliefResult#structfield.valid_to_ns).
                #[inline]
                pub fn valid_to_ns(&self) -> ::planus::Result<i64> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(8, "BeliefResult", "valid_to_ns")?
                            .unwrap_or(0),
                    )
                }

                /// Getter for the [`transaction_lsn` field](BeliefResult#structfield.transaction_lsn).
                #[inline]
                pub fn transaction_lsn(&self) -> ::planus::Result<u64> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(9, "BeliefResult", "transaction_lsn")?
                            .unwrap_or(0),
                    )
                }

                /// Getter for the [`version` field](BeliefResult#structfield.version).
                #[inline]
                pub fn version(&self) -> ::planus::Result<u64> {
                    ::core::result::Result::Ok(
                        self.0.access(10, "BeliefResult", "version")?.unwrap_or(0),
                    )
                }

                /// Getter for the [`supersedes_version` field](BeliefResult#structfield.supersedes_version).
                #[inline]
                pub fn supersedes_version(&self) -> ::planus::Result<u64> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(11, "BeliefResult", "supersedes_version")?
                            .unwrap_or(0),
                    )
                }

                /// Getter for the [`provenance` field](BeliefResult#structfield.provenance).
                #[inline]
                pub fn provenance(
                    &self,
                ) -> ::planus::Result<
                    ::core::option::Option<
                        ::planus::Vector<'a, ::planus::Result<self::BeliefProvenanceRecordRef<'a>>>,
                    >,
                > {
                    self.0.access(12, "BeliefResult", "provenance")
                }

                /// Getter for the [`conflict_edges` field](BeliefResult#structfield.conflict_edges).
                #[inline]
                pub fn conflict_edges(
                    &self,
                ) -> ::planus::Result<
                    ::core::option::Option<
                        ::planus::Vector<'a, ::planus::Result<self::BeliefConflictRecordRef<'a>>>,
                    >,
                > {
                    self.0.access(13, "BeliefResult", "conflict_edges")
                }

                /// Getter for the [`tombstoned` field](BeliefResult#structfield.tombstoned).
                #[inline]
                pub fn tombstoned(&self) -> ::planus::Result<bool> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(14, "BeliefResult", "tombstoned")?
                            .unwrap_or(false),
                    )
                }
            }

            impl<'a> ::core::fmt::Debug for BeliefResultRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("BeliefResultRef");
                    f.field("present", &self.present());
                    f.field("belief_type", &self.belief_type());
                    if let ::core::option::Option::Some(field_belief_id) =
                        self.belief_id().transpose()
                    {
                        f.field("belief_id", &field_belief_id);
                    }
                    if let ::core::option::Option::Some(field_canonical_identity) =
                        self.canonical_identity().transpose()
                    {
                        f.field("canonical_identity", &field_canonical_identity);
                    }
                    if let ::core::option::Option::Some(field_conflict_domain) =
                        self.conflict_domain().transpose()
                    {
                        f.field("conflict_domain", &field_conflict_domain);
                    }
                    if let ::core::option::Option::Some(field_value) = self.value().transpose() {
                        f.field("value", &field_value);
                    }
                    f.field("claim", &self.claim());
                    f.field("valid_from_ns", &self.valid_from_ns());
                    f.field("valid_to_ns", &self.valid_to_ns());
                    f.field("transaction_lsn", &self.transaction_lsn());
                    f.field("version", &self.version());
                    f.field("supersedes_version", &self.supersedes_version());
                    if let ::core::option::Option::Some(field_provenance) =
                        self.provenance().transpose()
                    {
                        f.field("provenance", &field_provenance);
                    }
                    if let ::core::option::Option::Some(field_conflict_edges) =
                        self.conflict_edges().transpose()
                    {
                        f.field("conflict_edges", &field_conflict_edges);
                    }
                    f.field("tombstoned", &self.tombstoned());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<BeliefResultRef<'a>> for BeliefResult {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: BeliefResultRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        present: ::core::convert::TryInto::try_into(value.present()?)?,
                        belief_type: ::core::convert::TryInto::try_into(value.belief_type()?)?,
                        belief_id: value.belief_id()?.map(|v| v.to_vec()),
                        canonical_identity: value
                            .canonical_identity()?
                            .map(::core::convert::Into::into),
                        conflict_domain: value.conflict_domain()?.map(::core::convert::Into::into),
                        value: value.value()?.map(|v| v.to_vec()),
                        claim: ::core::convert::TryInto::try_into(value.claim()?)?,
                        valid_from_ns: ::core::convert::TryInto::try_into(value.valid_from_ns()?)?,
                        valid_to_ns: ::core::convert::TryInto::try_into(value.valid_to_ns()?)?,
                        transaction_lsn: ::core::convert::TryInto::try_into(
                            value.transaction_lsn()?,
                        )?,
                        version: ::core::convert::TryInto::try_into(value.version()?)?,
                        supersedes_version: ::core::convert::TryInto::try_into(
                            value.supersedes_version()?,
                        )?,
                        provenance: if let ::core::option::Option::Some(provenance) =
                            value.provenance()?
                        {
                            ::core::option::Option::Some(provenance.to_vec_result()?)
                        } else {
                            ::core::option::Option::None
                        },
                        conflict_edges: if let ::core::option::Option::Some(conflict_edges) =
                            value.conflict_edges()?
                        {
                            ::core::option::Option::Some(conflict_edges.to_vec_result()?)
                        } else {
                            ::core::option::Option::None
                        },
                        tombstoned: ::core::convert::TryInto::try_into(value.tombstoned()?)?,
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for BeliefResultRef<'a> {
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

            impl<'a> ::planus::VectorReadInner<'a> for BeliefResultRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[BeliefResultRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<BeliefResult>> for BeliefResult {
                type Value = ::planus::Offset<BeliefResult>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<BeliefResult>],
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

            impl<'a> ::planus::ReadAsRoot<'a> for BeliefResultRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[BeliefResultRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `CheckpointAck` in the namespace `hypermind.protocol`
            ///
            /// Generated from these locations:
            /// * Table `CheckpointAck` in the file `schemas/protocol.fbs:263`
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
            pub struct CheckpointAck {
                /// The field `lsn` in the table `CheckpointAck`
                pub lsn: u64,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for CheckpointAck {
                fn default() -> Self {
                    Self { lsn: 0 }
                }
            }

            impl CheckpointAck {
                /// Creates a [CheckpointAckBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> CheckpointAckBuilder<()> {
                    CheckpointAckBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_lsn: impl ::planus::WriteAsDefault<u64, u64>,
                ) -> ::planus::Offset<Self> {
                    let prepared_lsn = field_lsn.prepare(builder, &0);

                    let mut table_writer: ::planus::table_writer::TableWriter<6> =
                        ::core::default::Default::default();
                    if prepared_lsn.is_some() {
                        table_writer.write_entry::<u64>(0);
                    }

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            if let ::core::option::Option::Some(prepared_lsn) = prepared_lsn {
                                object_writer.write::<_, _, 8>(&prepared_lsn);
                            }
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<CheckpointAck>> for CheckpointAck {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<CheckpointAck> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<CheckpointAck>> for CheckpointAck {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<CheckpointAck>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<CheckpointAck> for CheckpointAck {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<CheckpointAck> {
                    CheckpointAck::create(builder, self.lsn)
                }
            }

            /// Builder for serializing an instance of the [CheckpointAck] type.
            ///
            /// Can be created using the [CheckpointAck::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct CheckpointAckBuilder<State>(State);

            impl CheckpointAckBuilder<()> {
                /// Setter for the [`lsn` field](CheckpointAck#structfield.lsn).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn lsn<T0>(self, value: T0) -> CheckpointAckBuilder<(T0,)>
                where
                    T0: ::planus::WriteAsDefault<u64, u64>,
                {
                    CheckpointAckBuilder((value,))
                }

                /// Sets the [`lsn` field](CheckpointAck#structfield.lsn) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn lsn_as_default(self) -> CheckpointAckBuilder<(::planus::DefaultValue,)> {
                    self.lsn(::planus::DefaultValue)
                }
            }

            impl<T0> CheckpointAckBuilder<(T0,)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [CheckpointAck].
                #[inline]
                pub fn finish(
                    self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<CheckpointAck>
                where
                    Self: ::planus::WriteAsOffset<CheckpointAck>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<T0: ::planus::WriteAsDefault<u64, u64>>
                ::planus::WriteAs<::planus::Offset<CheckpointAck>> for CheckpointAckBuilder<(T0,)>
            {
                type Prepared = ::planus::Offset<CheckpointAck>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<CheckpointAck> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<T0: ::planus::WriteAsDefault<u64, u64>>
                ::planus::WriteAsOptional<::planus::Offset<CheckpointAck>>
                for CheckpointAckBuilder<(T0,)>
            {
                type Prepared = ::planus::Offset<CheckpointAck>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<CheckpointAck>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<T0: ::planus::WriteAsDefault<u64, u64>> ::planus::WriteAsOffset<CheckpointAck>
                for CheckpointAckBuilder<(T0,)>
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<CheckpointAck> {
                    let (v0,) = &self.0;
                    CheckpointAck::create(builder, v0)
                }
            }

            /// Reference to a deserialized [CheckpointAck].
            #[derive(Copy, Clone)]
            pub struct CheckpointAckRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> CheckpointAckRef<'a> {
                /// Getter for the [`lsn` field](CheckpointAck#structfield.lsn).
                #[inline]
                pub fn lsn(&self) -> ::planus::Result<u64> {
                    ::core::result::Result::Ok(
                        self.0.access(0, "CheckpointAck", "lsn")?.unwrap_or(0),
                    )
                }
            }

            impl<'a> ::core::fmt::Debug for CheckpointAckRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("CheckpointAckRef");
                    f.field("lsn", &self.lsn());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<CheckpointAckRef<'a>> for CheckpointAck {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: CheckpointAckRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        lsn: ::core::convert::TryInto::try_into(value.lsn()?)?,
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for CheckpointAckRef<'a> {
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

            impl<'a> ::planus::VectorReadInner<'a> for CheckpointAckRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[CheckpointAckRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<CheckpointAck>> for CheckpointAck {
                type Value = ::planus::Offset<CheckpointAck>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<CheckpointAck>],
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

            impl<'a> ::planus::ReadAsRoot<'a> for CheckpointAckRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[CheckpointAckRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `CheckpointResult` in the namespace `hypermind.protocol`
            ///
            /// Generated from these locations:
            /// * Table `CheckpointResult` in the file `schemas/protocol.fbs:267`
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
            pub struct CheckpointResult {
                /// The field `present` in the table `CheckpointResult`
                pub present: bool,
                /// The field `lsn` in the table `CheckpointResult`
                pub lsn: u64,
                /// The field `blob` in the table `CheckpointResult`
                pub blob: ::core::option::Option<::planus::alloc::vec::Vec<u8>>,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for CheckpointResult {
                fn default() -> Self {
                    Self {
                        present: false,
                        lsn: 0,
                        blob: ::core::default::Default::default(),
                    }
                }
            }

            impl CheckpointResult {
                /// Creates a [CheckpointResultBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> CheckpointResultBuilder<()> {
                    CheckpointResultBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_present: impl ::planus::WriteAsDefault<bool, bool>,
                    field_lsn: impl ::planus::WriteAsDefault<u64, u64>,
                    field_blob: impl ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                ) -> ::planus::Offset<Self> {
                    let prepared_present = field_present.prepare(builder, &false);
                    let prepared_lsn = field_lsn.prepare(builder, &0);
                    let prepared_blob = field_blob.prepare(builder);

                    let mut table_writer: ::planus::table_writer::TableWriter<10> =
                        ::core::default::Default::default();
                    if prepared_lsn.is_some() {
                        table_writer.write_entry::<u64>(1);
                    }
                    if prepared_blob.is_some() {
                        table_writer.write_entry::<::planus::Offset<[u8]>>(2);
                    }
                    if prepared_present.is_some() {
                        table_writer.write_entry::<bool>(0);
                    }

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            if let ::core::option::Option::Some(prepared_lsn) = prepared_lsn {
                                object_writer.write::<_, _, 8>(&prepared_lsn);
                            }
                            if let ::core::option::Option::Some(prepared_blob) = prepared_blob {
                                object_writer.write::<_, _, 4>(&prepared_blob);
                            }
                            if let ::core::option::Option::Some(prepared_present) = prepared_present
                            {
                                object_writer.write::<_, _, 1>(&prepared_present);
                            }
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<CheckpointResult>> for CheckpointResult {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<CheckpointResult> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<CheckpointResult>> for CheckpointResult {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<CheckpointResult>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<CheckpointResult> for CheckpointResult {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<CheckpointResult> {
                    CheckpointResult::create(builder, self.present, self.lsn, &self.blob)
                }
            }

            /// Builder for serializing an instance of the [CheckpointResult] type.
            ///
            /// Can be created using the [CheckpointResult::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct CheckpointResultBuilder<State>(State);

            impl CheckpointResultBuilder<()> {
                /// Setter for the [`present` field](CheckpointResult#structfield.present).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn present<T0>(self, value: T0) -> CheckpointResultBuilder<(T0,)>
                where
                    T0: ::planus::WriteAsDefault<bool, bool>,
                {
                    CheckpointResultBuilder((value,))
                }

                /// Sets the [`present` field](CheckpointResult#structfield.present) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn present_as_default(
                    self,
                ) -> CheckpointResultBuilder<(::planus::DefaultValue,)> {
                    self.present(::planus::DefaultValue)
                }
            }

            impl<T0> CheckpointResultBuilder<(T0,)> {
                /// Setter for the [`lsn` field](CheckpointResult#structfield.lsn).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn lsn<T1>(self, value: T1) -> CheckpointResultBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAsDefault<u64, u64>,
                {
                    let (v0,) = self.0;
                    CheckpointResultBuilder((v0, value))
                }

                /// Sets the [`lsn` field](CheckpointResult#structfield.lsn) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn lsn_as_default(
                    self,
                ) -> CheckpointResultBuilder<(T0, ::planus::DefaultValue)> {
                    self.lsn(::planus::DefaultValue)
                }
            }

            impl<T0, T1> CheckpointResultBuilder<(T0, T1)> {
                /// Setter for the [`blob` field](CheckpointResult#structfield.blob).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn blob<T2>(self, value: T2) -> CheckpointResultBuilder<(T0, T1, T2)>
                where
                    T2: ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
                {
                    let (v0, v1) = self.0;
                    CheckpointResultBuilder((v0, v1, value))
                }

                /// Sets the [`blob` field](CheckpointResult#structfield.blob) to null.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn blob_as_null(self) -> CheckpointResultBuilder<(T0, T1, ())> {
                    self.blob(())
                }
            }

            impl<T0, T1, T2> CheckpointResultBuilder<(T0, T1, T2)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [CheckpointResult].
                #[inline]
                pub fn finish(
                    self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<CheckpointResult>
                where
                    Self: ::planus::WriteAsOffset<CheckpointResult>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<bool, bool>,
                T1: ::planus::WriteAsDefault<u64, u64>,
                T2: ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
            > ::planus::WriteAs<::planus::Offset<CheckpointResult>>
                for CheckpointResultBuilder<(T0, T1, T2)>
            {
                type Prepared = ::planus::Offset<CheckpointResult>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<CheckpointResult> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<bool, bool>,
                T1: ::planus::WriteAsDefault<u64, u64>,
                T2: ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
            > ::planus::WriteAsOptional<::planus::Offset<CheckpointResult>>
                for CheckpointResultBuilder<(T0, T1, T2)>
            {
                type Prepared = ::planus::Offset<CheckpointResult>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<CheckpointResult>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<bool, bool>,
                T1: ::planus::WriteAsDefault<u64, u64>,
                T2: ::planus::WriteAsOptional<::planus::Offset<[u8]>>,
            > ::planus::WriteAsOffset<CheckpointResult> for CheckpointResultBuilder<(T0, T1, T2)>
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<CheckpointResult> {
                    let (v0, v1, v2) = &self.0;
                    CheckpointResult::create(builder, v0, v1, v2)
                }
            }

            /// Reference to a deserialized [CheckpointResult].
            #[derive(Copy, Clone)]
            pub struct CheckpointResultRef<'a>(
                #[allow(dead_code)] ::planus::table_reader::Table<'a>,
            );

            impl<'a> CheckpointResultRef<'a> {
                /// Getter for the [`present` field](CheckpointResult#structfield.present).
                #[inline]
                pub fn present(&self) -> ::planus::Result<bool> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(0, "CheckpointResult", "present")?
                            .unwrap_or(false),
                    )
                }

                /// Getter for the [`lsn` field](CheckpointResult#structfield.lsn).
                #[inline]
                pub fn lsn(&self) -> ::planus::Result<u64> {
                    ::core::result::Result::Ok(
                        self.0.access(1, "CheckpointResult", "lsn")?.unwrap_or(0),
                    )
                }

                /// Getter for the [`blob` field](CheckpointResult#structfield.blob).
                #[inline]
                pub fn blob(&self) -> ::planus::Result<::core::option::Option<&'a [u8]>> {
                    self.0.access(2, "CheckpointResult", "blob")
                }
            }

            impl<'a> ::core::fmt::Debug for CheckpointResultRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("CheckpointResultRef");
                    f.field("present", &self.present());
                    f.field("lsn", &self.lsn());
                    if let ::core::option::Option::Some(field_blob) = self.blob().transpose() {
                        f.field("blob", &field_blob);
                    }
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<CheckpointResultRef<'a>> for CheckpointResult {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: CheckpointResultRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        present: ::core::convert::TryInto::try_into(value.present()?)?,
                        lsn: ::core::convert::TryInto::try_into(value.lsn()?)?,
                        blob: value.blob()?.map(|v| v.to_vec()),
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for CheckpointResultRef<'a> {
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

            impl<'a> ::planus::VectorReadInner<'a> for CheckpointResultRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[CheckpointResultRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<CheckpointResult>> for CheckpointResult {
                type Value = ::planus::Offset<CheckpointResult>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<CheckpointResult>],
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

            impl<'a> ::planus::ReadAsRoot<'a> for CheckpointResultRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[CheckpointResultRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `AttestAck` in the namespace `hypermind.protocol`
            ///
            /// Generated from these locations:
            /// * Table `AttestAck` in the file `schemas/protocol.fbs:273`
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
            pub struct AttestAck {
                /// The field `first_lsn` in the table `AttestAck`
                pub first_lsn: u64,
                /// The field `last_lsn` in the table `AttestAck`
                pub last_lsn: u64,
                /// The field `count` in the table `AttestAck`
                pub count: u32,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for AttestAck {
                fn default() -> Self {
                    Self {
                        first_lsn: 0,
                        last_lsn: 0,
                        count: 0,
                    }
                }
            }

            impl AttestAck {
                /// Creates a [AttestAckBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> AttestAckBuilder<()> {
                    AttestAckBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_first_lsn: impl ::planus::WriteAsDefault<u64, u64>,
                    field_last_lsn: impl ::planus::WriteAsDefault<u64, u64>,
                    field_count: impl ::planus::WriteAsDefault<u32, u32>,
                ) -> ::planus::Offset<Self> {
                    let prepared_first_lsn = field_first_lsn.prepare(builder, &0);
                    let prepared_last_lsn = field_last_lsn.prepare(builder, &0);
                    let prepared_count = field_count.prepare(builder, &0);

                    let mut table_writer: ::planus::table_writer::TableWriter<10> =
                        ::core::default::Default::default();
                    if prepared_first_lsn.is_some() {
                        table_writer.write_entry::<u64>(0);
                    }
                    if prepared_last_lsn.is_some() {
                        table_writer.write_entry::<u64>(1);
                    }
                    if prepared_count.is_some() {
                        table_writer.write_entry::<u32>(2);
                    }

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            if let ::core::option::Option::Some(prepared_first_lsn) =
                                prepared_first_lsn
                            {
                                object_writer.write::<_, _, 8>(&prepared_first_lsn);
                            }
                            if let ::core::option::Option::Some(prepared_last_lsn) =
                                prepared_last_lsn
                            {
                                object_writer.write::<_, _, 8>(&prepared_last_lsn);
                            }
                            if let ::core::option::Option::Some(prepared_count) = prepared_count {
                                object_writer.write::<_, _, 4>(&prepared_count);
                            }
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<AttestAck>> for AttestAck {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<AttestAck> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<AttestAck>> for AttestAck {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<AttestAck>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<AttestAck> for AttestAck {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<AttestAck> {
                    AttestAck::create(builder, self.first_lsn, self.last_lsn, self.count)
                }
            }

            /// Builder for serializing an instance of the [AttestAck] type.
            ///
            /// Can be created using the [AttestAck::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct AttestAckBuilder<State>(State);

            impl AttestAckBuilder<()> {
                /// Setter for the [`first_lsn` field](AttestAck#structfield.first_lsn).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn first_lsn<T0>(self, value: T0) -> AttestAckBuilder<(T0,)>
                where
                    T0: ::planus::WriteAsDefault<u64, u64>,
                {
                    AttestAckBuilder((value,))
                }

                /// Sets the [`first_lsn` field](AttestAck#structfield.first_lsn) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn first_lsn_as_default(self) -> AttestAckBuilder<(::planus::DefaultValue,)> {
                    self.first_lsn(::planus::DefaultValue)
                }
            }

            impl<T0> AttestAckBuilder<(T0,)> {
                /// Setter for the [`last_lsn` field](AttestAck#structfield.last_lsn).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn last_lsn<T1>(self, value: T1) -> AttestAckBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAsDefault<u64, u64>,
                {
                    let (v0,) = self.0;
                    AttestAckBuilder((v0, value))
                }

                /// Sets the [`last_lsn` field](AttestAck#structfield.last_lsn) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn last_lsn_as_default(self) -> AttestAckBuilder<(T0, ::planus::DefaultValue)> {
                    self.last_lsn(::planus::DefaultValue)
                }
            }

            impl<T0, T1> AttestAckBuilder<(T0, T1)> {
                /// Setter for the [`count` field](AttestAck#structfield.count).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn count<T2>(self, value: T2) -> AttestAckBuilder<(T0, T1, T2)>
                where
                    T2: ::planus::WriteAsDefault<u32, u32>,
                {
                    let (v0, v1) = self.0;
                    AttestAckBuilder((v0, v1, value))
                }

                /// Sets the [`count` field](AttestAck#structfield.count) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn count_as_default(
                    self,
                ) -> AttestAckBuilder<(T0, T1, ::planus::DefaultValue)> {
                    self.count(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2> AttestAckBuilder<(T0, T1, T2)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [AttestAck].
                #[inline]
                pub fn finish(self, builder: &mut ::planus::Builder) -> ::planus::Offset<AttestAck>
                where
                    Self: ::planus::WriteAsOffset<AttestAck>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u64, u64>,
                T1: ::planus::WriteAsDefault<u64, u64>,
                T2: ::planus::WriteAsDefault<u32, u32>,
            > ::planus::WriteAs<::planus::Offset<AttestAck>> for AttestAckBuilder<(T0, T1, T2)>
            {
                type Prepared = ::planus::Offset<AttestAck>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<AttestAck> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u64, u64>,
                T1: ::planus::WriteAsDefault<u64, u64>,
                T2: ::planus::WriteAsDefault<u32, u32>,
            > ::planus::WriteAsOptional<::planus::Offset<AttestAck>>
                for AttestAckBuilder<(T0, T1, T2)>
            {
                type Prepared = ::planus::Offset<AttestAck>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<AttestAck>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u64, u64>,
                T1: ::planus::WriteAsDefault<u64, u64>,
                T2: ::planus::WriteAsDefault<u32, u32>,
            > ::planus::WriteAsOffset<AttestAck> for AttestAckBuilder<(T0, T1, T2)>
            {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<AttestAck> {
                    let (v0, v1, v2) = &self.0;
                    AttestAck::create(builder, v0, v1, v2)
                }
            }

            /// Reference to a deserialized [AttestAck].
            #[derive(Copy, Clone)]
            pub struct AttestAckRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> AttestAckRef<'a> {
                /// Getter for the [`first_lsn` field](AttestAck#structfield.first_lsn).
                #[inline]
                pub fn first_lsn(&self) -> ::planus::Result<u64> {
                    ::core::result::Result::Ok(
                        self.0.access(0, "AttestAck", "first_lsn")?.unwrap_or(0),
                    )
                }

                /// Getter for the [`last_lsn` field](AttestAck#structfield.last_lsn).
                #[inline]
                pub fn last_lsn(&self) -> ::planus::Result<u64> {
                    ::core::result::Result::Ok(
                        self.0.access(1, "AttestAck", "last_lsn")?.unwrap_or(0),
                    )
                }

                /// Getter for the [`count` field](AttestAck#structfield.count).
                #[inline]
                pub fn count(&self) -> ::planus::Result<u32> {
                    ::core::result::Result::Ok(self.0.access(2, "AttestAck", "count")?.unwrap_or(0))
                }
            }

            impl<'a> ::core::fmt::Debug for AttestAckRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("AttestAckRef");
                    f.field("first_lsn", &self.first_lsn());
                    f.field("last_lsn", &self.last_lsn());
                    f.field("count", &self.count());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<AttestAckRef<'a>> for AttestAck {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: AttestAckRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        first_lsn: ::core::convert::TryInto::try_into(value.first_lsn()?)?,
                        last_lsn: ::core::convert::TryInto::try_into(value.last_lsn()?)?,
                        count: ::core::convert::TryInto::try_into(value.count()?)?,
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for AttestAckRef<'a> {
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

            impl<'a> ::planus::VectorReadInner<'a> for AttestAckRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[AttestAckRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<AttestAck>> for AttestAck {
                type Value = ::planus::Offset<AttestAck>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<AttestAck>],
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

            impl<'a> ::planus::ReadAsRoot<'a> for AttestAckRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[AttestAckRef]", "read_as_root", 0)
                    })
                }
            }

            /// The union `ResponsePayload` in the namespace `hypermind.protocol`
            ///
            /// Generated from these locations:
            /// * Union `ResponsePayload` in the file `schemas/protocol.fbs:279`
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
            pub enum ResponsePayload {
                /// The variant of type `ErrorDetail` in the union `ResponsePayload`
                ErrorDetail(::planus::alloc::boxed::Box<self::ErrorDetail>),

                /// The variant of type `AppendAck` in the union `ResponsePayload`
                AppendAck(::planus::alloc::boxed::Box<self::AppendAck>),

                /// The variant of type `BytesResult` in the union `ResponsePayload`
                BytesResult(::planus::alloc::boxed::Box<self::BytesResult>),

                /// The variant of type `SubscriptionAck` in the union `ResponsePayload`
                SubscriptionAck(::planus::alloc::boxed::Box<self::SubscriptionAck>),

                /// The variant of type `HealthResult` in the union `ResponsePayload`
                HealthResult(::planus::alloc::boxed::Box<self::HealthResult>),

                /// The variant of type `StatsResult` in the union `ResponsePayload`
                StatsResult(::planus::alloc::boxed::Box<self::StatsResult>),

                /// The variant of type `LatencyResult` in the union `ResponsePayload`
                LatencyResult(::planus::alloc::boxed::Box<self::LatencyResult>),

                /// The variant of type `VerifyResult` in the union `ResponsePayload`
                VerifyResult(::planus::alloc::boxed::Box<self::VerifyResult>),

                /// The variant of type `RebuildResult` in the union `ResponsePayload`
                RebuildResult(::planus::alloc::boxed::Box<self::RebuildResult>),

                /// The variant of type `DeleteResult` in the union `ResponsePayload`
                DeleteResult(::planus::alloc::boxed::Box<self::DeleteResult>),

                /// The variant of type `TranscriptResult` in the union `ResponsePayload`
                TranscriptResult(::planus::alloc::boxed::Box<self::TranscriptResult>),

                /// The variant of type `RecallResult` in the union `ResponsePayload`
                RecallResult(::planus::alloc::boxed::Box<self::RecallResult>),

                /// The variant of type `BeliefResult` in the union `ResponsePayload`
                BeliefResult(::planus::alloc::boxed::Box<self::BeliefResult>),

                /// The variant of type `CheckpointAck` in the union `ResponsePayload`
                CheckpointAck(::planus::alloc::boxed::Box<self::CheckpointAck>),

                /// The variant of type `CheckpointResult` in the union `ResponsePayload`
                CheckpointResult(::planus::alloc::boxed::Box<self::CheckpointResult>),

                /// The variant of type `AttestAck` in the union `ResponsePayload`
                AttestAck(::planus::alloc::boxed::Box<self::AttestAck>),
            }

            impl ResponsePayload {
                /// Creates a [ResponsePayloadBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> ResponsePayloadBuilder<::planus::Uninitialized> {
                    ResponsePayloadBuilder(::planus::Uninitialized)
                }

                #[inline]
                pub fn create_error_detail(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::ErrorDetail>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(1, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_append_ack(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::AppendAck>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(2, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_bytes_result(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::BytesResult>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(3, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_subscription_ack(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::SubscriptionAck>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(4, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_health_result(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::HealthResult>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(5, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_stats_result(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::StatsResult>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(6, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_latency_result(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::LatencyResult>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(7, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_verify_result(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::VerifyResult>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(8, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_rebuild_result(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::RebuildResult>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(9, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_delete_result(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::DeleteResult>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(10, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_transcript_result(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::TranscriptResult>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(11, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_recall_result(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::RecallResult>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(12, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_belief_result(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::BeliefResult>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(13, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_checkpoint_ack(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::CheckpointAck>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(14, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_checkpoint_result(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::CheckpointResult>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(15, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_attest_ack(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::AttestAck>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(16, value.prepare(builder).downcast())
                }
            }

            impl ::planus::WriteAsUnion<ResponsePayload> for ResponsePayload {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::UnionOffset<Self> {
                    match self {
                        Self::ErrorDetail(value) => Self::create_error_detail(builder, value),
                        Self::AppendAck(value) => Self::create_append_ack(builder, value),
                        Self::BytesResult(value) => Self::create_bytes_result(builder, value),
                        Self::SubscriptionAck(value) => {
                            Self::create_subscription_ack(builder, value)
                        }
                        Self::HealthResult(value) => Self::create_health_result(builder, value),
                        Self::StatsResult(value) => Self::create_stats_result(builder, value),
                        Self::LatencyResult(value) => Self::create_latency_result(builder, value),
                        Self::VerifyResult(value) => Self::create_verify_result(builder, value),
                        Self::RebuildResult(value) => Self::create_rebuild_result(builder, value),
                        Self::DeleteResult(value) => Self::create_delete_result(builder, value),
                        Self::TranscriptResult(value) => {
                            Self::create_transcript_result(builder, value)
                        }
                        Self::RecallResult(value) => Self::create_recall_result(builder, value),
                        Self::BeliefResult(value) => Self::create_belief_result(builder, value),
                        Self::CheckpointAck(value) => Self::create_checkpoint_ack(builder, value),
                        Self::CheckpointResult(value) => {
                            Self::create_checkpoint_result(builder, value)
                        }
                        Self::AttestAck(value) => Self::create_attest_ack(builder, value),
                    }
                }
            }

            impl ::planus::WriteAsOptionalUnion<ResponsePayload> for ResponsePayload {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<Self>> {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }

            /// Builder for serializing an instance of the [ResponsePayload] type.
            ///
            /// Can be created using the [ResponsePayload::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct ResponsePayloadBuilder<T>(T);

            impl ResponsePayloadBuilder<::planus::Uninitialized> {
                /// Creates an instance of the [`ErrorDetail` variant](ResponsePayload#variant.ErrorDetail).
                #[inline]
                pub fn error_detail<T>(
                    self,
                    value: T,
                ) -> ResponsePayloadBuilder<::planus::Initialized<1, T>>
                where
                    T: ::planus::WriteAsOffset<self::ErrorDetail>,
                {
                    ResponsePayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`AppendAck` variant](ResponsePayload#variant.AppendAck).
                #[inline]
                pub fn append_ack<T>(
                    self,
                    value: T,
                ) -> ResponsePayloadBuilder<::planus::Initialized<2, T>>
                where
                    T: ::planus::WriteAsOffset<self::AppendAck>,
                {
                    ResponsePayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`BytesResult` variant](ResponsePayload#variant.BytesResult).
                #[inline]
                pub fn bytes_result<T>(
                    self,
                    value: T,
                ) -> ResponsePayloadBuilder<::planus::Initialized<3, T>>
                where
                    T: ::planus::WriteAsOffset<self::BytesResult>,
                {
                    ResponsePayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`SubscriptionAck` variant](ResponsePayload#variant.SubscriptionAck).
                #[inline]
                pub fn subscription_ack<T>(
                    self,
                    value: T,
                ) -> ResponsePayloadBuilder<::planus::Initialized<4, T>>
                where
                    T: ::planus::WriteAsOffset<self::SubscriptionAck>,
                {
                    ResponsePayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`HealthResult` variant](ResponsePayload#variant.HealthResult).
                #[inline]
                pub fn health_result<T>(
                    self,
                    value: T,
                ) -> ResponsePayloadBuilder<::planus::Initialized<5, T>>
                where
                    T: ::planus::WriteAsOffset<self::HealthResult>,
                {
                    ResponsePayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`StatsResult` variant](ResponsePayload#variant.StatsResult).
                #[inline]
                pub fn stats_result<T>(
                    self,
                    value: T,
                ) -> ResponsePayloadBuilder<::planus::Initialized<6, T>>
                where
                    T: ::planus::WriteAsOffset<self::StatsResult>,
                {
                    ResponsePayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`LatencyResult` variant](ResponsePayload#variant.LatencyResult).
                #[inline]
                pub fn latency_result<T>(
                    self,
                    value: T,
                ) -> ResponsePayloadBuilder<::planus::Initialized<7, T>>
                where
                    T: ::planus::WriteAsOffset<self::LatencyResult>,
                {
                    ResponsePayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`VerifyResult` variant](ResponsePayload#variant.VerifyResult).
                #[inline]
                pub fn verify_result<T>(
                    self,
                    value: T,
                ) -> ResponsePayloadBuilder<::planus::Initialized<8, T>>
                where
                    T: ::planus::WriteAsOffset<self::VerifyResult>,
                {
                    ResponsePayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`RebuildResult` variant](ResponsePayload#variant.RebuildResult).
                #[inline]
                pub fn rebuild_result<T>(
                    self,
                    value: T,
                ) -> ResponsePayloadBuilder<::planus::Initialized<9, T>>
                where
                    T: ::planus::WriteAsOffset<self::RebuildResult>,
                {
                    ResponsePayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`DeleteResult` variant](ResponsePayload#variant.DeleteResult).
                #[inline]
                pub fn delete_result<T>(
                    self,
                    value: T,
                ) -> ResponsePayloadBuilder<::planus::Initialized<10, T>>
                where
                    T: ::planus::WriteAsOffset<self::DeleteResult>,
                {
                    ResponsePayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`TranscriptResult` variant](ResponsePayload#variant.TranscriptResult).
                #[inline]
                pub fn transcript_result<T>(
                    self,
                    value: T,
                ) -> ResponsePayloadBuilder<::planus::Initialized<11, T>>
                where
                    T: ::planus::WriteAsOffset<self::TranscriptResult>,
                {
                    ResponsePayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`RecallResult` variant](ResponsePayload#variant.RecallResult).
                #[inline]
                pub fn recall_result<T>(
                    self,
                    value: T,
                ) -> ResponsePayloadBuilder<::planus::Initialized<12, T>>
                where
                    T: ::planus::WriteAsOffset<self::RecallResult>,
                {
                    ResponsePayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`BeliefResult` variant](ResponsePayload#variant.BeliefResult).
                #[inline]
                pub fn belief_result<T>(
                    self,
                    value: T,
                ) -> ResponsePayloadBuilder<::planus::Initialized<13, T>>
                where
                    T: ::planus::WriteAsOffset<self::BeliefResult>,
                {
                    ResponsePayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`CheckpointAck` variant](ResponsePayload#variant.CheckpointAck).
                #[inline]
                pub fn checkpoint_ack<T>(
                    self,
                    value: T,
                ) -> ResponsePayloadBuilder<::planus::Initialized<14, T>>
                where
                    T: ::planus::WriteAsOffset<self::CheckpointAck>,
                {
                    ResponsePayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`CheckpointResult` variant](ResponsePayload#variant.CheckpointResult).
                #[inline]
                pub fn checkpoint_result<T>(
                    self,
                    value: T,
                ) -> ResponsePayloadBuilder<::planus::Initialized<15, T>>
                where
                    T: ::planus::WriteAsOffset<self::CheckpointResult>,
                {
                    ResponsePayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`AttestAck` variant](ResponsePayload#variant.AttestAck).
                #[inline]
                pub fn attest_ack<T>(
                    self,
                    value: T,
                ) -> ResponsePayloadBuilder<::planus::Initialized<16, T>>
                where
                    T: ::planus::WriteAsOffset<self::AttestAck>,
                {
                    ResponsePayloadBuilder(::planus::Initialized(value))
                }
            }

            impl<const N: u8, T> ResponsePayloadBuilder<::planus::Initialized<N, T>> {
                /// Finish writing the builder to get an [UnionOffset](::planus::UnionOffset) to a serialized [ResponsePayload].
                #[inline]
                pub fn finish(
                    self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<ResponsePayload>
                where
                    Self: ::planus::WriteAsUnion<ResponsePayload>,
                {
                    ::planus::WriteAsUnion::prepare(&self, builder)
                }
            }

            impl<T> ::planus::WriteAsUnion<ResponsePayload>
                for ResponsePayloadBuilder<::planus::Initialized<1, T>>
            where
                T: ::planus::WriteAsOffset<self::ErrorDetail>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<ResponsePayload> {
                    ::planus::UnionOffset::new(1, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<ResponsePayload>
                for ResponsePayloadBuilder<::planus::Initialized<1, T>>
            where
                T: ::planus::WriteAsOffset<self::ErrorDetail>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<ResponsePayload>>
                {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<ResponsePayload>
                for ResponsePayloadBuilder<::planus::Initialized<2, T>>
            where
                T: ::planus::WriteAsOffset<self::AppendAck>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<ResponsePayload> {
                    ::planus::UnionOffset::new(2, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<ResponsePayload>
                for ResponsePayloadBuilder<::planus::Initialized<2, T>>
            where
                T: ::planus::WriteAsOffset<self::AppendAck>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<ResponsePayload>>
                {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<ResponsePayload>
                for ResponsePayloadBuilder<::planus::Initialized<3, T>>
            where
                T: ::planus::WriteAsOffset<self::BytesResult>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<ResponsePayload> {
                    ::planus::UnionOffset::new(3, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<ResponsePayload>
                for ResponsePayloadBuilder<::planus::Initialized<3, T>>
            where
                T: ::planus::WriteAsOffset<self::BytesResult>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<ResponsePayload>>
                {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<ResponsePayload>
                for ResponsePayloadBuilder<::planus::Initialized<4, T>>
            where
                T: ::planus::WriteAsOffset<self::SubscriptionAck>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<ResponsePayload> {
                    ::planus::UnionOffset::new(4, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<ResponsePayload>
                for ResponsePayloadBuilder<::planus::Initialized<4, T>>
            where
                T: ::planus::WriteAsOffset<self::SubscriptionAck>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<ResponsePayload>>
                {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<ResponsePayload>
                for ResponsePayloadBuilder<::planus::Initialized<5, T>>
            where
                T: ::planus::WriteAsOffset<self::HealthResult>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<ResponsePayload> {
                    ::planus::UnionOffset::new(5, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<ResponsePayload>
                for ResponsePayloadBuilder<::planus::Initialized<5, T>>
            where
                T: ::planus::WriteAsOffset<self::HealthResult>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<ResponsePayload>>
                {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<ResponsePayload>
                for ResponsePayloadBuilder<::planus::Initialized<6, T>>
            where
                T: ::planus::WriteAsOffset<self::StatsResult>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<ResponsePayload> {
                    ::planus::UnionOffset::new(6, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<ResponsePayload>
                for ResponsePayloadBuilder<::planus::Initialized<6, T>>
            where
                T: ::planus::WriteAsOffset<self::StatsResult>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<ResponsePayload>>
                {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<ResponsePayload>
                for ResponsePayloadBuilder<::planus::Initialized<7, T>>
            where
                T: ::planus::WriteAsOffset<self::LatencyResult>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<ResponsePayload> {
                    ::planus::UnionOffset::new(7, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<ResponsePayload>
                for ResponsePayloadBuilder<::planus::Initialized<7, T>>
            where
                T: ::planus::WriteAsOffset<self::LatencyResult>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<ResponsePayload>>
                {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<ResponsePayload>
                for ResponsePayloadBuilder<::planus::Initialized<8, T>>
            where
                T: ::planus::WriteAsOffset<self::VerifyResult>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<ResponsePayload> {
                    ::planus::UnionOffset::new(8, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<ResponsePayload>
                for ResponsePayloadBuilder<::planus::Initialized<8, T>>
            where
                T: ::planus::WriteAsOffset<self::VerifyResult>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<ResponsePayload>>
                {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<ResponsePayload>
                for ResponsePayloadBuilder<::planus::Initialized<9, T>>
            where
                T: ::planus::WriteAsOffset<self::RebuildResult>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<ResponsePayload> {
                    ::planus::UnionOffset::new(9, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<ResponsePayload>
                for ResponsePayloadBuilder<::planus::Initialized<9, T>>
            where
                T: ::planus::WriteAsOffset<self::RebuildResult>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<ResponsePayload>>
                {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<ResponsePayload>
                for ResponsePayloadBuilder<::planus::Initialized<10, T>>
            where
                T: ::planus::WriteAsOffset<self::DeleteResult>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<ResponsePayload> {
                    ::planus::UnionOffset::new(10, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<ResponsePayload>
                for ResponsePayloadBuilder<::planus::Initialized<10, T>>
            where
                T: ::planus::WriteAsOffset<self::DeleteResult>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<ResponsePayload>>
                {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<ResponsePayload>
                for ResponsePayloadBuilder<::planus::Initialized<11, T>>
            where
                T: ::planus::WriteAsOffset<self::TranscriptResult>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<ResponsePayload> {
                    ::planus::UnionOffset::new(11, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<ResponsePayload>
                for ResponsePayloadBuilder<::planus::Initialized<11, T>>
            where
                T: ::planus::WriteAsOffset<self::TranscriptResult>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<ResponsePayload>>
                {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<ResponsePayload>
                for ResponsePayloadBuilder<::planus::Initialized<12, T>>
            where
                T: ::planus::WriteAsOffset<self::RecallResult>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<ResponsePayload> {
                    ::planus::UnionOffset::new(12, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<ResponsePayload>
                for ResponsePayloadBuilder<::planus::Initialized<12, T>>
            where
                T: ::planus::WriteAsOffset<self::RecallResult>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<ResponsePayload>>
                {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<ResponsePayload>
                for ResponsePayloadBuilder<::planus::Initialized<13, T>>
            where
                T: ::planus::WriteAsOffset<self::BeliefResult>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<ResponsePayload> {
                    ::planus::UnionOffset::new(13, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<ResponsePayload>
                for ResponsePayloadBuilder<::planus::Initialized<13, T>>
            where
                T: ::planus::WriteAsOffset<self::BeliefResult>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<ResponsePayload>>
                {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<ResponsePayload>
                for ResponsePayloadBuilder<::planus::Initialized<14, T>>
            where
                T: ::planus::WriteAsOffset<self::CheckpointAck>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<ResponsePayload> {
                    ::planus::UnionOffset::new(14, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<ResponsePayload>
                for ResponsePayloadBuilder<::planus::Initialized<14, T>>
            where
                T: ::planus::WriteAsOffset<self::CheckpointAck>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<ResponsePayload>>
                {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<ResponsePayload>
                for ResponsePayloadBuilder<::planus::Initialized<15, T>>
            where
                T: ::planus::WriteAsOffset<self::CheckpointResult>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<ResponsePayload> {
                    ::planus::UnionOffset::new(15, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<ResponsePayload>
                for ResponsePayloadBuilder<::planus::Initialized<15, T>>
            where
                T: ::planus::WriteAsOffset<self::CheckpointResult>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<ResponsePayload>>
                {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<ResponsePayload>
                for ResponsePayloadBuilder<::planus::Initialized<16, T>>
            where
                T: ::planus::WriteAsOffset<self::AttestAck>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<ResponsePayload> {
                    ::planus::UnionOffset::new(16, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<ResponsePayload>
                for ResponsePayloadBuilder<::planus::Initialized<16, T>>
            where
                T: ::planus::WriteAsOffset<self::AttestAck>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<ResponsePayload>>
                {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }

            /// Reference to a deserialized [ResponsePayload].
            #[derive(Copy, Clone, Debug)]
            pub enum ResponsePayloadRef<'a> {
                ErrorDetail(self::ErrorDetailRef<'a>),
                AppendAck(self::AppendAckRef<'a>),
                BytesResult(self::BytesResultRef<'a>),
                SubscriptionAck(self::SubscriptionAckRef<'a>),
                HealthResult(self::HealthResultRef<'a>),
                StatsResult(self::StatsResultRef<'a>),
                LatencyResult(self::LatencyResultRef<'a>),
                VerifyResult(self::VerifyResultRef<'a>),
                RebuildResult(self::RebuildResultRef<'a>),
                DeleteResult(self::DeleteResultRef<'a>),
                TranscriptResult(self::TranscriptResultRef<'a>),
                RecallResult(self::RecallResultRef<'a>),
                BeliefResult(self::BeliefResultRef<'a>),
                CheckpointAck(self::CheckpointAckRef<'a>),
                CheckpointResult(self::CheckpointResultRef<'a>),
                AttestAck(self::AttestAckRef<'a>),
            }

            impl<'a> ::core::convert::TryFrom<ResponsePayloadRef<'a>> for ResponsePayload {
                type Error = ::planus::Error;

                fn try_from(value: ResponsePayloadRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(match value {
                        ResponsePayloadRef::ErrorDetail(value) => {
                            Self::ErrorDetail(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        ResponsePayloadRef::AppendAck(value) => {
                            Self::AppendAck(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        ResponsePayloadRef::BytesResult(value) => {
                            Self::BytesResult(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        ResponsePayloadRef::SubscriptionAck(value) => {
                            Self::SubscriptionAck(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        ResponsePayloadRef::HealthResult(value) => {
                            Self::HealthResult(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        ResponsePayloadRef::StatsResult(value) => {
                            Self::StatsResult(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        ResponsePayloadRef::LatencyResult(value) => {
                            Self::LatencyResult(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        ResponsePayloadRef::VerifyResult(value) => {
                            Self::VerifyResult(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        ResponsePayloadRef::RebuildResult(value) => {
                            Self::RebuildResult(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        ResponsePayloadRef::DeleteResult(value) => {
                            Self::DeleteResult(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        ResponsePayloadRef::TranscriptResult(value) => {
                            Self::TranscriptResult(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        ResponsePayloadRef::RecallResult(value) => {
                            Self::RecallResult(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        ResponsePayloadRef::BeliefResult(value) => {
                            Self::BeliefResult(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        ResponsePayloadRef::CheckpointAck(value) => {
                            Self::CheckpointAck(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        ResponsePayloadRef::CheckpointResult(value) => {
                            Self::CheckpointResult(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        ResponsePayloadRef::AttestAck(value) => {
                            Self::AttestAck(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }
                    })
                }
            }

            impl<'a> ::planus::TableReadUnion<'a> for ResponsePayloadRef<'a> {
                fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    tag: u8,
                    field_offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::ErrorKind> {
                    match tag {
                        1 => ::core::result::Result::Ok(Self::ErrorDetail(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        2 => ::core::result::Result::Ok(Self::AppendAck(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        3 => ::core::result::Result::Ok(Self::BytesResult(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        4 => ::core::result::Result::Ok(Self::SubscriptionAck(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        5 => ::core::result::Result::Ok(Self::HealthResult(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        6 => ::core::result::Result::Ok(Self::StatsResult(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        7 => ::core::result::Result::Ok(Self::LatencyResult(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        8 => ::core::result::Result::Ok(Self::VerifyResult(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        9 => ::core::result::Result::Ok(Self::RebuildResult(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        10 => ::core::result::Result::Ok(Self::DeleteResult(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        11 => ::core::result::Result::Ok(Self::TranscriptResult(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        12 => ::core::result::Result::Ok(Self::RecallResult(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        13 => ::core::result::Result::Ok(Self::BeliefResult(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        14 => ::core::result::Result::Ok(Self::CheckpointAck(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        15 => ::core::result::Result::Ok(Self::CheckpointResult(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        16 => ::core::result::Result::Ok(Self::AttestAck(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        _ => ::core::result::Result::Err(
                            ::planus::errors::ErrorKind::UnknownUnionTag { tag },
                        ),
                    }
                }
            }

            impl<'a> ::planus::VectorReadUnion<'a> for ResponsePayloadRef<'a> {
                const VECTOR_NAME: &'static str = "[ResponsePayloadRef]";
            }

            /// The table `Response` in the namespace `hypermind.protocol`
            ///
            /// Generated from these locations:
            /// * Table `Response` in the file `schemas/protocol.fbs:298`
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
            pub struct Response {
                /// The field `request_id` in the table `Response`
                pub request_id: u64,
                /// The field `status` in the table `Response`
                pub status: self::ResponseStatus,
                /// The field `payload` in the table `Response`
                pub payload: ::core::option::Option<self::ResponsePayload>,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for Response {
                fn default() -> Self {
                    Self {
                        request_id: 0,
                        status: self::ResponseStatus::Ok,
                        payload: ::core::default::Default::default(),
                    }
                }
            }

            impl Response {
                /// Creates a [ResponseBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> ResponseBuilder<()> {
                    ResponseBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_request_id: impl ::planus::WriteAsDefault<u64, u64>,
                    field_status: impl ::planus::WriteAsDefault<
                        self::ResponseStatus,
                        self::ResponseStatus,
                    >,
                    field_payload: impl ::planus::WriteAsOptionalUnion<self::ResponsePayload>,
                ) -> ::planus::Offset<Self> {
                    let prepared_request_id = field_request_id.prepare(builder, &0);
                    let prepared_status = field_status.prepare(builder, &self::ResponseStatus::Ok);
                    let prepared_payload = field_payload.prepare(builder);

                    let mut table_writer: ::planus::table_writer::TableWriter<12> =
                        ::core::default::Default::default();
                    if prepared_request_id.is_some() {
                        table_writer.write_entry::<u64>(0);
                    }
                    if prepared_payload.is_some() {
                        table_writer.write_entry::<::planus::Offset<self::ResponsePayload>>(3);
                    }
                    if prepared_status.is_some() {
                        table_writer.write_entry::<self::ResponseStatus>(1);
                    }
                    if prepared_payload.is_some() {
                        table_writer.write_entry::<u8>(2);
                    }

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            if let ::core::option::Option::Some(prepared_request_id) =
                                prepared_request_id
                            {
                                object_writer.write::<_, _, 8>(&prepared_request_id);
                            }
                            if let ::core::option::Option::Some(prepared_payload) = prepared_payload
                            {
                                object_writer.write::<_, _, 4>(&prepared_payload.offset());
                            }
                            if let ::core::option::Option::Some(prepared_status) = prepared_status {
                                object_writer.write::<_, _, 1>(&prepared_status);
                            }
                            if let ::core::option::Option::Some(prepared_payload) = prepared_payload
                            {
                                object_writer.write::<_, _, 1>(&prepared_payload.tag());
                            }
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<Response>> for Response {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Response> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<Response>> for Response {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Response>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<Response> for Response {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Response> {
                    Response::create(builder, self.request_id, self.status, &self.payload)
                }
            }

            /// Builder for serializing an instance of the [Response] type.
            ///
            /// Can be created using the [Response::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct ResponseBuilder<State>(State);

            impl ResponseBuilder<()> {
                /// Setter for the [`request_id` field](Response#structfield.request_id).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn request_id<T0>(self, value: T0) -> ResponseBuilder<(T0,)>
                where
                    T0: ::planus::WriteAsDefault<u64, u64>,
                {
                    ResponseBuilder((value,))
                }

                /// Sets the [`request_id` field](Response#structfield.request_id) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn request_id_as_default(self) -> ResponseBuilder<(::planus::DefaultValue,)> {
                    self.request_id(::planus::DefaultValue)
                }
            }

            impl<T0> ResponseBuilder<(T0,)> {
                /// Setter for the [`status` field](Response#structfield.status).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn status<T1>(self, value: T1) -> ResponseBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAsDefault<self::ResponseStatus, self::ResponseStatus>,
                {
                    let (v0,) = self.0;
                    ResponseBuilder((v0, value))
                }

                /// Sets the [`status` field](Response#structfield.status) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn status_as_default(self) -> ResponseBuilder<(T0, ::planus::DefaultValue)> {
                    self.status(::planus::DefaultValue)
                }
            }

            impl<T0, T1> ResponseBuilder<(T0, T1)> {
                /// Setter for the [`payload` field](Response#structfield.payload).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn payload<T2>(self, value: T2) -> ResponseBuilder<(T0, T1, T2)>
                where
                    T2: ::planus::WriteAsOptionalUnion<self::ResponsePayload>,
                {
                    let (v0, v1) = self.0;
                    ResponseBuilder((v0, v1, value))
                }

                /// Sets the [`payload` field](Response#structfield.payload) to null.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn payload_as_null(self) -> ResponseBuilder<(T0, T1, ())> {
                    self.payload(())
                }
            }

            impl<T0, T1, T2> ResponseBuilder<(T0, T1, T2)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [Response].
                #[inline]
                pub fn finish(self, builder: &mut ::planus::Builder) -> ::planus::Offset<Response>
                where
                    Self: ::planus::WriteAsOffset<Response>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u64, u64>,
                T1: ::planus::WriteAsDefault<self::ResponseStatus, self::ResponseStatus>,
                T2: ::planus::WriteAsOptionalUnion<self::ResponsePayload>,
            > ::planus::WriteAs<::planus::Offset<Response>> for ResponseBuilder<(T0, T1, T2)>
            {
                type Prepared = ::planus::Offset<Response>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Response> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u64, u64>,
                T1: ::planus::WriteAsDefault<self::ResponseStatus, self::ResponseStatus>,
                T2: ::planus::WriteAsOptionalUnion<self::ResponsePayload>,
            > ::planus::WriteAsOptional<::planus::Offset<Response>>
                for ResponseBuilder<(T0, T1, T2)>
            {
                type Prepared = ::planus::Offset<Response>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Response>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u64, u64>,
                T1: ::planus::WriteAsDefault<self::ResponseStatus, self::ResponseStatus>,
                T2: ::planus::WriteAsOptionalUnion<self::ResponsePayload>,
            > ::planus::WriteAsOffset<Response> for ResponseBuilder<(T0, T1, T2)>
            {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Response> {
                    let (v0, v1, v2) = &self.0;
                    Response::create(builder, v0, v1, v2)
                }
            }

            /// Reference to a deserialized [Response].
            #[derive(Copy, Clone)]
            pub struct ResponseRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> ResponseRef<'a> {
                /// Getter for the [`request_id` field](Response#structfield.request_id).
                #[inline]
                pub fn request_id(&self) -> ::planus::Result<u64> {
                    ::core::result::Result::Ok(
                        self.0.access(0, "Response", "request_id")?.unwrap_or(0),
                    )
                }

                /// Getter for the [`status` field](Response#structfield.status).
                #[inline]
                pub fn status(&self) -> ::planus::Result<self::ResponseStatus> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(1, "Response", "status")?
                            .unwrap_or(self::ResponseStatus::Ok),
                    )
                }

                /// Getter for the [`payload` field](Response#structfield.payload).
                #[inline]
                pub fn payload(
                    &self,
                ) -> ::planus::Result<::core::option::Option<self::ResponsePayloadRef<'a>>>
                {
                    self.0.access_union(2, "Response", "payload")
                }
            }

            impl<'a> ::core::fmt::Debug for ResponseRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("ResponseRef");
                    f.field("request_id", &self.request_id());
                    f.field("status", &self.status());
                    if let ::core::option::Option::Some(field_payload) = self.payload().transpose()
                    {
                        f.field("payload", &field_payload);
                    }
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<ResponseRef<'a>> for Response {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: ResponseRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        request_id: ::core::convert::TryInto::try_into(value.request_id()?)?,
                        status: ::core::convert::TryInto::try_into(value.status()?)?,
                        payload: if let ::core::option::Option::Some(payload) = value.payload()? {
                            ::core::option::Option::Some(::core::convert::TryInto::try_into(
                                payload,
                            )?)
                        } else {
                            ::core::option::Option::None
                        },
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for ResponseRef<'a> {
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

            impl<'a> ::planus::VectorReadInner<'a> for ResponseRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[ResponseRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<Response>> for Response {
                type Value = ::planus::Offset<Response>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<Response>],
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

            impl<'a> ::planus::ReadAsRoot<'a> for ResponseRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[ResponseRef]", "read_as_root", 0)
                    })
                }
            }

            /// The table `Event` in the namespace `hypermind.protocol`
            ///
            /// Generated from these locations:
            /// * Table `Event` in the file `schemas/protocol.fbs:304`
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
            pub struct Event {
                /// The field `subscription_id` in the table `Event`
                pub subscription_id: u64,
                /// The field `lsn` in the table `Event`
                pub lsn: u64,
                /// The field `kind` in the table `Event`
                pub kind: u8,
                /// The field `wall_timestamp_ns` in the table `Event`
                pub wall_timestamp_ns: i64,
                /// The field `actor` in the table `Event`
                pub actor: u16,
                /// The field `conversation` in the table `Event`
                pub conversation: ::planus::alloc::vec::Vec<u8>,
                /// The field `payload` in the table `Event`
                pub payload: ::planus::alloc::vec::Vec<u8>,
            }

            #[allow(clippy::derivable_impls)]
            impl ::core::default::Default for Event {
                fn default() -> Self {
                    Self {
                        subscription_id: 0,
                        lsn: 0,
                        kind: 0,
                        wall_timestamp_ns: 0,
                        actor: 0,
                        conversation: ::core::default::Default::default(),
                        payload: ::core::default::Default::default(),
                    }
                }
            }

            impl Event {
                /// Creates a [EventBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> EventBuilder<()> {
                    EventBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_subscription_id: impl ::planus::WriteAsDefault<u64, u64>,
                    field_lsn: impl ::planus::WriteAsDefault<u64, u64>,
                    field_kind: impl ::planus::WriteAsDefault<u8, u8>,
                    field_wall_timestamp_ns: impl ::planus::WriteAsDefault<i64, i64>,
                    field_actor: impl ::planus::WriteAsDefault<u16, u16>,
                    field_conversation: impl ::planus::WriteAs<::planus::Offset<[u8]>>,
                    field_payload: impl ::planus::WriteAs<::planus::Offset<[u8]>>,
                ) -> ::planus::Offset<Self> {
                    let prepared_subscription_id = field_subscription_id.prepare(builder, &0);
                    let prepared_lsn = field_lsn.prepare(builder, &0);
                    let prepared_kind = field_kind.prepare(builder, &0);
                    let prepared_wall_timestamp_ns = field_wall_timestamp_ns.prepare(builder, &0);
                    let prepared_actor = field_actor.prepare(builder, &0);
                    let prepared_conversation = field_conversation.prepare(builder);
                    let prepared_payload = field_payload.prepare(builder);

                    let mut table_writer: ::planus::table_writer::TableWriter<18> =
                        ::core::default::Default::default();
                    if prepared_subscription_id.is_some() {
                        table_writer.write_entry::<u64>(0);
                    }
                    if prepared_lsn.is_some() {
                        table_writer.write_entry::<u64>(1);
                    }
                    if prepared_wall_timestamp_ns.is_some() {
                        table_writer.write_entry::<i64>(3);
                    }
                    table_writer.write_entry::<::planus::Offset<[u8]>>(5);
                    table_writer.write_entry::<::planus::Offset<[u8]>>(6);
                    if prepared_actor.is_some() {
                        table_writer.write_entry::<u16>(4);
                    }
                    if prepared_kind.is_some() {
                        table_writer.write_entry::<u8>(2);
                    }

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            if let ::core::option::Option::Some(prepared_subscription_id) =
                                prepared_subscription_id
                            {
                                object_writer.write::<_, _, 8>(&prepared_subscription_id);
                            }
                            if let ::core::option::Option::Some(prepared_lsn) = prepared_lsn {
                                object_writer.write::<_, _, 8>(&prepared_lsn);
                            }
                            if let ::core::option::Option::Some(prepared_wall_timestamp_ns) =
                                prepared_wall_timestamp_ns
                            {
                                object_writer.write::<_, _, 8>(&prepared_wall_timestamp_ns);
                            }
                            object_writer.write::<_, _, 4>(&prepared_conversation);
                            object_writer.write::<_, _, 4>(&prepared_payload);
                            if let ::core::option::Option::Some(prepared_actor) = prepared_actor {
                                object_writer.write::<_, _, 2>(&prepared_actor);
                            }
                            if let ::core::option::Option::Some(prepared_kind) = prepared_kind {
                                object_writer.write::<_, _, 1>(&prepared_kind);
                            }
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<Event>> for Event {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Event> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<Event>> for Event {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Event>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<Event> for Event {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Event> {
                    Event::create(
                        builder,
                        self.subscription_id,
                        self.lsn,
                        self.kind,
                        self.wall_timestamp_ns,
                        self.actor,
                        &self.conversation,
                        &self.payload,
                    )
                }
            }

            /// Builder for serializing an instance of the [Event] type.
            ///
            /// Can be created using the [Event::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct EventBuilder<State>(State);

            impl EventBuilder<()> {
                /// Setter for the [`subscription_id` field](Event#structfield.subscription_id).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn subscription_id<T0>(self, value: T0) -> EventBuilder<(T0,)>
                where
                    T0: ::planus::WriteAsDefault<u64, u64>,
                {
                    EventBuilder((value,))
                }

                /// Sets the [`subscription_id` field](Event#structfield.subscription_id) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn subscription_id_as_default(self) -> EventBuilder<(::planus::DefaultValue,)> {
                    self.subscription_id(::planus::DefaultValue)
                }
            }

            impl<T0> EventBuilder<(T0,)> {
                /// Setter for the [`lsn` field](Event#structfield.lsn).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn lsn<T1>(self, value: T1) -> EventBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAsDefault<u64, u64>,
                {
                    let (v0,) = self.0;
                    EventBuilder((v0, value))
                }

                /// Sets the [`lsn` field](Event#structfield.lsn) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn lsn_as_default(self) -> EventBuilder<(T0, ::planus::DefaultValue)> {
                    self.lsn(::planus::DefaultValue)
                }
            }

            impl<T0, T1> EventBuilder<(T0, T1)> {
                /// Setter for the [`kind` field](Event#structfield.kind).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn kind<T2>(self, value: T2) -> EventBuilder<(T0, T1, T2)>
                where
                    T2: ::planus::WriteAsDefault<u8, u8>,
                {
                    let (v0, v1) = self.0;
                    EventBuilder((v0, v1, value))
                }

                /// Sets the [`kind` field](Event#structfield.kind) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn kind_as_default(self) -> EventBuilder<(T0, T1, ::planus::DefaultValue)> {
                    self.kind(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2> EventBuilder<(T0, T1, T2)> {
                /// Setter for the [`wall_timestamp_ns` field](Event#structfield.wall_timestamp_ns).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn wall_timestamp_ns<T3>(self, value: T3) -> EventBuilder<(T0, T1, T2, T3)>
                where
                    T3: ::planus::WriteAsDefault<i64, i64>,
                {
                    let (v0, v1, v2) = self.0;
                    EventBuilder((v0, v1, v2, value))
                }

                /// Sets the [`wall_timestamp_ns` field](Event#structfield.wall_timestamp_ns) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn wall_timestamp_ns_as_default(
                    self,
                ) -> EventBuilder<(T0, T1, T2, ::planus::DefaultValue)> {
                    self.wall_timestamp_ns(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2, T3> EventBuilder<(T0, T1, T2, T3)> {
                /// Setter for the [`actor` field](Event#structfield.actor).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn actor<T4>(self, value: T4) -> EventBuilder<(T0, T1, T2, T3, T4)>
                where
                    T4: ::planus::WriteAsDefault<u16, u16>,
                {
                    let (v0, v1, v2, v3) = self.0;
                    EventBuilder((v0, v1, v2, v3, value))
                }

                /// Sets the [`actor` field](Event#structfield.actor) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn actor_as_default(
                    self,
                ) -> EventBuilder<(T0, T1, T2, T3, ::planus::DefaultValue)> {
                    self.actor(::planus::DefaultValue)
                }
            }

            impl<T0, T1, T2, T3, T4> EventBuilder<(T0, T1, T2, T3, T4)> {
                /// Setter for the [`conversation` field](Event#structfield.conversation).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn conversation<T5>(self, value: T5) -> EventBuilder<(T0, T1, T2, T3, T4, T5)>
                where
                    T5: ::planus::WriteAs<::planus::Offset<[u8]>>,
                {
                    let (v0, v1, v2, v3, v4) = self.0;
                    EventBuilder((v0, v1, v2, v3, v4, value))
                }
            }

            impl<T0, T1, T2, T3, T4, T5> EventBuilder<(T0, T1, T2, T3, T4, T5)> {
                /// Setter for the [`payload` field](Event#structfield.payload).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn payload<T6>(self, value: T6) -> EventBuilder<(T0, T1, T2, T3, T4, T5, T6)>
                where
                    T6: ::planus::WriteAs<::planus::Offset<[u8]>>,
                {
                    let (v0, v1, v2, v3, v4, v5) = self.0;
                    EventBuilder((v0, v1, v2, v3, v4, v5, value))
                }
            }

            impl<T0, T1, T2, T3, T4, T5, T6> EventBuilder<(T0, T1, T2, T3, T4, T5, T6)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [Event].
                #[inline]
                pub fn finish(self, builder: &mut ::planus::Builder) -> ::planus::Offset<Event>
                where
                    Self: ::planus::WriteAsOffset<Event>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u64, u64>,
                T1: ::planus::WriteAsDefault<u64, u64>,
                T2: ::planus::WriteAsDefault<u8, u8>,
                T3: ::planus::WriteAsDefault<i64, i64>,
                T4: ::planus::WriteAsDefault<u16, u16>,
                T5: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T6: ::planus::WriteAs<::planus::Offset<[u8]>>,
            > ::planus::WriteAs<::planus::Offset<Event>>
                for EventBuilder<(T0, T1, T2, T3, T4, T5, T6)>
            {
                type Prepared = ::planus::Offset<Event>;

                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Event> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u64, u64>,
                T1: ::planus::WriteAsDefault<u64, u64>,
                T2: ::planus::WriteAsDefault<u8, u8>,
                T3: ::planus::WriteAsDefault<i64, i64>,
                T4: ::planus::WriteAsDefault<u16, u16>,
                T5: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T6: ::planus::WriteAs<::planus::Offset<[u8]>>,
            > ::planus::WriteAsOptional<::planus::Offset<Event>>
                for EventBuilder<(T0, T1, T2, T3, T4, T5, T6)>
            {
                type Prepared = ::planus::Offset<Event>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<Event>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u64, u64>,
                T1: ::planus::WriteAsDefault<u64, u64>,
                T2: ::planus::WriteAsDefault<u8, u8>,
                T3: ::planus::WriteAsDefault<i64, i64>,
                T4: ::planus::WriteAsDefault<u16, u16>,
                T5: ::planus::WriteAs<::planus::Offset<[u8]>>,
                T6: ::planus::WriteAs<::planus::Offset<[u8]>>,
            > ::planus::WriteAsOffset<Event> for EventBuilder<(T0, T1, T2, T3, T4, T5, T6)>
            {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::Offset<Event> {
                    let (v0, v1, v2, v3, v4, v5, v6) = &self.0;
                    Event::create(builder, v0, v1, v2, v3, v4, v5, v6)
                }
            }

            /// Reference to a deserialized [Event].
            #[derive(Copy, Clone)]
            pub struct EventRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> EventRef<'a> {
                /// Getter for the [`subscription_id` field](Event#structfield.subscription_id).
                #[inline]
                pub fn subscription_id(&self) -> ::planus::Result<u64> {
                    ::core::result::Result::Ok(
                        self.0.access(0, "Event", "subscription_id")?.unwrap_or(0),
                    )
                }

                /// Getter for the [`lsn` field](Event#structfield.lsn).
                #[inline]
                pub fn lsn(&self) -> ::planus::Result<u64> {
                    ::core::result::Result::Ok(self.0.access(1, "Event", "lsn")?.unwrap_or(0))
                }

                /// Getter for the [`kind` field](Event#structfield.kind).
                #[inline]
                pub fn kind(&self) -> ::planus::Result<u8> {
                    ::core::result::Result::Ok(self.0.access(2, "Event", "kind")?.unwrap_or(0))
                }

                /// Getter for the [`wall_timestamp_ns` field](Event#structfield.wall_timestamp_ns).
                #[inline]
                pub fn wall_timestamp_ns(&self) -> ::planus::Result<i64> {
                    ::core::result::Result::Ok(
                        self.0.access(3, "Event", "wall_timestamp_ns")?.unwrap_or(0),
                    )
                }

                /// Getter for the [`actor` field](Event#structfield.actor).
                #[inline]
                pub fn actor(&self) -> ::planus::Result<u16> {
                    ::core::result::Result::Ok(self.0.access(4, "Event", "actor")?.unwrap_or(0))
                }

                /// Getter for the [`conversation` field](Event#structfield.conversation).
                #[inline]
                pub fn conversation(&self) -> ::planus::Result<&'a [u8]> {
                    self.0.access_required(5, "Event", "conversation")
                }

                /// Getter for the [`payload` field](Event#structfield.payload).
                #[inline]
                pub fn payload(&self) -> ::planus::Result<&'a [u8]> {
                    self.0.access_required(6, "Event", "payload")
                }
            }

            impl<'a> ::core::fmt::Debug for EventRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("EventRef");
                    f.field("subscription_id", &self.subscription_id());
                    f.field("lsn", &self.lsn());
                    f.field("kind", &self.kind());
                    f.field("wall_timestamp_ns", &self.wall_timestamp_ns());
                    f.field("actor", &self.actor());
                    f.field("conversation", &self.conversation());
                    f.field("payload", &self.payload());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<EventRef<'a>> for Event {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: EventRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        subscription_id: ::core::convert::TryInto::try_into(
                            value.subscription_id()?,
                        )?,
                        lsn: ::core::convert::TryInto::try_into(value.lsn()?)?,
                        kind: ::core::convert::TryInto::try_into(value.kind()?)?,
                        wall_timestamp_ns: ::core::convert::TryInto::try_into(
                            value.wall_timestamp_ns()?,
                        )?,
                        actor: ::core::convert::TryInto::try_into(value.actor()?)?,
                        conversation: value.conversation()?.to_vec(),
                        payload: value.payload()?.to_vec(),
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for EventRef<'a> {
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

            impl<'a> ::planus::VectorReadInner<'a> for EventRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[EventRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<Event>> for Event {
                type Value = ::planus::Offset<Event>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<Event>],
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

            impl<'a> ::planus::ReadAsRoot<'a> for EventRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[EventRef]", "read_as_root", 0)
                    })
                }
            }

            /// The union `WirePayload` in the namespace `hypermind.protocol`
            ///
            /// Generated from these locations:
            /// * Union `WirePayload` in the file `schemas/protocol.fbs:314`
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
            pub enum WirePayload {
                /// The variant of type `Hello` in the union `WirePayload`
                Hello(::planus::alloc::boxed::Box<self::Hello>),

                /// The variant of type `Welcome` in the union `WirePayload`
                Welcome(::planus::alloc::boxed::Box<self::Welcome>),

                /// The variant of type `Request` in the union `WirePayload`
                Request(::planus::alloc::boxed::Box<self::Request>),

                /// The variant of type `Response` in the union `WirePayload`
                Response(::planus::alloc::boxed::Box<self::Response>),

                /// The variant of type `Event` in the union `WirePayload`
                Event(::planus::alloc::boxed::Box<self::Event>),
            }

            impl WirePayload {
                /// Creates a [WirePayloadBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> WirePayloadBuilder<::planus::Uninitialized> {
                    WirePayloadBuilder(::planus::Uninitialized)
                }

                #[inline]
                pub fn create_hello(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::Hello>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(1, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_welcome(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::Welcome>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(2, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_request(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::Request>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(3, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_response(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::Response>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(4, value.prepare(builder).downcast())
                }

                #[inline]
                pub fn create_event(
                    builder: &mut ::planus::Builder,
                    value: impl ::planus::WriteAsOffset<self::Event>,
                ) -> ::planus::UnionOffset<Self> {
                    ::planus::UnionOffset::new(5, value.prepare(builder).downcast())
                }
            }

            impl ::planus::WriteAsUnion<WirePayload> for WirePayload {
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> ::planus::UnionOffset<Self> {
                    match self {
                        Self::Hello(value) => Self::create_hello(builder, value),
                        Self::Welcome(value) => Self::create_welcome(builder, value),
                        Self::Request(value) => Self::create_request(builder, value),
                        Self::Response(value) => Self::create_response(builder, value),
                        Self::Event(value) => Self::create_event(builder, value),
                    }
                }
            }

            impl ::planus::WriteAsOptionalUnion<WirePayload> for WirePayload {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<Self>> {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }

            /// Builder for serializing an instance of the [WirePayload] type.
            ///
            /// Can be created using the [WirePayload::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct WirePayloadBuilder<T>(T);

            impl WirePayloadBuilder<::planus::Uninitialized> {
                /// Creates an instance of the [`Hello` variant](WirePayload#variant.Hello).
                #[inline]
                pub fn hello<T>(self, value: T) -> WirePayloadBuilder<::planus::Initialized<1, T>>
                where
                    T: ::planus::WriteAsOffset<self::Hello>,
                {
                    WirePayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`Welcome` variant](WirePayload#variant.Welcome).
                #[inline]
                pub fn welcome<T>(self, value: T) -> WirePayloadBuilder<::planus::Initialized<2, T>>
                where
                    T: ::planus::WriteAsOffset<self::Welcome>,
                {
                    WirePayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`Request` variant](WirePayload#variant.Request).
                #[inline]
                pub fn request<T>(self, value: T) -> WirePayloadBuilder<::planus::Initialized<3, T>>
                where
                    T: ::planus::WriteAsOffset<self::Request>,
                {
                    WirePayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`Response` variant](WirePayload#variant.Response).
                #[inline]
                pub fn response<T>(
                    self,
                    value: T,
                ) -> WirePayloadBuilder<::planus::Initialized<4, T>>
                where
                    T: ::planus::WriteAsOffset<self::Response>,
                {
                    WirePayloadBuilder(::planus::Initialized(value))
                }

                /// Creates an instance of the [`Event` variant](WirePayload#variant.Event).
                #[inline]
                pub fn event<T>(self, value: T) -> WirePayloadBuilder<::planus::Initialized<5, T>>
                where
                    T: ::planus::WriteAsOffset<self::Event>,
                {
                    WirePayloadBuilder(::planus::Initialized(value))
                }
            }

            impl<const N: u8, T> WirePayloadBuilder<::planus::Initialized<N, T>> {
                /// Finish writing the builder to get an [UnionOffset](::planus::UnionOffset) to a serialized [WirePayload].
                #[inline]
                pub fn finish(
                    self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<WirePayload>
                where
                    Self: ::planus::WriteAsUnion<WirePayload>,
                {
                    ::planus::WriteAsUnion::prepare(&self, builder)
                }
            }

            impl<T> ::planus::WriteAsUnion<WirePayload> for WirePayloadBuilder<::planus::Initialized<1, T>>
            where
                T: ::planus::WriteAsOffset<self::Hello>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<WirePayload> {
                    ::planus::UnionOffset::new(1, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<WirePayload>
                for WirePayloadBuilder<::planus::Initialized<1, T>>
            where
                T: ::planus::WriteAsOffset<self::Hello>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<WirePayload>> {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<WirePayload> for WirePayloadBuilder<::planus::Initialized<2, T>>
            where
                T: ::planus::WriteAsOffset<self::Welcome>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<WirePayload> {
                    ::planus::UnionOffset::new(2, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<WirePayload>
                for WirePayloadBuilder<::planus::Initialized<2, T>>
            where
                T: ::planus::WriteAsOffset<self::Welcome>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<WirePayload>> {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<WirePayload> for WirePayloadBuilder<::planus::Initialized<3, T>>
            where
                T: ::planus::WriteAsOffset<self::Request>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<WirePayload> {
                    ::planus::UnionOffset::new(3, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<WirePayload>
                for WirePayloadBuilder<::planus::Initialized<3, T>>
            where
                T: ::planus::WriteAsOffset<self::Request>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<WirePayload>> {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<WirePayload> for WirePayloadBuilder<::planus::Initialized<4, T>>
            where
                T: ::planus::WriteAsOffset<self::Response>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<WirePayload> {
                    ::planus::UnionOffset::new(4, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<WirePayload>
                for WirePayloadBuilder<::planus::Initialized<4, T>>
            where
                T: ::planus::WriteAsOffset<self::Response>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<WirePayload>> {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }
            impl<T> ::planus::WriteAsUnion<WirePayload> for WirePayloadBuilder<::planus::Initialized<5, T>>
            where
                T: ::planus::WriteAsOffset<self::Event>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::UnionOffset<WirePayload> {
                    ::planus::UnionOffset::new(5, (self.0).0.prepare(builder).downcast())
                }
            }

            impl<T> ::planus::WriteAsOptionalUnion<WirePayload>
                for WirePayloadBuilder<::planus::Initialized<5, T>>
            where
                T: ::planus::WriteAsOffset<self::Event>,
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::UnionOffset<WirePayload>> {
                    ::core::option::Option::Some(::planus::WriteAsUnion::prepare(self, builder))
                }
            }

            /// Reference to a deserialized [WirePayload].
            #[derive(Copy, Clone, Debug)]
            pub enum WirePayloadRef<'a> {
                Hello(self::HelloRef<'a>),
                Welcome(self::WelcomeRef<'a>),
                Request(self::RequestRef<'a>),
                Response(self::ResponseRef<'a>),
                Event(self::EventRef<'a>),
            }

            impl<'a> ::core::convert::TryFrom<WirePayloadRef<'a>> for WirePayload {
                type Error = ::planus::Error;

                fn try_from(value: WirePayloadRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(match value {
                        WirePayloadRef::Hello(value) => {
                            Self::Hello(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        WirePayloadRef::Welcome(value) => {
                            Self::Welcome(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        WirePayloadRef::Request(value) => {
                            Self::Request(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        WirePayloadRef::Response(value) => {
                            Self::Response(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }

                        WirePayloadRef::Event(value) => {
                            Self::Event(::planus::alloc::boxed::Box::new(
                                ::core::convert::TryFrom::try_from(value)?,
                            ))
                        }
                    })
                }
            }

            impl<'a> ::planus::TableReadUnion<'a> for WirePayloadRef<'a> {
                fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    tag: u8,
                    field_offset: usize,
                ) -> ::core::result::Result<Self, ::planus::errors::ErrorKind> {
                    match tag {
                        1 => ::core::result::Result::Ok(Self::Hello(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        2 => ::core::result::Result::Ok(Self::Welcome(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        3 => ::core::result::Result::Ok(Self::Request(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        4 => ::core::result::Result::Ok(Self::Response(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        5 => ::core::result::Result::Ok(Self::Event(
                            ::planus::TableRead::from_buffer(buffer, field_offset)?,
                        )),
                        _ => ::core::result::Result::Err(
                            ::planus::errors::ErrorKind::UnknownUnionTag { tag },
                        ),
                    }
                }
            }

            impl<'a> ::planus::VectorReadUnion<'a> for WirePayloadRef<'a> {
                const VECTOR_NAME: &'static str = "[WirePayloadRef]";
            }

            /// The table `WireEnvelope` in the namespace `hypermind.protocol`
            ///
            /// Generated from these locations:
            /// * Table `WireEnvelope` in the file `schemas/protocol.fbs:322`
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
            pub struct WireEnvelope {
                /// The field `proto_version` in the table `WireEnvelope`
                pub proto_version: u16,
                /// The field `payload` in the table `WireEnvelope`
                pub payload: self::WirePayload,
            }

            impl WireEnvelope {
                /// Creates a [WireEnvelopeBuilder] for serializing an instance of this table.
                #[inline]
                pub fn builder() -> WireEnvelopeBuilder<()> {
                    WireEnvelopeBuilder(())
                }

                #[allow(clippy::too_many_arguments)]
                pub fn create(
                    builder: &mut ::planus::Builder,
                    field_proto_version: impl ::planus::WriteAsDefault<u16, u16>,
                    field_payload: impl ::planus::WriteAsUnion<self::WirePayload>,
                ) -> ::planus::Offset<Self> {
                    let prepared_proto_version = field_proto_version.prepare(builder, &0);
                    let prepared_payload = field_payload.prepare(builder);

                    let mut table_writer: ::planus::table_writer::TableWriter<10> =
                        ::core::default::Default::default();
                    table_writer.write_entry::<::planus::Offset<self::WirePayload>>(2);
                    if prepared_proto_version.is_some() {
                        table_writer.write_entry::<u16>(0);
                    }
                    table_writer.write_entry::<u8>(1);

                    unsafe {
                        table_writer.finish(builder, |object_writer| {
                            object_writer.write::<_, _, 4>(&prepared_payload.offset());
                            if let ::core::option::Option::Some(prepared_proto_version) =
                                prepared_proto_version
                            {
                                object_writer.write::<_, _, 2>(&prepared_proto_version);
                            }
                            object_writer.write::<_, _, 1>(&prepared_payload.tag());
                        });
                    }
                    builder.current_offset()
                }
            }

            impl ::planus::WriteAs<::planus::Offset<WireEnvelope>> for WireEnvelope {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<WireEnvelope> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl ::planus::WriteAsOptional<::planus::Offset<WireEnvelope>> for WireEnvelope {
                type Prepared = ::planus::Offset<Self>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<WireEnvelope>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl ::planus::WriteAsOffset<WireEnvelope> for WireEnvelope {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<WireEnvelope> {
                    WireEnvelope::create(builder, self.proto_version, &self.payload)
                }
            }

            /// Builder for serializing an instance of the [WireEnvelope] type.
            ///
            /// Can be created using the [WireEnvelope::builder] method.
            #[derive(Debug)]
            #[must_use]
            pub struct WireEnvelopeBuilder<State>(State);

            impl WireEnvelopeBuilder<()> {
                /// Setter for the [`proto_version` field](WireEnvelope#structfield.proto_version).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn proto_version<T0>(self, value: T0) -> WireEnvelopeBuilder<(T0,)>
                where
                    T0: ::planus::WriteAsDefault<u16, u16>,
                {
                    WireEnvelopeBuilder((value,))
                }

                /// Sets the [`proto_version` field](WireEnvelope#structfield.proto_version) to the default value.
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn proto_version_as_default(
                    self,
                ) -> WireEnvelopeBuilder<(::planus::DefaultValue,)> {
                    self.proto_version(::planus::DefaultValue)
                }
            }

            impl<T0> WireEnvelopeBuilder<(T0,)> {
                /// Setter for the [`payload` field](WireEnvelope#structfield.payload).
                #[inline]
                #[allow(clippy::type_complexity)]
                pub fn payload<T1>(self, value: T1) -> WireEnvelopeBuilder<(T0, T1)>
                where
                    T1: ::planus::WriteAsUnion<self::WirePayload>,
                {
                    let (v0,) = self.0;
                    WireEnvelopeBuilder((v0, value))
                }
            }

            impl<T0, T1> WireEnvelopeBuilder<(T0, T1)> {
                /// Finish writing the builder to get an [Offset](::planus::Offset) to a serialized [WireEnvelope].
                #[inline]
                pub fn finish(
                    self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<WireEnvelope>
                where
                    Self: ::planus::WriteAsOffset<WireEnvelope>,
                {
                    ::planus::WriteAsOffset::prepare(&self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u16, u16>,
                T1: ::planus::WriteAsUnion<self::WirePayload>,
            > ::planus::WriteAs<::planus::Offset<WireEnvelope>> for WireEnvelopeBuilder<(T0, T1)>
            {
                type Prepared = ::planus::Offset<WireEnvelope>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<WireEnvelope> {
                    ::planus::WriteAsOffset::prepare(self, builder)
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u16, u16>,
                T1: ::planus::WriteAsUnion<self::WirePayload>,
            > ::planus::WriteAsOptional<::planus::Offset<WireEnvelope>>
                for WireEnvelopeBuilder<(T0, T1)>
            {
                type Prepared = ::planus::Offset<WireEnvelope>;

                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::core::option::Option<::planus::Offset<WireEnvelope>> {
                    ::core::option::Option::Some(::planus::WriteAsOffset::prepare(self, builder))
                }
            }

            impl<
                T0: ::planus::WriteAsDefault<u16, u16>,
                T1: ::planus::WriteAsUnion<self::WirePayload>,
            > ::planus::WriteAsOffset<WireEnvelope> for WireEnvelopeBuilder<(T0, T1)>
            {
                #[inline]
                fn prepare(
                    &self,
                    builder: &mut ::planus::Builder,
                ) -> ::planus::Offset<WireEnvelope> {
                    let (v0, v1) = &self.0;
                    WireEnvelope::create(builder, v0, v1)
                }
            }

            /// Reference to a deserialized [WireEnvelope].
            #[derive(Copy, Clone)]
            pub struct WireEnvelopeRef<'a>(#[allow(dead_code)] ::planus::table_reader::Table<'a>);

            impl<'a> WireEnvelopeRef<'a> {
                /// Getter for the [`proto_version` field](WireEnvelope#structfield.proto_version).
                #[inline]
                pub fn proto_version(&self) -> ::planus::Result<u16> {
                    ::core::result::Result::Ok(
                        self.0
                            .access(0, "WireEnvelope", "proto_version")?
                            .unwrap_or(0),
                    )
                }

                /// Getter for the [`payload` field](WireEnvelope#structfield.payload).
                #[inline]
                pub fn payload(&self) -> ::planus::Result<self::WirePayloadRef<'a>> {
                    self.0.access_union_required(1, "WireEnvelope", "payload")
                }
            }

            impl<'a> ::core::fmt::Debug for WireEnvelopeRef<'a> {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    let mut f = f.debug_struct("WireEnvelopeRef");
                    f.field("proto_version", &self.proto_version());
                    f.field("payload", &self.payload());
                    f.finish()
                }
            }

            impl<'a> ::core::convert::TryFrom<WireEnvelopeRef<'a>> for WireEnvelope {
                type Error = ::planus::Error;

                #[allow(unreachable_code)]
                fn try_from(value: WireEnvelopeRef<'a>) -> ::planus::Result<Self> {
                    ::core::result::Result::Ok(Self {
                        proto_version: ::core::convert::TryInto::try_into(value.proto_version()?)?,
                        payload: ::core::convert::TryInto::try_into(value.payload()?)?,
                    })
                }
            }

            impl<'a> ::planus::TableRead<'a> for WireEnvelopeRef<'a> {
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

            impl<'a> ::planus::VectorReadInner<'a> for WireEnvelopeRef<'a> {
                type Error = ::planus::Error;
                const STRIDE: usize = 4;

                unsafe fn from_buffer(
                    buffer: ::planus::SliceWithStartOffset<'a>,
                    offset: usize,
                ) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(buffer, offset).map_err(|error_kind| {
                        error_kind.with_error_location(
                            "[WireEnvelopeRef]",
                            "get",
                            buffer.offset_from_start,
                        )
                    })
                }
            }

            /// # Safety
            /// The planus compiler generates implementations that initialize
            /// the bytes in `write_values`.
            unsafe impl ::planus::VectorWrite<::planus::Offset<WireEnvelope>> for WireEnvelope {
                type Value = ::planus::Offset<WireEnvelope>;
                const STRIDE: usize = 4;
                #[inline]
                fn prepare(&self, builder: &mut ::planus::Builder) -> Self::Value {
                    ::planus::WriteAs::prepare(self, builder)
                }

                #[inline]
                unsafe fn write_values(
                    values: &[::planus::Offset<WireEnvelope>],
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

            impl<'a> ::planus::ReadAsRoot<'a> for WireEnvelopeRef<'a> {
                fn read_as_root(slice: &'a [u8]) -> ::planus::Result<Self> {
                    ::planus::TableRead::from_buffer(
                        ::planus::SliceWithStartOffset {
                            buffer: slice,
                            offset_from_start: 0,
                        },
                        0,
                    )
                    .map_err(|error_kind| {
                        error_kind.with_error_location("[WireEnvelopeRef]", "read_as_root", 0)
                    })
                }
            }
        }
    }
}
