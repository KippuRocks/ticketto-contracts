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

pub mod attributes {
    pub const DEPOSIT_BALANCE: [u8; 30] = *b"TICKETTO_EVENT_DEPOSIT_BALANCE";
    pub const ORGANISER: [u8; 24] = *b"TICKETTO_EVENT_ORGANIZER";
    pub const CAPACITY: [u8; 23] = *b"TICKETTO_EVENT_CAPACITY";
    pub const NAME: [u8; 19] = *b"TICKETTO_EVENT_NAME";
    pub const TICKET_CLASS: [u8; 27] = *b"TICKETTO_EVENT_TICKET_CLASS";
    pub const TICKET_PRICE: [u8; 27] = *b"TICKETTO_EVENT_TICKET_PRICE";
    pub const TICKET_RESTRICTIONS: [u8; 34] = *b"TICKETTO_EVENT_TICKET_RESTRICTIONS";
    pub const NEXT_TICKET_ID: [u8; 29] = *b"TICKETTO_EVENT_NEXT_TICKET_ID";
}
