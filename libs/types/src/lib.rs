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

mod event;
mod ticket;

use codec::{Decode, Encode};
use scale_info::TypeInfo;

use ink::primitives::AccountId;
use ink::{env::Environment, prelude::vec::Vec};
use kreivo_apis::{apis::KreivoApisError, KreivoApiEnvironment};
pub use virto_common::{
    listings::{InventoryId as EventId, ItemId as TicketId},
    FungibleAssetLocation,
};

pub use event::{attributes as event_attributes, EventState};
pub use ticket::{attributes as ticket_attributes, TicketRestrictions};

pub type EventName = Vec<u8>;
pub type EventCapacity = TicketId;
pub type Balance = <KreivoApiEnvironment as Environment>::Balance;
pub type Timestamp = <KreivoApiEnvironment as Environment>::Timestamp;
pub type EventDates = Vec<(Timestamp, Timestamp)>;
pub type ItemPrice = fc_traits_listings::item::ItemPrice<FungibleAssetLocation, Balance>;
pub type AttendancePolicy = ticket::AttendancePolicy<Timestamp>;

pub type EventInfo = (
    AccountId,
    EventName,
    EventCapacity,
    AttendancePolicy,
    Option<ItemPrice>,
    Option<TicketRestrictions>,
);

#[derive(Encode, Decode, TypeInfo)]
#[allow(clippy::cast_possible_truncation)]
#[repr(u8)]
pub enum Error {
    /// It was not possible to complete the operation due to low
    /// balance.
    LowBalance,
    /// The operation it's not possible due to overflow.
    Overflow,
    /// The given `EventId` is not found.
    EventNotFound,
    /// The caller does not have permissions to mutate the state of
    /// an event or a ticket.
    NoPermission,
    /// This action is not possible to be done until dates are set.
    DatesNotSet,
    /// This action is not possible to be done at the current state of the event.
    InvalidState,
    /// The maximum capacity of the event was reached. It is not
    /// possible to issue a new ticket.
    MaxCapacity,
    /// An error occurred on the Kreivo APIs side.
    KreivoApiError(KreivoApisError),
    /// A generic error that cannot be categorized.
    Other([u8; 32]),
}

impl From<KreivoApisError> for Error {
    fn from(value: KreivoApisError) -> Self {
        Self::KreivoApiError(value)
    }
}
