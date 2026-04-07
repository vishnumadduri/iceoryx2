// Copyright (c) 2026 Contributors to the Eclipse Foundation
//
// See the NOTICE file(s) distributed with this work for additional
// information regarding copyright ownership.
//
// This program and the accompanying materials are made available under the
// terms of the Apache Software License 2.0 which is available at
// https://www.apache.org/licenses/LICENSE-2.0, or the MIT license
// which is available at https://opensource.org/licenses/MIT.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Defines the [`Transport`] type used to select the IPC backend for a
//! [`Service`](crate::service::Service).
//!
//! The default transport is [`Transport::SharedMemory`]. To enable
//! [`Transport::DmaBuf`] compile with `--features dma-buf` on Linux.

use iceoryx2_bb_derive_macros::ZeroCopySend;
use iceoryx2_bb_elementary_traits::zero_copy_send::ZeroCopySend;
use serde::{de::Visitor, Deserialize, Serialize};

/// Selects the inter-process memory transport used by a
/// [`Service`](crate::service::Service).
///
/// Both publisher and subscriber of the same service must use the same transport.
/// If they differ, service creation / opening will fail with
/// [`PublishSubscribeOpenError::IncompatibleTransport`](crate::service::builder::publish_subscribe::PublishSubscribeOpenError::IncompatibleTransport).
///
/// # Example
///
/// ```
/// use iceoryx2::prelude::*;
/// use iceoryx2::config::Transport;
///
/// # fn main() -> Result<(), Box<dyn core::error::Error>> {
/// let node = NodeBuilder::new().create::<ipc::Service>()?;
/// let service = node
///     .service_builder(&"My/Transport/Service".try_into()?)
///     .publish_subscribe::<u64>()
///     .transport(Transport::SharedMemory)
///     .open_or_create()?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq, ZeroCopySend)]
#[repr(C)]
pub enum Transport {
    /// POSIX shared memory (default). Available on all platforms.
    SharedMemory = 0,
    /// Linux `dma-buf` file-descriptor backed memory.
    ///
    /// Requires the `dma-buf` Cargo feature **and** Linux.  On other platforms
    /// or without the feature, attempting to create a service with this
    /// transport returns an error at runtime.
    DmaBuf = 1,
}

impl Default for Transport {
    fn default() -> Self {
        Self::SharedMemory
    }
}

impl Transport {
    /// Returns `true` if `transport` is available on this platform and with the currently
    /// compiled feature flags.
    ///
    /// - [`Transport::SharedMemory`] is always supported.
    /// - [`Transport::DmaBuf`] requires Linux **and** the `dma-buf` Cargo feature.
    pub fn is_supported(transport: Transport) -> bool {
        match transport {
            Transport::SharedMemory => true,
            Transport::DmaBuf => {
                #[cfg(all(target_os = "linux", feature = "dma-buf"))]
                {
                    true
                }
                #[cfg(not(all(target_os = "linux", feature = "dma-buf")))]
                {
                    false
                }
            }
        }
    }
}

impl Serialize for Transport {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&alloc::format!("{self:?}"))
    }
}

struct TransportVisitor;

impl Visitor<'_> for TransportVisitor {
    type Value = Transport;

    fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
        formatter.write_str("a string containing either 'SharedMemory' or 'DmaBuf'")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        match v {
            "SharedMemory" => Ok(Transport::SharedMemory),
            "DmaBuf" => Ok(Transport::DmaBuf),
            v => Err(E::custom(alloc::format!(
                "Invalid Transport provided: \"{v:?}\"."
            ))),
        }
    }
}

impl<'de> Deserialize<'de> for Transport {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(TransportVisitor)
    }
}
