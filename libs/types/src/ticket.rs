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

#[derive(Encode, Decode, Debug, Clone, PartialEq)]
pub struct TicketInfo {
    owner: AccountId,
    name: EventName,
    attendance_policy: crate::AttendancePolicy,
    attendances: Vec<Timestamp>,
    restrictions: TicketRestrictions,
}


#[derive(Default, Encode, Decode, TypeInfo, Debug, Clone, PartialEq, Eq)]
pub struct Attendances {
    pub count: u32,
    pub last: Timestamp,
}

#[derive(Default, Encode, Decode, TypeInfo, Debug, Clone, PartialEq, Eq)]
pub struct TicketRestrictions {
    pub cannot_resale: bool,
    pub cannot_transfer: bool,
}

#[allow(clippy::cast_possible_truncation)]
#[repr(u8)]
#[derive(Default, Encode, Decode, TypeInfo, Debug, Clone, PartialEq, Eq)]
pub enum AttendancePolicy<Timestamp> {
    #[default]
    Single,
    Multiple {
        max: u32,
        maybe_until: Option<Timestamp>,
    },
    Unlimited {
        maybe_until: Option<Timestamp>,
    },
}
