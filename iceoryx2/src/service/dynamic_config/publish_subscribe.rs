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
//! println!("number of active publishers:      {:?}", pubsub.dynamic_config().number_of_publishers());
//! println!("number of active subscribers:     {:?}", pubsub.dynamic_config().number_of_subscribers());
//! # Ok(())
//! # }
//! ```
use iceoryx2_bb_elementary_traits::relocatable_container::RelocatableContainer;
use iceoryx2_bb_lock_free::mpmc::{container::*, unique_index_set::ReleaseMode};
use iceoryx2_bb_log::fatal_panic;
use iceoryx2_bb_memory::bump_allocator::BumpAllocator;

use crate::{
    node::NodeId,
    port::{
        details::data_segment::DataSegmentType,
        port_identifiers::{UniquePortId, UniquePublisherId, UniqueSubscriberId},
    },
};
use iceoryx2_bb_posix::permission::Permission;

use super::PortCleanupAction;

fn mode_to_permission(mode: u16) -> Permission {
    let mut p = Permission::none();
    if mode & 0o400 != 0 { p |= Permission::OWNER_READ; }
    if mode & 0o200 != 0 { p |= Permission::OWNER_WRITE; }
    if mode & 0o100 != 0 { p |= Permission::OWNER_EXEC; }
    if mode & 0o040 != 0 { p |= Permission::GROUP_READ; }
    if mode & 0o020 != 0 { p |= Permission::GROUP_WRITE; }
    if mode & 0o010 != 0 { p |= Permission::GROUP_EXEC; }
    if mode & 0o004 != 0 { p |= Permission::OTHERS_READ; }
    if mode & 0o002 != 0 { p |= Permission::OTHERS_WRITE; }
    if mode & 0o001 != 0 { p |= Permission::OTHERS_EXEC; }
    p
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct DynamicConfigSettings {
    pub number_of_subscribers: usize,
    pub number_of_publishers: usize,
}

/// Contains the communication settings of the connected
/// [`Publisher`](crate::port::publisher::Publisher).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PublisherDetails {
    /// The [`UniquePublisherId`] of the [`Publisher`](crate::port::publisher::Publisher).
    pub publisher_id: UniquePublisherId,
    /// The [`NodeId`] of the [`Node`](crate::node::Node) under which the
    /// [`Publisher`](crate::port::publisher::Publisher) was created.
    pub node_id: NodeId,
    /// The total number of samples contained in the
    /// [`Publisher`](crate::port::publisher::Publisher)s data segment.
    pub number_of_samples: usize,
    /// The current maximum length of a slice.
    pub max_slice_len: usize,
    /// The type of data segment the [`Publisher`](crate::port::publisher::Publisher)
    /// has.
    pub data_segment_type: DataSegmentType,
    /// If the [`Publisher`](crate::port::publisher::Publisher) has the
    /// [`DataSegmentType::Dynamic`] it defines how many segment the
    /// [`Publisher`](crate::port::publisher::Publisher) can have at most.
    pub max_number_of_segments: u8,
    /// The user ID of the owner of the [`Publisher`](crate::port::publisher::Publisher).
    pub owner_uid: u32,
    /// The group ID of the [`Publisher`](crate::port::publisher::Publisher).
    pub group_gid: u32,
    /// The POSIX permission mode bits of the [`Publisher`](crate::port::publisher::Publisher).
    pub mode: u16,
}

impl PublisherDetails {
    /// Returns the owner user ID of the [`Publisher`](crate::port::publisher::Publisher).
    pub fn owner_uid(&self) -> u32 {
        self.owner_uid
    }

    /// Returns the group ID of the [`Publisher`](crate::port::publisher::Publisher).
    pub fn group_gid(&self) -> u32 {
        self.group_gid
    }

    /// Returns the POSIX permission mode of the [`Publisher`](crate::port::publisher::Publisher).
    pub fn mode(&self) -> u16 {
        self.mode
    }

    /// Returns the POSIX permission as a [`Permission`] object.
    pub fn permission(&self) -> Permission {
        mode_to_permission(self.mode)
    }

    /// Sets the owner user ID of the [`Publisher`](crate::port::publisher::Publisher).
    pub fn set_owner_uid(&mut self, uid: u32) {
        self.owner_uid = uid;
    }

    /// Sets the group ID of the [`Publisher`](crate::port::publisher::Publisher).
    pub fn set_group_gid(&mut self, gid: u32) {
        self.group_gid = gid;
    }

    /// Sets the POSIX permission mode of the [`Publisher`](crate::port::publisher::Publisher).
    pub fn set_mode(&mut self, mode: u16) {
        self.mode = mode;
    }

    /// Sets the POSIX permission of the [`Publisher`](crate::port::publisher::Publisher).
    pub fn set_permission(&mut self, permission: Permission) {
        self.mode = permission.bits() as u16;
    }
}

/// Contains the communication settings of the connected
/// [`Subscriber`](crate::port::subscriber::Subscriber).
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct SubscriberDetails {
    /// The [`UniqueSubscriberId`] of the [`Subscriber`](crate::port::subscriber::Subscriber).
    pub subscriber_id: UniqueSubscriberId,
    /// The [`NodeId`] of the [`Node`](crate::node::Node) under which the
    /// [`Subscriber`](crate::port::subscriber::Subscriber) was created.
    pub node_id: NodeId,
    /// The size of the receive buffer that stores [`Sample`](crate::sample::Sample).
    pub buffer_size: usize,
    /// The user ID of the owner of the [`Subscriber`](crate::port::subscriber::Subscriber).
    pub owner_uid: u32,
    /// The group ID of the [`Subscriber`](crate::port::subscriber::Subscriber).
    pub group_gid: u32,
    /// The POSIX permission mode bits of the [`Subscriber`](crate::port::subscriber::Subscriber).
    pub mode: u16,
}

impl SubscriberDetails {
    /// Returns the owner user ID of the [`Subscriber`](crate::port::subscriber::Subscriber).
    pub fn owner_uid(&self) -> u32 {
        self.owner_uid
    }

    /// Returns the group ID of the [`Subscriber`](crate::port::subscriber::Subscriber).
    pub fn group_gid(&self) -> u32 {
        self.group_gid
    }

    /// Returns the POSIX permission mode of the [`Subscriber`](crate::port::subscriber::Subscriber).
    pub fn mode(&self) -> u16 {
        self.mode
    }

    /// Returns the POSIX permission as a [`Permission`] object.
    pub fn permission(&self) -> Permission {
        mode_to_permission(self.mode)
    }

    /// Sets the owner user ID of the [`Subscriber`](crate::port::subscriber::Subscriber).
    pub fn set_owner_uid(&mut self, uid: u32) {
        self.owner_uid = uid;
    }

    /// Sets the group ID of the [`Subscriber`](crate::port::subscriber::Subscriber).
    pub fn set_group_gid(&mut self, gid: u32) {
        self.group_gid = gid;
    }

    /// Sets the POSIX permission mode of the [`Subscriber`](crate::port::subscriber::Subscriber).
    pub fn set_mode(&mut self, mode: u16) {
        self.mode = mode;
    }

    /// Sets the POSIX permission of the [`Subscriber`](crate::port::subscriber::Subscriber).
    pub fn set_permission(&mut self, permission: Permission) {
        self.mode = permission.bits() as u16;
    }
}

/// The dynamic configuration of an
/// [`crate::service::messaging_pattern::MessagingPattern::PublishSubscribe`]
/// based service. Contains dynamic parameters like the connected endpoints etc..
#[repr(C)]
#[derive(Debug)]
pub struct DynamicConfig {
    pub(crate) subscribers: Container<SubscriberDetails>,
    pub(crate) publishers: Container<PublisherDetails>,
}

impl DynamicConfig {
    pub(crate) fn new(config: &DynamicConfigSettings) -> Self {
        Self {
            subscribers: unsafe { Container::new_uninit(config.number_of_subscribers) },
            publishers: unsafe { Container::new_uninit(config.number_of_publishers) },
        }
    }

    pub(crate) unsafe fn init(&mut self, allocator: &BumpAllocator) {
        fatal_panic!(from self,
            when self.subscribers.init(allocator),
            "This should never happen! Unable to initialize subscriber port id container.");
        fatal_panic!(from self,
            when self.publishers.init(allocator),
            "This should never happen! Unable to initialize publisher port id container.");
    }

    pub(crate) fn memory_size(config: &DynamicConfigSettings) -> usize {
        Container::<SubscriberDetails>::memory_size(config.number_of_subscribers)
            + Container::<PublisherDetails>::memory_size(config.number_of_publishers)
    }

    pub(crate) unsafe fn remove_dead_node_id<
        PortCleanup: FnMut(UniquePortId) -> PortCleanupAction,
    >(
        &self,
        node_id: &NodeId,
        mut port_cleanup_callback: PortCleanup,
    ) {
        self.publishers
            .get_state()
            .for_each(|handle: ContainerHandle, registered_publisher| {
                if registered_publisher.node_id == *node_id
                    && port_cleanup_callback(UniquePortId::Publisher(
                        registered_publisher.publisher_id,
                    )) == PortCleanupAction::RemovePort
                {
                    self.release_publisher_handle(handle);
                }
                CallbackProgression::Continue
            });

        self.subscribers
            .get_state()
            .for_each(|handle: ContainerHandle, registered_subscriber| {
                if registered_subscriber.node_id == *node_id
                    && port_cleanup_callback(UniquePortId::Subscriber(
                        registered_subscriber.subscriber_id,
                    )) == PortCleanupAction::RemovePort
                {
                    self.release_subscriber_handle(handle);
                }
                CallbackProgression::Continue
            });
    }

    /// Returns how many [`crate::port::publisher::Publisher`] ports are currently connected.
    pub fn number_of_publishers(&self) -> usize {
        self.publishers.len()
    }

    /// Returns how many [`crate::port::subscriber::Subscriber`] ports are currently connected.
    pub fn number_of_subscribers(&self) -> usize {
        self.subscribers.len()
    }

    /// Iterates over all [`Subscriber`](crate::port::subscriber::Subscriber)s and calls the
    /// callback with the corresponding [`SubscriberDetails`].
    /// The callback shall return [`CallbackProgression::Continue`] when the iteration shall
    /// continue otherwise [`CallbackProgression::Stop`].
    pub fn list_subscribers<F: FnMut(&SubscriberDetails) -> CallbackProgression>(
        &self,
        mut callback: F,
    ) {
        let state = unsafe { self.subscribers.get_state() };

        state.for_each(|_, details| callback(details));
    }

    /// Iterates over all [`Publisher`](crate::port::publisher::Publisher)s and calls the
    /// callback with the corresponding [`PublisherDetails`].
    /// The callback shall return [`CallbackProgression::Continue`] when the iteration shall
    /// continue otherwise [`CallbackProgression::Stop`].
    pub fn list_publishers<F: FnMut(&PublisherDetails) -> CallbackProgression>(
        &self,
        mut callback: F,
    ) {
        let state = unsafe { self.publishers.get_state() };

        state.for_each(|_, details| callback(details));
    }

    pub(crate) fn add_subscriber_id(&self, details: SubscriberDetails) -> Option<ContainerHandle> {
        unsafe { self.subscribers.add(details).ok() }
    }

    pub(crate) fn release_subscriber_handle(&self, handle: ContainerHandle) {
        unsafe { self.subscribers.remove(handle, ReleaseMode::Default) };
    }

    pub(crate) fn add_publisher_id(&self, details: PublisherDetails) -> Option<ContainerHandle> {
        unsafe { self.publishers.add(details).ok() }
    }

    pub(crate) fn release_publisher_handle(&self, handle: ContainerHandle) {
        unsafe { self.publishers.remove(handle, ReleaseMode::Default) };
    }
}
