// Copyright 2026 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Renderer-independent image comparison and snapshot support.
//!
//! Callers provide their rendered PNG, reference name and test run name. Native tests load
//! references from the caller's snapshot directory and write failure artifacts to its diff
//! directory. Browser tests compare against embedded reference PNGs and display failures in
//! the document. This crate does not depend on any Vello renderer.

pub mod diff;
mod snapshot;

pub use snapshot::Snapshot;
