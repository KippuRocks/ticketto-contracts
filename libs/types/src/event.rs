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

use super::*;

#[derive(Encode, Decode, TypeInfo, Debug, Clone, PartialEq)]
pub struct EventInfo {
    pub organiser: [u8; 32],
    pub name: EventName,
    pub state: EventState,
    pub capacity: EventCapacity,
    pub class: TicketClass,
    pub dates: Option<EventDates>,
}

#[allow(clippy::cast_possible_truncation)]
#[repr(u8)]
#[derive(Default, Encode, Decode, TypeInfo, Debug, Clone, PartialEq, Eq)]
pub enum EventState {
    /// The initial status. Tickets cannot be issued.
    #[default]
    Created,
    /// The event is now ready to be sold. Tickets can be issued, but not used. Capacity cannot be
    /// modified, but ticket's details can still be set.
    Sales,
    /// The first date in the list of dates has begun. Tickets can be issued, and used. Dates cannot
    /// longer be modified.
    Ongoing,
    /// The last date in the list of dates has finished. Tickets can no longer be issued.
    Finished,
}

#[derive(Encode, Decode, TypeInfo, Debug, Clone, PartialEq)]
pub struct TicketClass {
    pub attendance_policy: AttendancePolicy,
    pub price: ItemPrice,
    pub maybe_restrictions: Option<TicketRestrictions>,
}
