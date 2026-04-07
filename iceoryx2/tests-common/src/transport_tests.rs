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

use iceoryx2::transport::Transport;
use iceoryx2_bb_testing::assert_that;
use iceoryx2_bb_testing_macros::test;

// ----- default transport is SharedMemory -----

#[test]
fn default_transport_is_shared_memory() {
    assert_that!(Transport::default(), eq Transport::SharedMemory);
}

#[test]
fn shared_memory_transport_is_always_supported() {
    assert_that!(Transport::is_supported(Transport::SharedMemory), eq true);
}

// ----- shared-memory transport works end-to-end (requires std) -----

#[test]
#[cfg(feature = "std")]
fn shared_memory_publish_subscribe_roundtrip() {
    use iceoryx2::testing::{generate_isolated_config, generate_service_name};
    use iceoryx2::{node::NodeBuilder, prelude::*};

    let service_name = generate_service_name();
    let config = generate_isolated_config();

    let node = NodeBuilder::new()
        .config(&config)
        .create::<ipc::Service>()
        .unwrap();

    let service = node
        .service_builder(&service_name)
        .publish_subscribe::<u64>()
        .transport(Transport::SharedMemory)
        .open_or_create()
        .unwrap();

    let publisher = service.publisher_builder().create().unwrap();
    let subscriber = service.subscriber_builder().create().unwrap();

    publisher.send_copy(42_u64).unwrap();

    let sample = subscriber.receive().unwrap().unwrap();
    assert_that!(*sample, eq 42_u64);
}

// ----- transport mismatch is rejected -----

#[test]
#[cfg(feature = "std")]
fn transport_mismatch_fails_with_incompatible_transport() {
    use iceoryx2::service::builder::publish_subscribe::PublishSubscribeOpenError;
    use iceoryx2::testing::{generate_isolated_config, generate_service_name};
    use iceoryx2::{node::NodeBuilder, prelude::*};

    let service_name = generate_service_name();
    let config = generate_isolated_config();

    let node = NodeBuilder::new()
        .config(&config)
        .create::<ipc::Service>()
        .unwrap();

    // Create with SharedMemory transport (default).
    let _service_creator = node
        .service_builder(&service_name)
        .publish_subscribe::<u64>()
        .transport(Transport::SharedMemory)
        .open_or_create()
        .unwrap();

    // Attempt to open with DmaBuf transport – must fail with IncompatibleTransport.
    let result = node
        .service_builder(&service_name)
        .publish_subscribe::<u64>()
        .transport(Transport::DmaBuf)
        .open();

    assert_that!(result, is_err);
    assert_that!(
        result.unwrap_err(),
        eq PublishSubscribeOpenError::IncompatibleTransport
    );
}

#[test]
#[cfg(feature = "std")]
fn transport_mismatch_reverse_fails_with_incompatible_transport() {
    use iceoryx2::service::builder::publish_subscribe::{
        PublishSubscribeCreateError, PublishSubscribeOpenError,
    };
    use iceoryx2::testing::{generate_isolated_config, generate_service_name};
    use iceoryx2::{node::NodeBuilder, prelude::*};

    let service_name = generate_service_name();
    let config = generate_isolated_config();

    let node = NodeBuilder::new()
        .config(&config)
        .create::<ipc::Service>()
        .unwrap();

    // First: select DmaBuf transport (not yet created, so this depends on platform support).
    // If DmaBuf is unsupported we test the create-error path instead.
    let create_result = node
        .service_builder(&service_name)
        .publish_subscribe::<u64>()
        .transport(Transport::DmaBuf)
        .create();

    if !Transport::is_supported(Transport::DmaBuf) {
        // On non-Linux or without the feature: create must fail with TransportNotSupported.
        assert_that!(create_result, is_err);
        assert_that!(
            create_result.unwrap_err(),
            eq PublishSubscribeCreateError::TransportNotSupported
        );
        return;
    }

    // On supported platforms: create succeeds.
    let _service_dma = create_result.unwrap();

    // Now open with SharedMemory transport – must fail.
    let open_result = node
        .service_builder(&service_name)
        .publish_subscribe::<u64>()
        .transport(Transport::SharedMemory)
        .open();

    assert_that!(open_result, is_err);
    assert_that!(
        open_result.unwrap_err(),
        eq PublishSubscribeOpenError::IncompatibleTransport
    );
}

// ----- dma_buf transport (Linux + feature-gated) -----

#[test]
#[cfg(all(target_os = "linux", feature = "dma-buf"))]
fn dma_buf_transport_is_supported_on_linux_with_feature() {
    assert_that!(Transport::is_supported(Transport::DmaBuf), eq true);
}

#[test]
#[cfg(all(target_os = "linux", feature = "dma-buf", feature = "std"))]
fn dma_buf_publish_subscribe_roundtrip() {
    use iceoryx2::testing::{generate_isolated_config, generate_service_name};
    use iceoryx2::{node::NodeBuilder, prelude::*};

    let service_name = generate_service_name();
    let config = generate_isolated_config();

    let node = NodeBuilder::new()
        .config(&config)
        .create::<ipc::Service>()
        .unwrap();

    let service = node
        .service_builder(&service_name)
        .publish_subscribe::<u64>()
        .transport(Transport::DmaBuf)
        .open_or_create()
        .unwrap();

    let publisher = service.publisher_builder().create().unwrap();
    let subscriber = service.subscriber_builder().create().unwrap();

    publisher.send_copy(99_u64).unwrap();

    let sample = subscriber.receive().unwrap().unwrap();
    assert_that!(*sample, eq 99_u64);
}

#[test]
#[cfg(all(target_os = "linux", feature = "dma-buf", feature = "std"))]
fn dma_buf_static_config_records_transport() {
    use iceoryx2::testing::{generate_isolated_config, generate_service_name};
    use iceoryx2::{node::NodeBuilder, prelude::*};

    let service_name = generate_service_name();
    let config = generate_isolated_config();

    let node = NodeBuilder::new()
        .config(&config)
        .create::<ipc::Service>()
        .unwrap();

    let service = node
        .service_builder(&service_name)
        .publish_subscribe::<u64>()
        .transport(Transport::DmaBuf)
        .open_or_create()
        .unwrap();

    let static_cfg = service.static_config();
    if let iceoryx2::service::messaging_pattern::MessagingPattern::PublishSubscribe(ref ps) =
        static_cfg.messaging_pattern()
    {
        assert_that!(ps.transport(), eq Transport::DmaBuf);
    } else {
        panic!("expected PublishSubscribe messaging pattern");
    }
}

// ----- DmaBuf unsupported on non-Linux or without feature -----

#[test]
#[cfg(not(all(target_os = "linux", feature = "dma-buf")))]
fn dma_buf_transport_is_not_supported_without_feature() {
    assert_that!(Transport::is_supported(Transport::DmaBuf), eq false);
}

#[test]
#[cfg(all(not(all(target_os = "linux", feature = "dma-buf")), feature = "std"))]
fn dma_buf_create_fails_with_transport_not_supported() {
    use iceoryx2::service::builder::publish_subscribe::PublishSubscribeCreateError;
    use iceoryx2::testing::{generate_isolated_config, generate_service_name};
    use iceoryx2::{node::NodeBuilder, prelude::*};

    let service_name = generate_service_name();
    let config = generate_isolated_config();

    let node = NodeBuilder::new()
        .config(&config)
        .create::<ipc::Service>()
        .unwrap();

    let result = node
        .service_builder(&service_name)
        .publish_subscribe::<u64>()
        .transport(Transport::DmaBuf)
        .create();

    assert_that!(result, is_err);
    assert_that!(
        result.unwrap_err(),
        eq PublishSubscribeCreateError::TransportNotSupported
    );
}
