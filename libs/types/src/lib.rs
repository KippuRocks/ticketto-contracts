// Copyright 2025 The Kippu Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use codec::{Decode, Encode};
use scale_info::TypeInfo;

use ink::primitives::AccountId;
use ink::{env::Environment, prelude::vec::Vec};
use kreivo_apis::KreivoApiEnvironment;
pub use virto_common::{
    listings::{InventoryId as EventId, ItemId as TicketId},
    FungibleAssetLocation,
};

mod attributes;
mod error;
mod event;
mod ticket;

pub use attributes::*;
pub use error::*;
pub use event::*;
pub use ticket::*;

pub type EventName = Vec<u8>;
pub type EventCapacity = TicketId;
pub type Balance = <KreivoApiEnvironment as Environment>::Balance;
pub type Timestamp = <KreivoApiEnvironment as Environment>::Timestamp;
pub type EventDates = Vec<(Timestamp, Timestamp)>;
pub type ItemPrice = fc_traits_listings::item::ItemPrice<FungibleAssetLocation, Balance>;
pub type AttendancePolicy = ticket::AttendancePolicy<Timestamp>;

pub const DEFAULT_CLASS: [u8; 7] = *b"default";