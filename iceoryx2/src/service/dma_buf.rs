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

//! Linux `dma-buf` backed inter-process communication service.
//!
//! This service type is identical to [`ipc::Service`](crate::service::ipc::Service) in terms
//! of the underlying IPC machinery, but it tags all services with the
//! [`Transport::DmaBuf`](crate::transport::Transport::DmaBuf) transport identifier so that
//! only endpoints that explicitly select the same transport can communicate with each other.
//!
//! **Platform and feature requirements:** this module is only available on **Linux** when the
//! `dma-buf` Cargo feature is enabled.  On other platforms or without the feature the service
//! type still compiles but any attempt to *create* a service will return
//! [`PublishSubscribeCreateError::TransportNotSupported`](crate::service::builder::publish_subscribe::PublishSubscribeCreateError::TransportNotSupported).
//!
//! # Example
//!
//! ```
//! #[cfg(all(target_os = "linux", feature = "dma-buf"))]
//! # {
//! use iceoryx2::prelude::*;
//! use iceoryx2::config::Transport;
//!
//! # fn main() -> Result<(), Box<dyn core::error::Error>> {
//! let node = NodeBuilder::new().create::<ipc::Service>()?;
//!
//! // Select the dma-buf transport explicitly – both ends must match.
//! let service = node
//!     .service_builder(&"My/DmaBuf/Service".try_into()?)
//!     .publish_subscribe::<u64>()
//!     .transport(Transport::DmaBuf)
//!     .open_or_create()?;
//!
//! let publisher = service.publisher_builder().create()?;
//! let subscriber = service.subscriber_builder().create()?;
//! # Ok(())
//! # }
//! # }
//! ```
//!
//! See [`Service`](crate::service) for more detailed examples.

// Re-export the ipc service as the base type.  The dma-buf distinction is enforced purely
// through the transport field in the static service configuration, not through different
// underlying memory primitives.
pub use crate::service::ipc::Service;
