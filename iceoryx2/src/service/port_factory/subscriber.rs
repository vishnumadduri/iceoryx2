// Copyright (c) 2023 Contributors to the Eclipse Foundation
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

//! # Example
//!
//! ```
//! use iceoryx2::prelude::*;
//!
//! # fn main() -> Result<(), Box<dyn core::error::Error>> {
//! let node = NodeBuilder::new().create::<ipc::Service>()?;
//! let pubsub = node.service_builder(&"My/Funk/ServiceName".try_into()?)
//!     .publish_subscribe::<u64>()
//!     .open_or_create()?;
//!
//! let subscriber = pubsub.subscriber_builder()
//!                     .create()?;
//!
//! # Ok(())
//! # }
//! ```

use core::fmt::Debug;

use iceoryx2_bb_elementary_traits::zero_copy_send::ZeroCopySend;
use iceoryx2_bb_log::fail;

use crate::{
    port::{
        subscriber::{Subscriber, SubscriberCreateError},
        DegradationAction, DegradationCallback,
    },
    service,
};

use super::publish_subscribe::PortFactory;

#[derive(Debug)]
pub(crate) struct SubscriberConfig {
    pub(crate) buffer_size: Option<usize>,
    pub(crate) degradation_callback: Option<DegradationCallback<'static>>,
    pub(crate) owner_uid: Option<u32>,
    pub(crate) group_gid: Option<u32>,
    pub(crate) mode: Option<u16>,
}

/// Factory to create a new [`Subscriber`] port/endpoint for
/// [`MessagingPattern::PublishSubscribe`](crate::service::messaging_pattern::MessagingPattern::PublishSubscribe) based
/// communication.
#[derive(Debug)]
pub struct PortFactorySubscriber<
    'factory,
    Service: service::Service,
    PayloadType: Debug + ZeroCopySend + ?Sized,
    UserHeader: Debug + ZeroCopySend,
> {
    config: SubscriberConfig,
    pub(crate) factory: &'factory PortFactory<Service, PayloadType, UserHeader>,
}

unsafe impl<
        Service: service::Service,
        Payload: Debug + ZeroCopySend + ?Sized,
        UserHeader: Debug + ZeroCopySend,
    > Send for PortFactorySubscriber<'_, Service, Payload, UserHeader>
{
}

impl<
        'factory,
        Service: service::Service,
        PayloadType: Debug + ZeroCopySend + ?Sized,
        UserHeader: Debug + ZeroCopySend,
    > PortFactorySubscriber<'factory, Service, PayloadType, UserHeader>
{
    #[doc(hidden)]
    /// # Safety
    ///
    ///   * does not clone the degradation callback
    pub unsafe fn __internal_partial_clone(&self) -> Self {
        Self {
            config: SubscriberConfig {
                buffer_size: self.config.buffer_size,
                degradation_callback: None,
                owner_uid: self.config.owner_uid,
                group_gid: self.config.group_gid,
                mode: self.config.mode,
            },
            factory: self.factory,
        }
    }

    pub(crate) fn new(factory: &'factory PortFactory<Service, PayloadType, UserHeader>) -> Self {
        Self {
            config: SubscriberConfig {
                buffer_size: None,
                degradation_callback: None,
                owner_uid: None,
                group_gid: None,
                mode: None,
            },
            factory,
        }
    }

    /// Defines the buffer size of the [`Subscriber`]. Smallest possible value is `1`.
    pub fn buffer_size(mut self, value: usize) -> Self {
        self.config.buffer_size = Some(value.max(1));
        self
    }

    /// Sets the [`DegradationCallback`] of the [`Subscriber`]. Whenever a connection to a
    /// [`crate::port::subscriber::Subscriber`] is corrupted or it seems to be dead, this callback
    /// is called and depending on the returned [`DegradationAction`] measures will be taken.
    pub fn set_degradation_callback<
        F: Fn(&service::static_config::StaticConfig, u128, u128) -> DegradationAction + 'static,
    >(
        mut self,
        callback: Option<F>,
    ) -> Self {
        match callback {
            Some(c) => self.config.degradation_callback = Some(DegradationCallback::new(c)),
            None => self.config.degradation_callback = None,
        }

        self
    }

    /// Sets the owner user ID for the [`Subscriber`]. If not set, defaults to the current process UID.
    pub fn owner_uid(mut self, uid: u32) -> Self {
        self.config.owner_uid = Some(uid);
        self
    }

    /// Sets the group ID for the [`Subscriber`]. If not set, defaults to the current process GID.
    pub fn group_gid(mut self, gid: u32) -> Self {
        self.config.group_gid = Some(gid);
        self
    }

    /// Sets the POSIX permission mode for the [`Subscriber`]. If not set, defaults to 0o640 (rw-r-----).
    pub fn mode(mut self, mode: u16) -> Self {
        self.config.mode = Some(mode);
        self
    }

    /// Sets the POSIX permission for the [`Subscriber`]. If not set, defaults to 0o640 (rw-r-----).
    pub fn permission(mut self, permission: iceoryx2_bb_posix::permission::Permission) -> Self {
        self.config.mode = Some(permission.bits() as u16);
        self
    }

    /// Creates a new [`Subscriber`] or returns a [`SubscriberCreateError`] on failure.
    pub fn create(
        self,
    ) -> Result<Subscriber<Service, PayloadType, UserHeader>, SubscriberCreateError> {
        let origin = format!("{self:?}");
        Ok(
            fail!(from origin, when Subscriber::new(&self.factory.service, self.factory.service.__internal_state().static_config.publish_subscribe(), self.config),
                "Failed to create new Subscriber port."),
        )
    }
}
